//! A small interpreter for the subset of Squirrel that Portal 2's level
//! scripts (`scripts/vscripts/*.nut`) are written in: tables, arrays,
//! closures, classes, `foreach`, `switch`, `try`. Our own implementation;
//! the engine side (entities, sounds, scenes) comes in through [`Host`].
pub mod ast;
mod builtins;
mod interp;
pub mod lexer;
pub mod parser;
pub mod value;

pub use interp::{Exc, Host, Vm};
pub use value::{Key, Table, TableRef, Value};

#[cfg(test)]
mod tests {
    use super::*;

    struct TestHost {
        out: String,
        fired: Vec<String>,
    }

    impl Host for TestHost {
        fn call(&mut self, name: &str, args: &[Value]) -> Option<Result<Value, String>> {
            match name {
                "EntFire" => {
                    self.fired.push(args.iter().map(|a| a.to_string()).collect::<Vec<_>>().join(","));
                    Some(Ok(Value::Null))
                }
                "Time" => Some(Ok(Value::Float(1.5))),
                _ => None,
            }
        }
        fn method(&mut self, handle: u64, name: &str, _args: &[Value]) -> Result<Value, String> {
            match name {
                "GetName" => Ok(Value::Str(format!("ent{handle}").into())),
                _ => Err(format!("no method {name}")),
            }
        }
        fn read_script(&mut self, name: &str) -> Option<String> {
            (name == "inc").then(|| "IncValue <- 7\nfunction IncFn() { return IncValue * 2 }".to_string())
        }
        fn print(&mut self, text: &str) {
            self.out.push_str(text);
        }
    }

    fn run(src: &str) -> (Vm, TestHost) {
        let mut vm = Vm::new(&["EntFire", "Time"]);
        let mut host = TestHost { out: String::new(), fired: Vec::new() };
        let scope = vm.scope(1);
        scope.borrow_mut().map.insert(Key::str("self"), Value::Handle(1));
        vm.run_code(scope, src, &mut host).unwrap_or_else(|e| panic!("{e}"));
        (vm, host)
    }

    #[test]
    fn arithmetic_strings_and_control_flow() {
        let (_, h) = run(r#"
local total = 0
for (local i = 1; i <= 4; i++) total += i
local s = "n=" + total + " f=" + (7 / 2) + " " + (7.0 / 2)
printl(s)
local t = { a = 1, b = 2 }
t.c <- 3
local keys = 0
foreach (k, v in t) keys += v
printl(keys + " " + ("c" in t) + " " + t.len())
local arr = [3, 1, 2]
arr.sort()
arr.append(9)
printl(arr[0] + "," + arr.top() + "," + arr.len())
switch (total) { case 10: printl("ten"); case 11: printl("fall"); break; default: printl("no") }
try { throw "boom" } catch (e) { printl("caught " + e) }
printl(typeof t + typeof arr + typeof 1 + typeof 1.0 + typeof "s" + typeof null)
"#);
        assert_eq!(h.out, "n=10 f=3 3.5\n6 true 3\n1,9,4\nten\nfall\ncaught boom\ntablearrayintegerfloatstringnull\n");
    }

    #[test]
    fn functions_classes_and_scopes() {
        let (mut vm, mut h) = run(r#"
counter <- 0
function Bump(by = 1) { counter += by; return counter }
Bump(); Bump(5)
class Scene {
    name = null
    next = null
    constructor(n) { name = n }
    function Describe() { return "scene " + name }
}
s <- Scene("a")
printl(s.Describe() + " " + counter + " " + (typeof s))
function Later() { EntFire("door", "Open", "", 0.5) }
DoIncludeScript("inc", this)
printl(IncFn())
printl(self.GetName())
"#);
        assert_eq!(h.out, "scene a 6 instance\n14\nent1\n");
        // A function in the scope, called later (an input's RunScriptCode).
        let scope = vm.scope(1);
        vm.run_code(scope.clone(), "Later()", &mut h).unwrap();
        assert_eq!(h.fired, vec!["door,Open,,0.5".to_string()]);
        let r = vm.call_in(scope, "Bump", &[Value::Int(10)], &mut h).unwrap();
        assert_eq!(r.unwrap().as_i64(), Some(16));
    }
}
