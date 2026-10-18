//! Tree-walking evaluation of [`Stmt`]s.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use indexmap::IndexMap;

use crate::ast::*;
use crate::value::*;

/// What the engine provides to scripts.
pub trait Host {
    /// A global function the interpreter doesn't know (`Time`, `EntFire`...).
    /// `None` when the host doesn't know it either.
    fn call(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>>;
    /// A method on an engine object (an entity handle).
    fn method(&mut self, handle: u64, name: &str, args: &[Value]) -> Result<Value, String>;
    /// Source of a script by its VScript name (`choreo/glados`).
    fn read_script(&mut self, name: &str) -> Option<String>;
    fn print(&mut self, text: &str);
}

/// A thrown value (or a runtime error, as a string).
#[derive(Debug, Clone)]
pub struct Exc(pub Value);

type Res<T> = Result<T, Exc>;

fn err<T>(msg: impl Into<String>) -> Res<T> {
    Err(Exc(Value::Str(msg.into().into())))
}

enum Flow {
    Normal,
    Break,
    Continue,
    Return(Value),
}

#[derive(Clone)]
struct Frame {
    scope: Rc<Scope>,
    this: Value,
}

/// Interpreter state: the root table, every entity's script scope, parsed
/// scripts.
pub struct Vm {
    pub root: TableRef,
    pub scopes: HashMap<u64, TableRef>,
    parsed: HashMap<String, Rc<Vec<Stmt>>>,
    /// Statements run in the current entry call (runaway guard).
    steps: u64,
    depth: usize,
    /// Function names being run (for error messages).
    pub trace: Vec<Rc<str>>,
}

const STEP_LIMIT: u64 = 20_000_000;
const DEPTH_LIMIT: usize = 200;

/// Functions the interpreter itself provides.
const BUILTINS: &[&str] = &[
    "print", "printl", "array", "getroottable", "assert", "type", "abs", "fabs", "sqrt", "floor", "ceil", "pow", "sin", "cos", "tan", "atan2", "rand",
    "IncludeScript", "DoIncludeScript", "Vector", "split", "strip", "lstrip", "rstrip", "format", "max", "min", "clamp", "seterrorhandler", "collectgarbage",
];

impl Vm {
    /// A VM whose root table knows the host's global functions `natives`.
    pub fn new(natives: &[&str]) -> Self {
        let root = Table::new_ref();
        {
            let mut r = root.borrow_mut();
            for n in BUILTINS.iter().chain(natives) {
                r.map.insert(Key::str(n), Value::Native((*n).into()));
            }
            r.map.insert(Key::str("_version_"), Value::str("Squirrel 2.2 (p2 vscript)"));
            r.map.insert(Key::str("RAND_MAX"), Value::Int(32767));
            r.map.insert(Key::str("PI"), Value::Float(std::f64::consts::PI));
        }
        Vm { root, scopes: HashMap::new(), parsed: HashMap::new(), steps: 0, depth: 0, trace: Vec::new() }
    }

    /// The script scope of entity `id` (made on first use; it sees the root
    /// table through its delegate).
    pub fn scope(&mut self, id: u64) -> TableRef {
        let root = self.root.clone();
        self.scopes
            .entry(id)
            .or_insert_with(|| {
                let t = Table::new_ref();
                t.borrow_mut().delegate = Some(root);
                t
            })
            .clone()
    }

    fn parse_cached(&mut self, key: &str, src: &str) -> Result<Rc<Vec<Stmt>>, String> {
        if let Some(p) = self.parsed.get(key) {
            return Ok(p.clone());
        }
        let p = Rc::new(crate::parser::parse(src)?);
        if self.parsed.len() > 4096 {
            self.parsed.clear();
        }
        self.parsed.insert(key.to_string(), p.clone());
        Ok(p)
    }

    /// Run a script file in `scope`.
    pub fn run_file(&mut self, scope: TableRef, name: &str, host: &mut dyn Host) -> Result<(), String> {
        let key = format!("file:{name}");
        let prog = match self.parsed.get(&key) {
            Some(p) => p.clone(),
            None => {
                let src = host.read_script(name).ok_or_else(|| format!("script {name} not found"))?;
                self.parse_cached(&key, &src).map_err(|e| format!("{name}: {e}"))?
            }
        };
        self.run_program(&prog, Value::Table(scope), host).map(|_| ())
    }

    /// Run source text (an input's `RunScriptCode`) in `scope`.
    pub fn run_code(&mut self, scope: TableRef, code: &str, host: &mut dyn Host) -> Result<Value, String> {
        let prog = self.parse_cached(&format!("code:{code}"), code)?;
        self.run_program(&prog, Value::Table(scope), host)
    }

    fn run_program(&mut self, prog: &[Stmt], this: Value, host: &mut dyn Host) -> Result<Value, String> {
        self.steps = 0;
        self.depth = 0;
        self.trace.clear();
        let frame = Frame { scope: Scope::new(None), this };
        let mut cx = Cx { vm: self, host };
        match cx.block(prog, &frame) {
            Ok(Flow::Return(v)) => Ok(v),
            Ok(_) => Ok(Value::Null),
            Err(Exc(v)) => Err(format!("{v} (in {})", cx.vm.trace.iter().map(|s| &**s).collect::<Vec<_>>().join(" > "))),
        }
    }

    /// Call `name` found in `scope` (or the root), with `this` = the scope.
    /// `Ok(None)` when there is no such function.
    pub fn call_in(&mut self, scope: TableRef, name: &str, args: &[Value], host: &mut dyn Host) -> Result<Option<Value>, String> {
        let Some(f) = Table::get(&scope, &Key::str(name)) else { return Ok(None) };
        self.call_value(f, Value::Table(scope), args, host).map(Some)
    }

    pub fn call_value(&mut self, f: Value, this: Value, args: &[Value], host: &mut dyn Host) -> Result<Value, String> {
        self.steps = 0;
        self.depth = 0;
        self.trace.clear();
        let mut cx = Cx { vm: self, host };
        cx.call(&f, this, args.to_vec()).map_err(|Exc(v)| format!("{v} (in {})", cx.vm.trace.iter().map(|s| &**s).collect::<Vec<_>>().join(" > ")))
    }
}

pub(crate) struct Cx<'a> {
    vm: &'a mut Vm,
    host: &'a mut dyn Host,
}

fn num_op(op: &str, a: &Value, b: &Value) -> Res<Value> {
    match (a, b) {
        (Value::Int(x), Value::Int(y)) => Ok(match op {
            "+" => Value::Int(x.wrapping_add(*y)),
            "-" => Value::Int(x.wrapping_sub(*y)),
            "*" => Value::Int(x.wrapping_mul(*y)),
            "/" => {
                if *y == 0 {
                    return err("division by zero");
                }
                Value::Int(x / y)
            }
            "%" => {
                if *y == 0 {
                    return err("modulo by zero");
                }
                Value::Int(x % y)
            }
            _ => return err(format!("bad operator {op}")),
        }),
        _ => {
            let (Some(x), Some(y)) = (a.as_f64(), b.as_f64()) else {
                return err(format!("arith op {op} on {} and {}", a.type_name(), b.type_name()));
            };
            Ok(Value::Float(match op {
                "+" => x + y,
                "-" => x - y,
                "*" => x * y,
                "/" => x / y,
                "%" => x % y,
                _ => return err(format!("bad operator {op}")),
            }))
        }
    }
}

fn compare(a: &Value, b: &Value) -> Res<std::cmp::Ordering> {
    match (a, b) {
        (Value::Str(x), Value::Str(y)) => Ok(x.cmp(y)),
        (Value::Int(x), Value::Int(y)) => Ok(x.cmp(y)),
        _ => match (a.as_f64(), b.as_f64()) {
            (Some(x), Some(y)) => Ok(x.partial_cmp(&y).unwrap_or(std::cmp::Ordering::Equal)),
            _ => err(format!("comparison between {} and {}", a.type_name(), b.type_name())),
        },
    }
}

fn vec3(v: &Value) -> Option<[f64; 3]> {
    match v {
        Value::Vector(r) => Some(*r.borrow()),
        _ => None,
    }
}

impl Cx<'_> {
    fn tick(&mut self) -> Res<()> {
        self.vm.steps += 1;
        if self.vm.steps > STEP_LIMIT {
            return err("script ran too long");
        }
        Ok(())
    }

    fn block(&mut self, stmts: &[Stmt], frame: &Frame) -> Res<Flow> {
        for s in stmts {
            match self.stmt(s, frame)? {
                Flow::Normal => {}
                other => return Ok(other),
            }
        }
        Ok(Flow::Normal)
    }

    fn scoped_block(&mut self, stmts: &[Stmt], frame: &Frame) -> Res<Flow> {
        let inner = Frame { scope: Scope::new(Some(frame.scope.clone())), this: frame.this.clone() };
        self.block(stmts, &inner)
    }

    fn stmt(&mut self, s: &Stmt, frame: &Frame) -> Res<Flow> {
        self.tick()?;
        match s {
            Stmt::Expr(e) => {
                self.eval(e, frame)?;
            }
            Stmt::Local(vars) => {
                for (n, init) in vars {
                    let v = match init {
                        Some(e) => self.eval(e, frame)?,
                        None => Value::Null,
                    };
                    frame.scope.declare(n.clone(), v);
                }
            }
            Stmt::FuncDecl(path, def) => {
                let f = self.closure(def, frame)?;
                self.assign_to(path, "<-", f, frame)?;
            }
            Stmt::ClassDecl(path, def) => {
                let c = self.class(def, frame)?;
                self.assign_to(path, "<-", c, frame)?;
            }
            Stmt::If(c, a, b) => {
                return if self.eval(c, frame)?.truthy() { self.scoped_block(a, frame) } else { self.scoped_block(b, frame) };
            }
            Stmt::While(c, body) => {
                while self.eval(c, frame)?.truthy() {
                    match self.scoped_block(body, frame)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            Stmt::DoWhile(body, c) => loop {
                match self.scoped_block(body, frame)? {
                    Flow::Break => break,
                    Flow::Return(v) => return Ok(Flow::Return(v)),
                    _ => {}
                }
                if !self.eval(c, frame)?.truthy() {
                    break;
                }
            },
            Stmt::For(init, cond, step, body) => {
                let inner = Frame { scope: Scope::new(Some(frame.scope.clone())), this: frame.this.clone() };
                for s in init {
                    self.stmt(s, &inner)?;
                }
                loop {
                    if let Some(c) = cond {
                        if !self.eval(c, &inner)?.truthy() {
                            break;
                        }
                    }
                    match self.scoped_block(body, &inner)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                    for e in step {
                        self.eval(e, &inner)?;
                    }
                }
            }
            Stmt::Foreach(k, v, e, body) => {
                let container = self.eval(e, frame)?;
                let items: Vec<(Value, Value)> = match &container {
                    Value::Table(t) => t.borrow().map.iter().map(|(k, v)| (Value::from_key(k), v.clone())).collect(),
                    Value::Array(a) => a.borrow().iter().enumerate().map(|(i, v)| (Value::Int(i as i64), v.clone())).collect(),
                    Value::Str(s) => s.bytes().enumerate().map(|(i, b)| (Value::Int(i as i64), Value::Int(b as i64))).collect(),
                    Value::Instance(i) => i.fields.borrow().iter().map(|(k, v)| (Value::from_key(k), v.clone())).collect(),
                    Value::Class(c) => c.members.borrow().iter().map(|(k, v)| (Value::from_key(k), v.clone())).collect(),
                    Value::Null => Vec::new(),
                    other => return err(format!("cannot iterate a {}", other.type_name())),
                };
                for (key, val) in items {
                    let inner = Frame { scope: Scope::new(Some(frame.scope.clone())), this: frame.this.clone() };
                    if let Some(k) = k {
                        inner.scope.declare(k.clone(), key);
                    }
                    inner.scope.declare(v.clone(), val);
                    match self.block(body, &inner)? {
                        Flow::Break => break,
                        Flow::Return(v) => return Ok(Flow::Return(v)),
                        _ => {}
                    }
                }
            }
            Stmt::Switch(e, cases) => {
                let v = self.eval(e, frame)?;
                let mut start = None;
                for (i, (label, _)) in cases.iter().enumerate() {
                    if let Some(l) = label {
                        if self.eval(l, frame)?.equals(&v) {
                            start = Some(i);
                            break;
                        }
                    }
                }
                if start.is_none() {
                    start = cases.iter().position(|(l, _)| l.is_none());
                }
                if let Some(s) = start {
                    let inner = Frame { scope: Scope::new(Some(frame.scope.clone())), this: frame.this.clone() };
                    for (_, body) in &cases[s..] {
                        match self.block(body, &inner)? {
                            Flow::Normal => {}
                            Flow::Break => break,
                            other => return Ok(other),
                        }
                    }
                }
            }
            Stmt::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e, frame)?,
                    None => Value::Null,
                };
                return Ok(Flow::Return(v));
            }
            Stmt::Break => return Ok(Flow::Break),
            Stmt::Continue => return Ok(Flow::Continue),
            Stmt::Throw(e) => {
                let v = self.eval(e, frame)?;
                return Err(Exc(v));
            }
            Stmt::Try(body, name, handler) => match self.scoped_block(body, frame) {
                Ok(f) => return Ok(f),
                Err(Exc(v)) => {
                    let inner = Frame { scope: Scope::new(Some(frame.scope.clone())), this: frame.this.clone() };
                    inner.scope.declare(name.clone(), v);
                    return self.block(handler, &inner);
                }
            },
            Stmt::Block(b) => return self.scoped_block(b, frame),
            Stmt::Const(name, e) => {
                let v = self.eval(e, frame)?;
                self.vm.root.borrow_mut().map.insert(Key::Str(name.clone()), v);
            }
        }
        Ok(Flow::Normal)
    }

    fn closure(&mut self, def: &Rc<FuncDef>, frame: &Frame) -> Res<Value> {
        let mut defaults = Vec::with_capacity(def.params.len());
        for (_, d) in &def.params {
            defaults.push(match d {
                Some(e) => Some(self.eval(e, frame)?),
                None => None,
            });
        }
        Ok(Value::Closure(Rc::new(Closure { def: def.clone(), scope: Some(frame.scope.clone()), env: None, defaults })))
    }

    fn class(&mut self, def: &Rc<ClassDef>, frame: &Frame) -> Res<Value> {
        let base = match &def.base {
            Some(b) => match self.eval(b, frame)? {
                Value::Class(c) => Some(c),
                other => return err(format!("cannot extend a {}", other.type_name())),
            },
            None => None,
        };
        let mut members = IndexMap::new();
        for (k, v, _) in &def.members {
            let k = self.eval(k, frame)?.key();
            let v = self.eval(v, frame)?;
            members.insert(k, v);
        }
        Ok(Value::Class(Rc::new(Class { def: def.clone(), base, members: RefCell::new(members) })))
    }

    fn lookup(&mut self, name: &str, frame: &Frame) -> Res<Value> {
        if let Some(v) = frame.scope.lookup(name) {
            return Ok(v);
        }
        let k = Key::str(name);
        match &frame.this {
            Value::Table(t) => {
                if let Some(v) = Table::get(t, &k) {
                    return Ok(v);
                }
            }
            Value::Instance(i) => {
                if let Some(v) = i.fields.borrow().get(&k) {
                    return Ok(v.clone());
                }
                if let Some(v) = i.class.get(&k) {
                    return Ok(v);
                }
            }
            _ => {}
        }
        if let Some(v) = Table::get(&self.vm.root, &k) {
            return Ok(v);
        }
        err(format!("the index '{name}' does not exist"))
    }

    fn get_member(&mut self, obj: &Value, key: &Value) -> Res<Value> {
        let k = key.key();
        match obj {
            Value::Table(t) => match Table::get(t, &k) {
                Some(v) => Ok(v),
                None => err(format!("the index '{key}' does not exist")),
            },
            Value::Instance(i) => {
                if let Some(v) = i.fields.borrow().get(&k) {
                    return Ok(v.clone());
                }
                i.class.get(&k).map_or_else(|| err(format!("the index '{key}' does not exist")), Ok)
            }
            Value::Class(c) => c.get(&k).map_or_else(|| err(format!("the index '{key}' does not exist")), Ok),
            Value::Array(a) => {
                let i = key.as_i64().ok_or_else(|| Exc(Value::str("array index must be an integer")))?;
                let a = a.borrow();
                let idx = if i < 0 { a.len() as i64 + i } else { i };
                a.get(idx as usize).cloned().map_or_else(|| err(format!("array index {i} out of range")), Ok)
            }
            Value::Str(s) => {
                let i = key.as_i64().ok_or_else(|| Exc(Value::str("string index must be an integer")))?;
                s.as_bytes().get(i as usize).map(|b| Value::Int(*b as i64)).map_or_else(|| err("string index out of range"), Ok)
            }
            Value::Vector(v) => {
                let idx = match key.as_str() {
                    Some("x") => 0,
                    Some("y") => 1,
                    Some("z") => 2,
                    _ => return err(format!("vector has no member {key}")),
                };
                Ok(Value::Float(v.borrow()[idx]))
            }
            other => err(format!("cannot get '{key}' of a {}", other.type_name())),
        }
    }

    fn set_member(&mut self, obj: &Value, key: Value, v: Value, newslot: bool) -> Res<()> {
        let k = key.key();
        match obj {
            Value::Table(t) => {
                let mut b = t.borrow_mut();
                if newslot || b.map.contains_key(&k) {
                    b.map.insert(k, v);
                    return Ok(());
                }
                // A slot that lives in a delegate (a scope's root): set it there.
                let mut cur = b.delegate.clone();
                drop(b);
                while let Some(d) = cur {
                    if d.borrow().map.contains_key(&k) {
                        d.borrow_mut().map.insert(k, v);
                        return Ok(());
                    }
                    cur = d.borrow().delegate.clone();
                }
                // Real Squirrel throws here; scripts never rely on that.
                t.borrow_mut().map.insert(k, v);
                Ok(())
            }
            Value::Instance(i) => {
                i.fields.borrow_mut().insert(k, v);
                Ok(())
            }
            Value::Class(c) => {
                c.members.borrow_mut().insert(k, v);
                Ok(())
            }
            Value::Array(a) => {
                let i = key.as_i64().ok_or_else(|| Exc(Value::str("array index must be an integer")))?;
                let mut a = a.borrow_mut();
                let idx = if i < 0 { a.len() as i64 + i } else { i } as usize;
                if idx >= a.len() {
                    return err(format!("array index {i} out of range"));
                }
                a[idx] = v;
                Ok(())
            }
            Value::Vector(vec) => {
                let idx = match key.as_str() {
                    Some("x") => 0,
                    Some("y") => 1,
                    Some("z") => 2,
                    _ => return err(format!("vector has no member {key}")),
                };
                vec.borrow_mut()[idx] = v.as_f64().unwrap_or(0.0);
                Ok(())
            }
            other => err(format!("cannot set '{key}' on a {}", other.type_name())),
        }
    }

    /// `target op value` where op is `=`, `<-` or a compound operator.
    fn assign_to(&mut self, target: &Expr, op: &str, value: Value, frame: &Frame) -> Res<Value> {
        let combine = |cx: &mut Self, old: Value, value: Value| -> Res<Value> {
            match op {
                "=" | "<-" => Ok(value),
                _ => cx.binop(&op[..op.len() - 1], &old, &value),
            }
        };
        match target {
            Expr::Ident(name) => {
                if op == "<-" {
                    let this = frame.this.clone();
                    self.set_member(&this, Value::Str(name.clone()), value.clone(), true)?;
                    return Ok(value);
                }
                if let Some(old) = frame.scope.lookup(name) {
                    let v = combine(self, old, value)?;
                    frame.scope.set(name, v.clone());
                    return Ok(v);
                }
                let k = Key::Str(name.clone());
                let old = self.lookup(name, frame).unwrap_or(Value::Null);
                let v = combine(self, old, value)?;
                let in_this = match &frame.this {
                    Value::Table(t) => Table::has(t, &k),
                    Value::Instance(i) => i.fields.borrow().contains_key(&k) || i.class.get(&k).is_some(),
                    _ => false,
                };
                if in_this {
                    let this = frame.this.clone();
                    self.set_member(&this, Value::Str(name.clone()), v.clone(), false)?;
                } else if Table::has(&self.vm.root, &k) {
                    self.vm.root.borrow_mut().map.insert(k, v.clone());
                } else {
                    let this = frame.this.clone();
                    self.set_member(&this, Value::Str(name.clone()), v.clone(), true)?;
                }
                Ok(v)
            }
            Expr::Root(name) => {
                let k = Key::Str(name.clone());
                let old = self.vm.root.borrow().map.get(&k).cloned().unwrap_or(Value::Null);
                let v = combine(self, old, value)?;
                self.vm.root.borrow_mut().map.insert(k, v.clone());
                Ok(v)
            }
            Expr::Member(obj, name) => {
                let o = self.eval(obj, frame)?;
                let key = Value::Str(name.clone());
                let old = if op.len() > 1 && op != "<-" { self.get_member(&o, &key)? } else { Value::Null };
                let v = combine(self, old, value)?;
                self.set_member(&o, key, v.clone(), op == "<-")?;
                Ok(v)
            }
            Expr::Index(obj, idx) => {
                let o = self.eval(obj, frame)?;
                let key = self.eval(idx, frame)?;
                let old = if op.len() > 1 && op != "<-" { self.get_member(&o, &key)? } else { Value::Null };
                let v = combine(self, old, value)?;
                self.set_member(&o, key, v.clone(), op == "<-")?;
                Ok(v)
            }
            Expr::This | Expr::Base => err("cannot assign to this"),
            _ => err("invalid assignment target"),
        }
    }

    fn binop(&mut self, op: &str, a: &Value, b: &Value) -> Res<Value> {
        match op {
            "+" => {
                if matches!(a, Value::Str(_)) || matches!(b, Value::Str(_)) {
                    let s = format!("{}{}", self.to_string(a)?, self.to_string(b)?);
                    return Ok(Value::Str(s.into()));
                }
                if let (Some(x), Some(y)) = (vec3(a), vec3(b)) {
                    return Ok(Value::vector(x[0] + y[0], x[1] + y[1], x[2] + y[2]));
                }
                num_op(op, a, b)
            }
            "-" => {
                if let (Some(x), Some(y)) = (vec3(a), vec3(b)) {
                    return Ok(Value::vector(x[0] - y[0], x[1] - y[1], x[2] - y[2]));
                }
                num_op(op, a, b)
            }
            "*" => {
                if let (Some(x), Some(s)) = (vec3(a), b.as_f64()) {
                    return Ok(Value::vector(x[0] * s, x[1] * s, x[2] * s));
                }
                if let (Some(s), Some(x)) = (a.as_f64(), vec3(b)) {
                    return Ok(Value::vector(x[0] * s, x[1] * s, x[2] * s));
                }
                num_op(op, a, b)
            }
            "/" | "%" => num_op(op, a, b),
            "==" => Ok(Value::Bool(a.equals(b))),
            "!=" => Ok(Value::Bool(!a.equals(b))),
            "<" => Ok(Value::Bool(compare(a, b)?.is_lt())),
            "<=" => Ok(Value::Bool(compare(a, b)?.is_le())),
            ">" => Ok(Value::Bool(compare(a, b)?.is_gt())),
            ">=" => Ok(Value::Bool(compare(a, b)?.is_ge())),
            "<=>" => Ok(Value::Int(compare(a, b)? as i64)),
            "&" | "|" | "^" | "<<" | ">>" => {
                let (Some(x), Some(y)) = (a.as_i64(), b.as_i64()) else { return err(format!("bitwise op on {}", a.type_name())) };
                Ok(Value::Int(match op {
                    "&" => x & y,
                    "|" => x | y,
                    "^" => x ^ y,
                    "<<" => x.wrapping_shl(y as u32),
                    _ => x.wrapping_shr(y as u32),
                }))
            }
            _ => err(format!("unknown operator {op}")),
        }
    }

    fn to_string(&mut self, v: &Value) -> Res<String> {
        // Instances with a _tostring metamethod.
        if let Value::Instance(i) = v {
            if let Some(f) = i.class.get(&Key::str("_tostring")) {
                let r = self.call(&f, v.clone(), Vec::new())?;
                return Ok(r.to_string());
            }
        }
        Ok(v.to_string())
    }

    fn eval(&mut self, e: &Expr, frame: &Frame) -> Res<Value> {
        Ok(match e {
            Expr::Null => Value::Null,
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Int(i) => Value::Int(*i),
            Expr::Float(f) => Value::Float(*f),
            Expr::Str(s) => Value::Str(s.clone()),
            Expr::Ident(n) => self.lookup(n, frame)?,
            Expr::Root(n) => Table::get(&self.vm.root, &Key::Str(n.clone())).map_or_else(|| err(format!("the index '{n}' does not exist")), Ok)?,
            Expr::This => frame.this.clone(),
            Expr::Base => match &frame.this {
                Value::Instance(i) => i.class.base.clone().map(Value::Class).unwrap_or(Value::Null),
                _ => Value::Null,
            },
            Expr::Member(obj, name) => {
                let o = self.eval(obj, frame)?;
                self.get_member(&o, &Value::Str(name.clone()))?
            }
            Expr::Index(obj, idx) => {
                let o = self.eval(obj, frame)?;
                let k = self.eval(idx, frame)?;
                self.get_member(&o, &k)?
            }
            Expr::Call(callee, args) => return self.eval_call(callee, args, frame),
            Expr::Unary(op, x) => {
                let v = self.eval(x, frame)?;
                match *op {
                    "-" => match v {
                        Value::Int(i) => Value::Int(-i),
                        Value::Float(f) => Value::Float(-f),
                        Value::Vector(r) => {
                            let a = *r.borrow();
                            Value::vector(-a[0], -a[1], -a[2])
                        }
                        other => return err(format!("negate a {}", other.type_name())),
                    },
                    "!" => Value::Bool(!v.truthy()),
                    _ => Value::Int(!v.as_i64().unwrap_or(0)),
                }
            }
            Expr::Binary(op, a, b) => {
                let a = self.eval(a, frame)?;
                let b = self.eval(b, frame)?;
                self.binop(op, &a, &b)?
            }
            Expr::And(a, b) => {
                let a = self.eval(a, frame)?;
                if !a.truthy() {
                    a
                } else {
                    self.eval(b, frame)?
                }
            }
            Expr::Or(a, b) => {
                let a = self.eval(a, frame)?;
                if a.truthy() {
                    a
                } else {
                    self.eval(b, frame)?
                }
            }
            Expr::Ternary(c, a, b) => {
                if self.eval(c, frame)?.truthy() {
                    self.eval(a, frame)?
                } else {
                    self.eval(b, frame)?
                }
            }
            Expr::Assign(op, target, value) => {
                let v = self.eval(value, frame)?;
                self.assign_to(target, op, v, frame)?
            }
            Expr::IncDec(target, d, prefix) => {
                let old = self.eval(target, frame)?;
                let new = num_op("+", &old, &Value::Int(*d))?;
                self.assign_to(target, "=", new.clone(), frame)?;
                if *prefix {
                    new
                } else {
                    old
                }
            }
            Expr::Table(entries) => {
                let t = Table::new_ref();
                for (k, v) in entries {
                    let k = self.eval(k, frame)?.key();
                    let v = self.eval(v, frame)?;
                    t.borrow_mut().map.insert(k, v);
                }
                Value::Table(t)
            }
            Expr::Array(items) => {
                let mut out = Vec::with_capacity(items.len());
                for i in items {
                    out.push(self.eval(i, frame)?);
                }
                Value::Array(Rc::new(RefCell::new(out)))
            }
            Expr::Function(def) => self.closure(def, frame)?,
            Expr::Class(def) => self.class(def, frame)?,
            Expr::Typeof(x) => {
                let v = self.eval(x, frame)?;
                Value::str(match v {
                    Value::Native(_) => "function",
                    other => other.type_name(),
                })
            }
            Expr::Delete(x) => match &**x {
                Expr::Member(obj, name) => {
                    let o = self.eval(obj, frame)?;
                    self.delete(&o, Value::Str(name.clone()))?
                }
                Expr::Index(obj, idx) => {
                    let o = self.eval(obj, frame)?;
                    let k = self.eval(idx, frame)?;
                    self.delete(&o, k)?
                }
                _ => return err("delete needs a slot"),
            },
            Expr::Clone(x) => {
                let v = self.eval(x, frame)?;
                match v {
                    Value::Table(t) => {
                        let b = t.borrow();
                        Value::Table(Rc::new(RefCell::new(Table { map: b.map.clone(), delegate: b.delegate.clone() })))
                    }
                    Value::Array(a) => Value::Array(Rc::new(RefCell::new(a.borrow().clone()))),
                    Value::Instance(i) => Value::Instance(Rc::new(Instance { class: i.class.clone(), fields: RefCell::new(i.fields.borrow().clone()) })),
                    Value::Vector(v) => Value::Vector(Rc::new(RefCell::new(*v.borrow()))),
                    other => other,
                }
            }
            Expr::In(k, c) => {
                let k = self.eval(k, frame)?;
                let c = self.eval(c, frame)?;
                let key = k.key();
                Value::Bool(match &c {
                    Value::Table(t) => Table::has(t, &key),
                    Value::Instance(i) => i.fields.borrow().contains_key(&key) || i.class.get(&key).is_some(),
                    Value::Class(cl) => cl.get(&key).is_some(),
                    Value::Array(a) => k.as_i64().is_some_and(|i| i >= 0 && (i as usize) < a.borrow().len()),
                    _ => false,
                })
            }
            Expr::InstanceOf(x, c) => {
                let x = self.eval(x, frame)?;
                let c = self.eval(c, frame)?;
                let (Value::Instance(i), Value::Class(c)) = (&x, &c) else { return Ok(Value::Bool(false)) };
                let mut cur = Some(i.class.clone());
                let mut found = false;
                while let Some(k) = cur {
                    if Rc::ptr_eq(&k, c) {
                        found = true;
                        break;
                    }
                    cur = k.base.clone();
                }
                Value::Bool(found)
            }
        })
    }

    fn delete(&mut self, obj: &Value, key: Value) -> Res<Value> {
        match obj {
            Value::Table(t) => Ok(t.borrow_mut().map.shift_remove(&key.key()).unwrap_or(Value::Null)),
            Value::Instance(i) => Ok(i.fields.borrow_mut().shift_remove(&key.key()).unwrap_or(Value::Null)),
            _ => err(format!("cannot delete from a {}", obj.type_name())),
        }
    }

    fn eval_call(&mut self, callee: &Expr, args: &[Expr], frame: &Frame) -> Res<Value> {
        let mut argv = Vec::with_capacity(args.len());
        // `obj.name(...)`: method call with this = obj.
        if let Expr::Member(obj, name) = callee {
            let o = self.eval(obj, frame)?;
            for a in args {
                argv.push(self.eval(a, frame)?);
            }
            return self.method(o, name, argv);
        }
        let (f, this) = match callee {
            Expr::Index(obj, idx) => {
                let o = self.eval(obj, frame)?;
                let k = self.eval(idx, frame)?;
                (self.get_member(&o, &k)?, o)
            }
            Expr::Base => {
                // base(...) in a constructor: the base class's constructor.
                let f = match &frame.this {
                    Value::Instance(i) => i.class.base.as_ref().and_then(|b| b.get(&Key::str("constructor"))).unwrap_or(Value::Null),
                    _ => Value::Null,
                };
                (f, frame.this.clone())
            }
            other => (self.eval(other, frame)?, frame.this.clone()),
        };
        for a in args {
            argv.push(self.eval(a, frame)?);
        }
        self.call(&f, this, argv)
    }

    fn method(&mut self, o: Value, name: &str, args: Vec<Value>) -> Res<Value> {
        let key = Key::str(name);
        // Slots first (tables, instances, classes).
        let slot = match &o {
            Value::Table(t) => Table::get(t, &key),
            Value::Instance(i) => i.fields.borrow().get(&key).cloned().or_else(|| i.class.get(&key)),
            Value::Class(c) => c.get(&key),
            _ => None,
        };
        if let Some(f) = slot {
            return self.call(&f, o, args);
        }
        crate::builtins::method(self, o, name, args)
    }

    pub(crate) fn call(&mut self, f: &Value, this: Value, args: Vec<Value>) -> Res<Value> {
        match f {
            Value::Closure(c) => {
                if self.vm.depth >= DEPTH_LIMIT {
                    return err("stack overflow");
                }
                let this = c.env.clone().unwrap_or(this);
                let scope = Scope::new(c.scope.clone());
                let mut extra = Vec::new();
                for (i, a) in args.into_iter().enumerate() {
                    if i < c.def.params.len() {
                        scope.declare(c.def.params[i].0.clone(), a);
                    } else {
                        extra.push(a);
                    }
                }
                let given = scope.vars.borrow().len();
                for i in given..c.def.params.len() {
                    scope.declare(c.def.params[i].0.clone(), c.defaults[i].clone().unwrap_or(Value::Null));
                }
                if c.def.varargs {
                    scope.declare("vargv".into(), Value::Array(Rc::new(RefCell::new(extra))));
                }
                self.vm.depth += 1;
                self.vm.trace.push(c.def.name.clone());
                let r = self.block(&c.def.body, &Frame { scope, this });
                self.vm.depth -= 1;
                let v = match r? {
                    Flow::Return(v) => v,
                    _ => Value::Null,
                };
                self.vm.trace.pop();
                Ok(v)
            }
            Value::Native(name) => crate::builtins::call_native(self, name, this, args),
            Value::Class(c) => {
                let inst = Rc::new(Instance { class: c.clone(), fields: RefCell::new(IndexMap::new()) });
                // Fields from the class chain, base first.
                let mut chain = Vec::new();
                let mut cur = Some(c.clone());
                while let Some(k) = cur {
                    cur = k.base.clone();
                    chain.push(k);
                }
                for k in chain.iter().rev() {
                    for (key, v) in k.members.borrow().iter() {
                        if !matches!(v, Value::Closure(_) | Value::Native(_)) {
                            inst.fields.borrow_mut().insert(key.clone(), v.clone());
                        }
                    }
                }
                let v = Value::Instance(inst);
                if let Some(ctor) = c.get(&Key::str("constructor")) {
                    self.call(&ctor, v.clone(), args)?;
                }
                Ok(v)
            }
            other => err(format!("attempt to call a {}", other.type_name())),
        }
    }

    pub(crate) fn host(&mut self) -> &mut dyn Host {
        self.host
    }
    pub(crate) fn vm(&mut self) -> &mut Vm {
        self.vm
    }
    pub(crate) fn include(&mut self, name: &str, scope: Value) -> Res<Value> {
        let name = name.trim_end_matches(".nut").to_string();
        let key = format!("file:{name}");
        let prog = match self.vm.parsed.get(&key) {
            Some(p) => p.clone(),
            None => {
                let Some(src) = self.host.read_script(&name) else { return err(format!("script {name} not found")) };
                match self.vm.parse_cached(&key, &src) {
                    Ok(p) => p,
                    Err(e) => return err(format!("{name}: {e}")),
                }
            }
        };
        let this = match scope {
            Value::Null => Value::Table(self.vm.root.clone()),
            other => other,
        };
        self.block(&prog, &Frame { scope: Scope::new(None), this })?;
        Ok(Value::Bool(true))
    }
    pub(crate) fn stringify(&mut self, v: &Value) -> Res<String> {
        self.to_string(v)
    }
    pub(crate) fn compare_values(&mut self, f: Option<&Value>, a: &Value, b: &Value) -> Res<std::cmp::Ordering> {
        match f {
            Some(f) => {
                let r = self.call(f, Value::Null, vec![a.clone(), b.clone()])?;
                Ok(r.as_i64().unwrap_or(0).cmp(&0))
            }
            None => compare(a, b),
        }
    }
}

pub(crate) use Cx as Context;
pub(crate) fn throw<T>(msg: impl Into<String>) -> Result<T, Exc> {
    err(msg)
}
