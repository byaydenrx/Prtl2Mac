//! Squirrel parser: tokens to [`Stmt`]s.
use std::rc::Rc;

use crate::ast::*;
use crate::lexer::{Tok, Token};

pub struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

type R<T> = Result<T, String>;

impl Parser {
    pub fn new(toks: Vec<Token>) -> Self {
        Parser { toks, pos: 0 }
    }

    fn peek(&self) -> &Tok {
        &self.toks[self.pos].tok
    }
    fn line(&self) -> u32 {
        self.toks[self.pos].line
    }
    fn newline_before(&self) -> bool {
        self.toks[self.pos].newline_before
    }
    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].tok.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn is_op(&self, op: &str) -> bool {
        matches!(self.peek(), Tok::Op(o) if *o == op)
    }
    fn is_kw(&self, kw: &str) -> bool {
        matches!(self.peek(), Tok::Ident(i) if i == kw)
    }
    fn eat_op(&mut self, op: &str) -> bool {
        if self.is_op(op) {
            self.next();
            true
        } else {
            false
        }
    }
    fn eat_kw(&mut self, kw: &str) -> bool {
        if self.is_kw(kw) {
            self.next();
            true
        } else {
            false
        }
    }
    fn expect_op(&mut self, op: &str) -> R<()> {
        if self.eat_op(op) {
            Ok(())
        } else {
            Err(format!("line {}: expected '{op}', found {:?}", self.line(), self.peek()))
        }
    }
    fn ident(&mut self) -> R<Rc<str>> {
        match self.next() {
            Tok::Ident(s) => Ok(s.into()),
            t => Err(format!("line {}: expected a name, found {t:?}", self.line())),
        }
    }

    pub fn program(&mut self) -> R<Vec<Stmt>> {
        let mut out = Vec::new();
        while *self.peek() != Tok::Eof {
            out.push(self.statement()?);
        }
        Ok(out)
    }

    fn block_or_statement(&mut self) -> R<Vec<Stmt>> {
        if self.is_op("{") {
            self.next();
            let mut out = Vec::new();
            while !self.is_op("}") {
                if *self.peek() == Tok::Eof {
                    return Err("unexpected end of script in a block".into());
                }
                out.push(self.statement()?);
            }
            self.next();
            Ok(out)
        } else {
            Ok(vec![self.statement()?])
        }
    }

    fn end_statement(&mut self) {
        self.eat_op(";");
    }

    fn statement(&mut self) -> R<Stmt> {
        if self.eat_op(";") {
            return Ok(Stmt::Block(Vec::new()));
        }
        if self.is_op("{") {
            return Ok(Stmt::Block(self.block_or_statement()?));
        }
        if let Tok::Ident(kw) = self.peek().clone() {
            match kw.as_str() {
                "local" => {
                    self.next();
                    if self.eat_kw("function") {
                        let name = self.ident()?;
                        let f = self.function_rest(name.clone())?;
                        return Ok(Stmt::Local(vec![(name, Some(Expr::Function(f)))]));
                    }
                    let mut vars = Vec::new();
                    loop {
                        let name = self.ident()?;
                        let init = if self.eat_op("=") { Some(self.expr()?) } else { None };
                        vars.push((name, init));
                        if !self.eat_op(",") {
                            break;
                        }
                    }
                    self.end_statement();
                    return Ok(Stmt::Local(vars));
                }
                "function" => {
                    self.next();
                    // function a.b::c() / function ::f() / function f()
                    let mut path = if self.eat_op("::") { Expr::Root(self.ident()?) } else { Expr::Ident(self.ident()?) };
                    let mut last: Rc<str> = match &path {
                        Expr::Root(n) | Expr::Ident(n) => n.clone(),
                        _ => unreachable!(),
                    };
                    while self.is_op(".") || self.is_op("::") {
                        self.next();
                        let n = self.ident()?;
                        path = Expr::Member(Box::new(path), n.clone());
                        last = n;
                    }
                    let f = self.function_rest(last)?;
                    return Ok(Stmt::FuncDecl(path, f));
                }
                "class" => {
                    self.next();
                    let mut path = Expr::Ident(self.ident()?);
                    let mut last: Rc<str> = match &path {
                        Expr::Ident(n) => n.clone(),
                        _ => unreachable!(),
                    };
                    while self.eat_op(".") {
                        let n = self.ident()?;
                        path = Expr::Member(Box::new(path), n.clone());
                        last = n;
                    }
                    let c = self.class_rest(last)?;
                    return Ok(Stmt::ClassDecl(path, c));
                }
                "if" => {
                    self.next();
                    self.expect_op("(")?;
                    let c = self.expr()?;
                    self.expect_op(")")?;
                    let then = self.block_or_statement()?;
                    let els = if self.eat_kw("else") { self.block_or_statement()? } else { Vec::new() };
                    return Ok(Stmt::If(c, then, els));
                }
                "while" => {
                    self.next();
                    self.expect_op("(")?;
                    let c = self.expr()?;
                    self.expect_op(")")?;
                    let body = self.block_or_statement()?;
                    return Ok(Stmt::While(c, body));
                }
                "do" => {
                    self.next();
                    let body = self.block_or_statement()?;
                    if !self.eat_kw("while") {
                        return Err(format!("line {}: expected while after do", self.line()));
                    }
                    self.expect_op("(")?;
                    let c = self.expr()?;
                    self.expect_op(")")?;
                    self.end_statement();
                    return Ok(Stmt::DoWhile(body, c));
                }
                "for" => {
                    self.next();
                    self.expect_op("(")?;
                    let mut init = Vec::new();
                    if !self.is_op(";") {
                        if self.is_kw("local") {
                            init.push(self.statement()?);
                        } else {
                            init.push(Stmt::Expr(self.expr()?));
                            while self.eat_op(",") {
                                init.push(Stmt::Expr(self.expr()?));
                            }
                            self.expect_op(";")?;
                        }
                    } else {
                        self.next();
                    }
                    let cond = if self.is_op(";") { None } else { Some(self.expr()?) };
                    self.expect_op(";")?;
                    let mut step = Vec::new();
                    if !self.is_op(")") {
                        step.push(self.expr()?);
                        while self.eat_op(",") {
                            step.push(self.expr()?);
                        }
                    }
                    self.expect_op(")")?;
                    let body = self.block_or_statement()?;
                    return Ok(Stmt::For(init, cond, step, body));
                }
                "foreach" => {
                    self.next();
                    self.expect_op("(")?;
                    let a = self.ident()?;
                    let (k, v) = if self.eat_op(",") { (Some(a), self.ident()?) } else { (None, a) };
                    if !self.eat_kw("in") {
                        return Err(format!("line {}: expected in", self.line()));
                    }
                    let e = self.expr()?;
                    self.expect_op(")")?;
                    let body = self.block_or_statement()?;
                    return Ok(Stmt::Foreach(k, v, e, body));
                }
                "switch" => {
                    self.next();
                    self.expect_op("(")?;
                    let e = self.expr()?;
                    self.expect_op(")")?;
                    self.expect_op("{")?;
                    let mut cases: Vec<(Option<Expr>, Vec<Stmt>)> = Vec::new();
                    while !self.eat_op("}") {
                        let label = if self.eat_kw("case") {
                            let v = self.expr()?;
                            Some(v)
                        } else if self.eat_kw("default") {
                            None
                        } else {
                            return Err(format!("line {}: expected case", self.line()));
                        };
                        self.expect_op(":")?;
                        let mut body = Vec::new();
                        while !self.is_kw("case") && !self.is_kw("default") && !self.is_op("}") {
                            body.push(self.statement()?);
                        }
                        cases.push((label, body));
                    }
                    return Ok(Stmt::Switch(e, cases));
                }
                "return" => {
                    self.next();
                    let v = if self.is_op(";") || self.is_op("}") || self.newline_before() || *self.peek() == Tok::Eof {
                        None
                    } else {
                        Some(self.expr()?)
                    };
                    self.end_statement();
                    return Ok(Stmt::Return(v));
                }
                "break" => {
                    self.next();
                    self.end_statement();
                    return Ok(Stmt::Break);
                }
                "continue" => {
                    self.next();
                    self.end_statement();
                    return Ok(Stmt::Continue);
                }
                "throw" => {
                    self.next();
                    let e = self.expr()?;
                    self.end_statement();
                    return Ok(Stmt::Throw(e));
                }
                "try" => {
                    self.next();
                    let body = self.block_or_statement()?;
                    if !self.eat_kw("catch") {
                        return Err(format!("line {}: expected catch", self.line()));
                    }
                    self.expect_op("(")?;
                    let name = self.ident()?;
                    self.expect_op(")")?;
                    let handler = self.block_or_statement()?;
                    return Ok(Stmt::Try(body, name, handler));
                }
                "const" => {
                    self.next();
                    let name = self.ident()?;
                    self.expect_op("=")?;
                    let v = self.expr()?;
                    self.end_statement();
                    return Ok(Stmt::Const(name, v));
                }
                "enum" => {
                    self.next();
                    let name = self.ident()?;
                    self.expect_op("{")?;
                    let mut entries = Vec::new();
                    let mut n = 0i64;
                    while !self.eat_op("}") {
                        let k = self.ident()?;
                        if self.eat_op("=") {
                            if let Expr::Int(v) = self.expr()? {
                                n = v;
                            }
                        }
                        entries.push((Expr::Str(k), Expr::Int(n)));
                        n += 1;
                        self.eat_op(",");
                    }
                    return Ok(Stmt::Const(name, Expr::Table(entries)));
                }
                _ => {}
            }
        }
        let e = self.expr()?;
        self.end_statement();
        Ok(Stmt::Expr(e))
    }

    fn params(&mut self) -> R<(Vec<(Rc<str>, Option<Expr>)>, bool)> {
        self.expect_op("(")?;
        let mut params = Vec::new();
        let mut varargs = false;
        while !self.eat_op(")") {
            if self.eat_op("...") {
                varargs = true;
                continue;
            }
            let name = self.ident()?;
            let def = if self.eat_op("=") { Some(self.expr()?) } else { None };
            params.push((name, def));
            self.eat_op(",");
        }
        Ok((params, varargs))
    }

    fn function_rest(&mut self, name: Rc<str>) -> R<Rc<FuncDef>> {
        let (params, varargs) = self.params()?;
        let body = self.block_or_statement()?;
        Ok(Rc::new(FuncDef { name, params, varargs, body }))
    }

    fn class_rest(&mut self, name: Rc<str>) -> R<Rc<ClassDef>> {
        let base = if self.eat_kw("extends") { Some(self.postfix()?) } else { None };
        self.expect_op("{")?;
        let mut members = Vec::new();
        while !self.eat_op("}") {
            if self.eat_op(";") || self.eat_op(",") {
                continue;
            }
            let is_static = self.eat_kw("static");
            if self.is_kw("constructor") {
                self.next();
                let f = self.function_rest("constructor".into())?;
                members.push((Expr::Str("constructor".into()), Expr::Function(f), false));
            } else if self.eat_kw("function") {
                let n = self.ident()?;
                let f = self.function_rest(n.clone())?;
                members.push((Expr::Str(n), Expr::Function(f), is_static));
            } else if self.eat_op("[") {
                let k = self.expr()?;
                self.expect_op("]")?;
                self.expect_op("=")?;
                members.push((k, self.expr()?, is_static));
            } else {
                let n = self.ident()?;
                self.expect_op("=")?;
                members.push((Expr::Str(n), self.expr()?, is_static));
            }
        }
        Ok(Rc::new(ClassDef { name, base, members }))
    }

    pub fn expr(&mut self) -> R<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> R<Expr> {
        let left = self.ternary()?;
        for op in ["=", "<-", "+=", "-=", "*=", "/=", "%=", "|=", "&=", "^=", "<<=", ">>="] {
            if self.is_op(op) {
                self.next();
                let right = self.assignment()?;
                return Ok(Expr::Assign(op, Box::new(left), Box::new(right)));
            }
        }
        Ok(left)
    }

    fn ternary(&mut self) -> R<Expr> {
        let c = self.or()?;
        if self.eat_op("?") {
            let a = self.assignment()?;
            self.expect_op(":")?;
            let b = self.assignment()?;
            return Ok(Expr::Ternary(Box::new(c), Box::new(a), Box::new(b)));
        }
        Ok(c)
    }

    fn or(&mut self) -> R<Expr> {
        let mut l = self.and()?;
        while self.eat_op("||") {
            let r = self.and()?;
            l = Expr::Or(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn and(&mut self) -> R<Expr> {
        let mut l = self.in_expr()?;
        while self.eat_op("&&") {
            let r = self.in_expr()?;
            l = Expr::And(Box::new(l), Box::new(r));
        }
        Ok(l)
    }

    fn in_expr(&mut self) -> R<Expr> {
        let mut l = self.binary(0)?;
        loop {
            if self.eat_kw("in") {
                let r = self.binary(0)?;
                l = Expr::In(Box::new(l), Box::new(r));
            } else if self.eat_kw("instanceof") {
                let r = self.binary(0)?;
                l = Expr::InstanceOf(Box::new(l), Box::new(r));
            } else {
                return Ok(l);
            }
        }
    }

    fn binary(&mut self, level: usize) -> R<Expr> {
        const LEVELS: &[&[&str]] = &[&["|"], &["^"], &["&"], &["==", "!=", "<=>"], &["<", "<=", ">", ">="], &["<<", ">>"], &["+", "-"], &["*", "/", "%"]];
        if level >= LEVELS.len() {
            return self.unary();
        }
        let mut l = self.binary(level + 1)?;
        'outer: loop {
            for op in LEVELS[level] {
                if self.is_op(op) {
                    self.next();
                    let r = self.binary(level + 1)?;
                    l = Expr::Binary(op, Box::new(l), Box::new(r));
                    continue 'outer;
                }
            }
            return Ok(l);
        }
    }

    fn unary(&mut self) -> R<Expr> {
        for op in ["-", "!", "~"] {
            if self.is_op(op) {
                self.next();
                let e = self.unary()?;
                return Ok(Expr::Unary(op, Box::new(e)));
            }
        }
        if self.eat_op("++") {
            let e = self.unary()?;
            return Ok(Expr::IncDec(Box::new(e), 1, true));
        }
        if self.eat_op("--") {
            let e = self.unary()?;
            return Ok(Expr::IncDec(Box::new(e), -1, true));
        }
        if self.eat_kw("typeof") {
            return Ok(Expr::Typeof(Box::new(self.unary()?)));
        }
        if self.eat_kw("delete") {
            return Ok(Expr::Delete(Box::new(self.unary()?)));
        }
        if self.eat_kw("clone") {
            return Ok(Expr::Clone(Box::new(self.unary()?)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> R<Expr> {
        let mut e = self.primary()?;
        loop {
            if self.is_op(".") {
                self.next();
                let n = self.ident()?;
                e = Expr::Member(Box::new(e), n);
            } else if self.is_op("[") && !self.newline_before() {
                self.next();
                let i = self.expr()?;
                self.expect_op("]")?;
                e = Expr::Index(Box::new(e), Box::new(i));
            } else if self.is_op("(") && !self.newline_before() {
                self.next();
                let mut args = Vec::new();
                while !self.eat_op(")") {
                    args.push(self.expr()?);
                    self.eat_op(",");
                }
                e = Expr::Call(Box::new(e), args);
            } else if self.is_op("++") && !self.newline_before() {
                self.next();
                e = Expr::IncDec(Box::new(e), 1, false);
            } else if self.is_op("--") && !self.newline_before() {
                self.next();
                e = Expr::IncDec(Box::new(e), -1, false);
            } else {
                return Ok(e);
            }
        }
    }

    fn primary(&mut self) -> R<Expr> {
        let line = self.line();
        match self.next() {
            Tok::Int(v) => Ok(Expr::Int(v)),
            Tok::Float(v) => Ok(Expr::Float(v)),
            Tok::Str(s) => Ok(Expr::Str(s.into())),
            Tok::Op("(") => {
                let e = self.expr()?;
                self.expect_op(")")?;
                Ok(e)
            }
            Tok::Op("::") => Ok(Expr::Root(self.ident()?)),
            Tok::Op("[") => {
                let mut items = Vec::new();
                while !self.eat_op("]") {
                    items.push(self.expr()?);
                    self.eat_op(",");
                }
                Ok(Expr::Array(items))
            }
            Tok::Op("{") => self.table_rest(),
            Tok::Ident(id) => match id.as_str() {
                "null" => Ok(Expr::Null),
                "true" => Ok(Expr::Bool(true)),
                "false" => Ok(Expr::Bool(false)),
                "this" => Ok(Expr::This),
                "base" => Ok(Expr::Base),
                "function" => {
                    let name: Rc<str> = if let Tok::Ident(n) = self.peek().clone() {
                        self.next();
                        n.into()
                    } else {
                        "<lambda>".into()
                    };
                    Ok(Expr::Function(self.function_rest(name)?))
                }
                "class" => Ok(Expr::Class(self.class_rest("<class>".into())?)),
                _ => Ok(Expr::Ident(id.into())),
            },
            t => Err(format!("line {line}: unexpected {t:?}")),
        }
    }

    fn table_rest(&mut self) -> R<Expr> {
        let mut entries = Vec::new();
        while !self.eat_op("}") {
            if self.eat_op(",") || self.eat_op(";") {
                continue;
            }
            if self.eat_kw("function") {
                let n = self.ident()?;
                let f = self.function_rest(n.clone())?;
                entries.push((Expr::Str(n), Expr::Function(f)));
                continue;
            }
            if self.eat_op("[") {
                let k = self.expr()?;
                self.expect_op("]")?;
                self.expect_op("=")?;
                entries.push((k, self.expr()?));
                continue;
            }
            let key = match self.next() {
                Tok::Ident(s) => s,
                Tok::Str(s) => s,
                Tok::Int(v) => v.to_string(),
                t => return Err(format!("line {}: bad table key {t:?}", self.line())),
            };
            if !self.eat_op("=") {
                self.expect_op(":")?;
            }
            entries.push((Expr::Str(key.into()), self.expr()?));
        }
        Ok(Expr::Table(entries))
    }
}

/// Parse a whole script.
pub fn parse(src: &str) -> Result<Vec<Stmt>, String> {
    let toks = crate::lexer::lex(src)?;
    Parser::new(toks).program()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_typical_script() {
        let src = r#"
SceneTable <- {}
SceneTable["a"] <- { vcd = CreateSceneEntity("scenes/x.vcd"), postdelay = 0.1, next = "b", char = "glados" }
function GladosPlayVcd(arg, IgnoreQueue = null)
{
    if (typeof arg == "integer") { arg = SceneTableLookup[arg] }
    foreach (k, v in SceneTable) { printl(k) }
    switch (arg) { case 1: x++; break; default: y <- 2 }
    local i = 0, j
    for (local n = 0; n < 3; n++) i += n
    return i > 2 ? "a" : "b"
}
class Thing { a = 1; constructor(x) { a = x } function get() { return a } }
"#;
        let p = parse(src).unwrap();
        assert_eq!(p.len(), 4);
    }
}
