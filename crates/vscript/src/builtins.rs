//! Built-in functions and the methods every value type has.
use std::cell::RefCell;
use std::rc::Rc;

use crate::interp::{throw, Context, Exc};
use crate::value::*;

type Res<T> = Result<T, Exc>;

fn arg(args: &[Value], i: usize) -> Value {
    args.get(i).cloned().unwrap_or(Value::Null)
}
fn f(args: &[Value], i: usize) -> f64 {
    args.get(i).and_then(Value::as_f64).unwrap_or(0.0)
}

pub(crate) fn call_native(cx: &mut Context, name: &str, this: Value, args: Vec<Value>) -> Res<Value> {
    Ok(match name {
        "print" => {
            let s = cx.stringify(&arg(&args, 0))?;
            cx.host().print(&s);
            Value::Null
        }
        "printl" => {
            let s = cx.stringify(&arg(&args, 0))?;
            cx.host().print(&format!("{s}\n"));
            Value::Null
        }
        "array" => {
            let n = arg(&args, 0).as_i64().unwrap_or(0).max(0) as usize;
            Value::Array(Rc::new(RefCell::new(vec![arg(&args, 1); n])))
        }
        "getroottable" => Value::Table(cx.vm().root.clone()),
        "assert" => {
            if !arg(&args, 0).truthy() {
                return throw("assertion failed");
            }
            Value::Null
        }
        "type" => Value::str(arg(&args, 0).type_name()),
        "abs" => match arg(&args, 0) {
            Value::Int(i) => Value::Int(i.abs()),
            v => Value::Float(v.as_f64().unwrap_or(0.0).abs()),
        },
        "fabs" => Value::Float(f(&args, 0).abs()),
        "sqrt" => Value::Float(f(&args, 0).sqrt()),
        "floor" => Value::Float(f(&args, 0).floor()),
        "ceil" => Value::Float(f(&args, 0).ceil()),
        "pow" => Value::Float(f(&args, 0).powf(f(&args, 1))),
        "sin" => Value::Float(f(&args, 0).sin()),
        "cos" => Value::Float(f(&args, 0).cos()),
        "tan" => Value::Float(f(&args, 0).tan()),
        "atan2" => Value::Float(f(&args, 0).atan2(f(&args, 1))),
        "max" => {
            if f(&args, 0) >= f(&args, 1) {
                arg(&args, 0)
            } else {
                arg(&args, 1)
            }
        }
        "min" => {
            if f(&args, 0) <= f(&args, 1) {
                arg(&args, 0)
            } else {
                arg(&args, 1)
            }
        }
        "clamp" => Value::Float(f(&args, 0).clamp(f(&args, 1), f(&args, 2).max(f(&args, 1)))),
        "rand" => {
            // The host may provide a seeded one; this is a fallback.
            match cx.host().call("RandomInt", &[Value::Int(0), Value::Int(32767)]) {
                Some(r) => r.map_err(|e| Exc(Value::Str(e.into())))?,
                None => Value::Int(0),
            }
        }
        "Vector" => Value::vector(f(&args, 0), f(&args, 1), f(&args, 2)),
        "IncludeScript" | "DoIncludeScript" => {
            let name = arg(&args, 0).to_string();
            cx.include(&name, arg(&args, 1))?
        }
        "split" => {
            let s = arg(&args, 0).to_string();
            let seps = arg(&args, 1).to_string();
            let parts: Vec<Value> = s.split(|c| seps.contains(c)).filter(|p| !p.is_empty()).map(Value::str).collect();
            Value::Array(Rc::new(RefCell::new(parts)))
        }
        "strip" => Value::str(arg(&args, 0).to_string().trim()),
        "lstrip" => Value::str(arg(&args, 0).to_string().trim_start()),
        "rstrip" => Value::str(arg(&args, 0).to_string().trim_end()),
        "format" => {
            // %d %s %f %i %.Nf, enough for debug prints.
            let fmt = arg(&args, 0).to_string();
            let mut out = String::new();
            let mut ai = 1;
            let mut chars = fmt.chars().peekable();
            while let Some(c) = chars.next() {
                if c != '%' {
                    out.push(c);
                    continue;
                }
                let mut spec = String::new();
                while let Some(&n) = chars.peek() {
                    spec.push(n);
                    chars.next();
                    if n.is_ascii_alphabetic() || n == '%' {
                        break;
                    }
                }
                if spec == "%" {
                    out.push('%');
                    continue;
                }
                let v = arg(&args, ai);
                ai += 1;
                match spec.chars().last() {
                    Some('d') | Some('i') => out.push_str(&v.as_i64().unwrap_or(0).to_string()),
                    Some('f') | Some('g') => {
                        let prec = spec.split('.').nth(1).and_then(|p| p.trim_end_matches(|c: char| c.is_ascii_alphabetic()).parse::<usize>().ok());
                        match prec {
                            Some(p) => out.push_str(&format!("{:.*}", p, v.as_f64().unwrap_or(0.0))),
                            None => out.push_str(&v.as_f64().unwrap_or(0.0).to_string()),
                        }
                    }
                    _ => out.push_str(&cx.stringify(&v)?),
                }
            }
            Value::Str(out.into())
        }
        "seterrorhandler" | "collectgarbage" => Value::Null,
        other => {
            let _ = this;
            match cx.host().call(other, &args) {
                Some(Ok(v)) => v,
                Some(Err(e)) => return throw(e),
                None => return throw(format!("the function '{other}' is not available")),
            }
        }
    })
}

pub(crate) fn method(cx: &mut Context, o: Value, name: &str, args: Vec<Value>) -> Res<Value> {
    // Every type.
    match name {
        "tostring" => return Ok(Value::Str(cx.stringify(&o)?.into())),
        "tointeger" => {
            return Ok(match &o {
                Value::Str(s) => Value::Int(s.trim().parse::<i64>().or_else(|_| s.trim().parse::<f64>().map(|f| f as i64)).unwrap_or(0)),
                v => Value::Int(v.as_i64().unwrap_or(0)),
            })
        }
        "tofloat" => {
            return Ok(match &o {
                Value::Str(s) => Value::Float(s.trim().parse::<f64>().unwrap_or(0.0)),
                v => Value::Float(v.as_f64().unwrap_or(0.0)),
            })
        }
        "weakref" | "ref" => return Ok(o),
        _ => {}
    }
    match &o {
        Value::Int(i) if name == "tochar" => Ok(Value::Str(char::from_u32(*i as u32).unwrap_or('?').to_string().into())),
        Value::Str(s) => string_method(s, name, &args),
        Value::Array(a) => array_method(cx, a, name, args),
        Value::Table(t) => table_method(t, name, &args),
        Value::Closure(_) | Value::Native(_) => match name {
            "bindenv" => match &o {
                Value::Closure(c) => Ok(Value::Closure(Rc::new(Closure { def: c.def.clone(), scope: c.scope.clone(), env: Some(arg(&args, 0)), defaults: c.defaults.clone() }))),
                other => Ok(other.clone()),
            },
            "call" | "pcall" => {
                let this = arg(&args, 0);
                cx.call(&o, this, args.into_iter().skip(1).collect())
            }
            "acall" | "pacall" => {
                let list = match arg(&args, 0) {
                    Value::Array(a) => a.borrow().clone(),
                    _ => Vec::new(),
                };
                let this = list.first().cloned().unwrap_or(Value::Null);
                cx.call(&o, this, list.into_iter().skip(1).collect())
            }
            _ => throw(format!("function has no method {name}")),
        },
        Value::Instance(i) => match name {
            "getclass" => Ok(Value::Class(i.class.clone())),
            _ => throw(format!("the index '{name}' does not exist")),
        },
        Value::Vector(v) => {
            let a = *v.borrow();
            let b = vec_arg(&args, 0);
            Ok(match name {
                "Length" => Value::Float((a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt()),
                "LengthSqr" => Value::Float(a[0] * a[0] + a[1] * a[1] + a[2] * a[2]),
                "Length2D" => Value::Float((a[0] * a[0] + a[1] * a[1]).sqrt()),
                "Length2DSqr" => Value::Float(a[0] * a[0] + a[1] * a[1]),
                "Dot" => Value::Float(a[0] * b[0] + a[1] * b[1] + a[2] * b[2]),
                "Cross" => Value::vector(a[1] * b[2] - a[2] * b[1], a[2] * b[0] - a[0] * b[2], a[0] * b[1] - a[1] * b[0]),
                "Norm" => {
                    let l = (a[0] * a[0] + a[1] * a[1] + a[2] * a[2]).sqrt();
                    if l > 0.0 {
                        *v.borrow_mut() = [a[0] / l, a[1] / l, a[2] / l];
                    }
                    Value::Float(l)
                }
                "ToKVString" => Value::Str(format!("{} {} {}", a[0], a[1], a[2]).into()),
                _ => return throw(format!("vector has no method {name}")),
            })
        }
        Value::Handle(h) => {
            let h = *h;
            match name {
                "GetScriptScope" => Ok(cx.vm().scopes.get(&h).cloned().map(Value::Table).unwrap_or(Value::Null)),
                "ValidateScriptScope" => {
                    let s = cx.vm().scope(h);
                    s.borrow_mut().map.entry(Key::str("self")).or_insert(Value::Handle(h));
                    Ok(Value::Bool(true))
                }
                _ => cx.host().method(h, name, &args).map_err(|e| Exc(Value::Str(e.into()))),
            }
        }
        other => throw(format!("{} has no method {name}", other.type_name())),
    }
}

fn vec_arg(args: &[Value], i: usize) -> [f64; 3] {
    match args.get(i) {
        Some(Value::Vector(v)) => *v.borrow(),
        _ => [0.0; 3],
    }
}

fn string_method(s: &Rc<str>, name: &str, args: &[Value]) -> Res<Value> {
    Ok(match name {
        "len" => Value::Int(s.len() as i64),
        "tolower" => Value::str(&s.to_lowercase()),
        "toupper" => Value::str(&s.to_uppercase()),
        "slice" => {
            let len = s.len() as i64;
            let fix = |v: i64| if v < 0 { (len + v).max(0) } else { v.min(len) } as usize;
            let a = fix(arg(args, 0).as_i64().unwrap_or(0));
            let b = match arg(args, 1) {
                Value::Null => len as usize,
                v => fix(v.as_i64().unwrap_or(len)),
            };
            Value::str(s.get(a..b.max(a)).unwrap_or(""))
        }
        "find" => {
            let needle = arg(args, 0).to_string();
            let start = arg(args, 1).as_i64().unwrap_or(0).max(0) as usize;
            match s.get(start..).and_then(|rest| rest.find(&needle)) {
                Some(i) => Value::Int((i + start) as i64),
                None => Value::Null,
            }
        }
        _ => return throw(format!("string has no method {name}")),
    })
}

fn array_method(cx: &mut Context, a: &ArrayRef, name: &str, args: Vec<Value>) -> Res<Value> {
    Ok(match name {
        "len" => Value::Int(a.borrow().len() as i64),
        "append" | "push" => {
            a.borrow_mut().push(arg(&args, 0));
            Value::Null
        }
        "extend" => {
            if let Value::Array(b) = arg(&args, 0) {
                let items = b.borrow().clone();
                a.borrow_mut().extend(items);
            }
            Value::Null
        }
        "pop" => a.borrow_mut().pop().unwrap_or(Value::Null),
        "top" => a.borrow().last().cloned().unwrap_or(Value::Null),
        "insert" => {
            let i = arg(&args, 0).as_i64().unwrap_or(0).max(0) as usize;
            let mut v = a.borrow_mut();
            let i = i.min(v.len());
            v.insert(i, arg(&args, 1));
            Value::Null
        }
        "remove" => {
            let i = arg(&args, 0).as_i64().unwrap_or(0);
            let mut v = a.borrow_mut();
            if i < 0 || i as usize >= v.len() {
                return throw("array index out of range");
            }
            v.remove(i as usize)
        }
        "clear" => {
            a.borrow_mut().clear();
            Value::Null
        }
        "resize" => {
            let n = arg(&args, 0).as_i64().unwrap_or(0).max(0) as usize;
            a.borrow_mut().resize(n, arg(&args, 1));
            Value::Null
        }
        "reverse" => {
            a.borrow_mut().reverse();
            Value::Null
        }
        "slice" => {
            let v = a.borrow();
            let len = v.len() as i64;
            let fix = |x: i64| if x < 0 { (len + x).max(0) } else { x.min(len) } as usize;
            let s = fix(arg(&args, 0).as_i64().unwrap_or(0));
            let e = match arg(&args, 1) {
                Value::Null => len as usize,
                x => fix(x.as_i64().unwrap_or(len)),
            };
            Value::Array(Rc::new(RefCell::new(v.get(s..e.max(s)).unwrap_or(&[]).to_vec())))
        }
        "find" => {
            let needle = arg(&args, 0);
            match a.borrow().iter().position(|v| v.equals(&needle)) {
                Some(i) => Value::Int(i as i64),
                None => Value::Null,
            }
        }
        "sort" => {
            let cmp = args.first().cloned();
            let mut items = a.borrow().clone();
            // Insertion sort: the comparator can call script code (and fail).
            for i in 1..items.len() {
                let mut j = i;
                while j > 0 && cx.compare_values(cmp.as_ref(), &items[j - 1], &items[j])?.is_gt() {
                    items.swap(j - 1, j);
                    j -= 1;
                }
            }
            *a.borrow_mut() = items;
            Value::Null
        }
        "map" | "apply" | "filter" => {
            let func = arg(&args, 0);
            let items = a.borrow().clone();
            let mut out = Vec::new();
            for (i, v) in items.into_iter().enumerate() {
                match name {
                    "filter" => {
                        if cx.call(&func, Value::Null, vec![Value::Int(i as i64), v.clone()])?.truthy() {
                            out.push(v);
                        }
                    }
                    _ => out.push(cx.call(&func, Value::Null, vec![v])?),
                }
            }
            if name == "apply" {
                *a.borrow_mut() = out;
                Value::Null
            } else {
                Value::Array(Rc::new(RefCell::new(out)))
            }
        }
        _ => return throw(format!("array has no method {name}")),
    })
}

fn table_method(t: &TableRef, name: &str, args: &[Value]) -> Res<Value> {
    Ok(match name {
        "len" => Value::Int(t.borrow().map.len() as i64),
        "rawin" => Value::Bool(t.borrow().map.contains_key(&arg(args, 0).key())),
        "rawget" => t.borrow().map.get(&arg(args, 0).key()).cloned().unwrap_or(Value::Null),
        "rawset" => {
            t.borrow_mut().map.insert(arg(args, 0).key(), arg(args, 1));
            Value::Null
        }
        "rawdelete" => t.borrow_mut().map.shift_remove(&arg(args, 0).key()).unwrap_or(Value::Null),
        "clear" => {
            t.borrow_mut().map.clear();
            Value::Null
        }
        "keys" => Value::Array(Rc::new(RefCell::new(t.borrow().map.keys().map(Value::from_key).collect()))),
        "values" => Value::Array(Rc::new(RefCell::new(t.borrow().map.values().cloned().collect()))),
        "getdelegate" => t.borrow().delegate.clone().map(Value::Table).unwrap_or(Value::Null),
        "setdelegate" => {
            t.borrow_mut().delegate = match arg(args, 0) {
                Value::Table(d) => Some(d),
                _ => None,
            };
            Value::Table(t.clone())
        }
        _ => return throw(format!("the index '{name}' does not exist")),
    })
}
