//! Squirrel values.
use std::cell::RefCell;
use std::fmt;
use std::rc::Rc;

use indexmap::IndexMap;

use crate::ast::{ClassDef, FuncDef};

pub type TableRef = Rc<RefCell<Table>>;
pub type ArrayRef = Rc<RefCell<Vec<Value>>>;

/// A table: ordered slots and an optional delegate (looked in for missing keys).
#[derive(Default, Debug)]
pub struct Table {
    pub map: IndexMap<Key, Value>,
    pub delegate: Option<TableRef>,
}

impl Table {
    pub fn new_ref() -> TableRef {
        Rc::new(RefCell::new(Table::default()))
    }
    /// Look up `k` here, then in the delegates.
    pub fn get(t: &TableRef, k: &Key) -> Option<Value> {
        let mut cur = t.clone();
        for _ in 0..32 {
            let next = {
                let b = cur.borrow();
                if let Some(v) = b.map.get(k) {
                    return Some(v.clone());
                }
                b.delegate.clone()
            };
            cur = next?;
        }
        None
    }
    pub fn has(t: &TableRef, k: &Key) -> bool {
        Table::get(t, k).is_some()
    }
}

/// A table key.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Key {
    Null,
    Bool(bool),
    Int(i64),
    Float(u64),
    Str(Rc<str>),
    /// Reference types by identity.
    Ptr(usize),
    Handle(u64),
}

impl Key {
    pub fn str(s: &str) -> Key {
        Key::Str(s.into())
    }
}

/// Local variable scope: a function's (or block's) variables, with the
/// enclosing scope closures were made in.
#[derive(Debug)]
pub struct Scope {
    pub vars: RefCell<Vec<(Rc<str>, Value)>>,
    pub parent: Option<Rc<Scope>>,
}

impl Scope {
    pub fn new(parent: Option<Rc<Scope>>) -> Rc<Scope> {
        Rc::new(Scope { vars: RefCell::new(Vec::new()), parent })
    }
    pub fn lookup(self: &Rc<Scope>, name: &str) -> Option<Value> {
        let mut cur = Some(self.clone());
        while let Some(s) = cur {
            if let Some((_, v)) = s.vars.borrow().iter().rev().find(|(n, _)| &**n == name) {
                return Some(v.clone());
            }
            cur = s.parent.clone();
        }
        None
    }
    /// Set an existing local; false when there is none by that name.
    pub fn set(self: &Rc<Scope>, name: &str, v: Value) -> bool {
        let mut cur = Some(self.clone());
        while let Some(s) = cur {
            {
                let mut vars = s.vars.borrow_mut();
                if let Some(slot) = vars.iter_mut().rev().find(|(n, _)| &**n == name) {
                    slot.1 = v;
                    return true;
                }
            }
            cur = s.parent.clone();
        }
        false
    }
    pub fn declare(&self, name: Rc<str>, v: Value) {
        self.vars.borrow_mut().push((name, v));
    }
}

#[derive(Debug)]
pub struct Closure {
    pub def: Rc<FuncDef>,
    pub scope: Option<Rc<Scope>>,
    /// Environment bound with `bindenv` (the `this` of every call).
    pub env: Option<Value>,
    /// Default parameter values, evaluated when the function was made.
    pub defaults: Vec<Option<Value>>,
}

#[derive(Debug)]
pub struct Class {
    pub def: Rc<ClassDef>,
    pub base: Option<Rc<Class>>,
    pub members: RefCell<IndexMap<Key, Value>>,
}

impl Class {
    pub fn get(&self, k: &Key) -> Option<Value> {
        if let Some(v) = self.members.borrow().get(k) {
            return Some(v.clone());
        }
        self.base.as_ref().and_then(|b| b.get(k))
    }
}

#[derive(Debug)]
pub struct Instance {
    pub class: Rc<Class>,
    pub fields: RefCell<IndexMap<Key, Value>>,
}

#[derive(Clone, Debug, Default)]
pub enum Value {
    #[default]
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    Table(TableRef),
    Array(ArrayRef),
    Closure(Rc<Closure>),
    /// A built-in function, by name (the interpreter or the host runs it).
    Native(Rc<str>),
    Class(Rc<Class>),
    Instance(Rc<Instance>),
    /// An engine object (an entity, `Entities`...), by the host's id.
    Handle(u64),
    /// `Vector(x, y, z)`.
    Vector(Rc<RefCell<[f64; 3]>>),
}

impl Value {
    pub fn str(s: &str) -> Value {
        Value::Str(s.into())
    }
    pub fn vector(x: f64, y: f64, z: f64) -> Value {
        Value::Vector(Rc::new(RefCell::new([x, y, z])))
    }
    pub fn truthy(&self) -> bool {
        match self {
            Value::Null => false,
            Value::Bool(b) => *b,
            Value::Int(i) => *i != 0,
            Value::Float(f) => *f != 0.0,
            _ => true,
        }
    }
    pub fn type_name(&self) -> &'static str {
        match self {
            Value::Null => "null",
            Value::Bool(_) => "bool",
            Value::Int(_) => "integer",
            Value::Float(_) => "float",
            Value::Str(_) => "string",
            Value::Table(_) => "table",
            Value::Array(_) => "array",
            Value::Closure(_) => "function",
            Value::Native(_) => "native function",
            Value::Class(_) => "class",
            Value::Instance(_) | Value::Handle(_) | Value::Vector(_) => "instance",
        }
    }
    pub fn as_f64(&self) -> Option<f64> {
        match self {
            Value::Int(i) => Some(*i as f64),
            Value::Float(f) => Some(*f),
            Value::Bool(b) => Some(*b as i64 as f64),
            _ => None,
        }
    }
    pub fn as_i64(&self) -> Option<i64> {
        match self {
            Value::Int(i) => Some(*i),
            Value::Float(f) => Some(*f as i64),
            Value::Bool(b) => Some(*b as i64),
            _ => None,
        }
    }
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Value::Str(s) => Some(s),
            _ => None,
        }
    }
    pub fn key(&self) -> Key {
        match self {
            Value::Null => Key::Null,
            Value::Bool(b) => Key::Bool(*b),
            Value::Int(i) => Key::Int(*i),
            Value::Float(f) => Key::Float(f.to_bits()),
            Value::Str(s) => Key::Str(s.clone()),
            Value::Table(t) => Key::Ptr(Rc::as_ptr(t) as *const () as usize),
            Value::Array(a) => Key::Ptr(Rc::as_ptr(a) as *const () as usize),
            Value::Closure(c) => Key::Ptr(Rc::as_ptr(c) as *const () as usize),
            Value::Native(n) => Key::Str(format!("native:{n}").into()),
            Value::Class(c) => Key::Ptr(Rc::as_ptr(c) as *const () as usize),
            Value::Instance(i) => Key::Ptr(Rc::as_ptr(i) as *const () as usize),
            Value::Handle(h) => Key::Handle(*h),
            Value::Vector(v) => Key::Ptr(Rc::as_ptr(v) as *const () as usize),
        }
    }
    pub fn from_key(k: &Key) -> Value {
        match k {
            Key::Null => Value::Null,
            Key::Bool(b) => Value::Bool(*b),
            Key::Int(i) => Value::Int(*i),
            Key::Float(f) => Value::Float(f64::from_bits(*f)),
            Key::Str(s) => Value::Str(s.clone()),
            Key::Ptr(p) => Value::Int(*p as i64),
            Key::Handle(h) => Value::Handle(*h),
        }
    }
    /// Squirrel `==`.
    pub fn equals(&self, o: &Value) -> bool {
        match (self, o) {
            (Value::Null, Value::Null) => true,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Int(_) | Value::Float(_), Value::Int(_) | Value::Float(_)) => match (self, o) {
                (Value::Int(a), Value::Int(b)) => a == b,
                _ => self.as_f64() == o.as_f64(),
            },
            (Value::Handle(a), Value::Handle(b)) => a == b,
            (Value::Vector(a), Value::Vector(b)) => *a.borrow() == *b.borrow(),
            (Value::Null, _) | (_, Value::Null) => false,
            _ => self.key() == o.key() && !matches!(self, Value::Int(_) | Value::Float(_)),
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Null => write!(f, "null"),
            Value::Bool(b) => write!(f, "{b}"),
            Value::Int(i) => write!(f, "{i}"),
            Value::Float(v) => {
                if v.fract() == 0.0 && v.abs() < 1e15 {
                    write!(f, "{v:.0}")
                } else {
                    write!(f, "{v}")
                }
            }
            Value::Str(s) => write!(f, "{s}"),
            Value::Table(t) => write!(f, "(table : {:p})", Rc::as_ptr(t)),
            Value::Array(a) => write!(f, "(array : {:p})", Rc::as_ptr(a)),
            Value::Closure(c) => write!(f, "(function : {})", c.def.name),
            Value::Native(n) => write!(f, "(native function : {n})"),
            Value::Class(c) => write!(f, "(class : {})", c.def.name),
            Value::Instance(i) => write!(f, "(instance : {})", i.class.def.name),
            Value::Handle(h) => write!(f, "([{h}])"),
            Value::Vector(v) => {
                let v = v.borrow();
                write!(f, "(vector : ({}, {}, {}))", v[0], v[1], v[2])
            }
        }
    }
}
