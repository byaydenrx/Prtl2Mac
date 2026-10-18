//! Squirrel syntax tree.
use std::rc::Rc;

#[derive(Debug)]
pub enum Expr {
    Null,
    Bool(bool),
    Int(i64),
    Float(f64),
    Str(Rc<str>),
    /// A name: a local, or looked up in `this` then the root table.
    Ident(Rc<str>),
    /// `::name`: the root table.
    Root(Rc<str>),
    This,
    Base,
    /// `a.b`
    Member(Box<Expr>, Rc<str>),
    /// `a[b]`
    Index(Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    /// `target op= value` (`=`, `+=`...), or `<-` for a new slot.
    Assign(&'static str, Box<Expr>, Box<Expr>),
    /// `++x` / `x++` (delta, prefix).
    IncDec(Box<Expr>, i64, bool),
    Table(Vec<(Expr, Expr)>),
    Array(Vec<Expr>),
    Function(Rc<FuncDef>),
    Class(Rc<ClassDef>),
    Typeof(Box<Expr>),
    Delete(Box<Expr>),
    Clone(Box<Expr>),
    In(Box<Expr>, Box<Expr>),
    InstanceOf(Box<Expr>, Box<Expr>),
}

#[derive(Debug)]
pub struct FuncDef {
    pub name: Rc<str>,
    pub params: Vec<(Rc<str>, Option<Expr>)>,
    pub varargs: bool,
    pub body: Vec<Stmt>,
}

#[derive(Debug)]
pub struct ClassDef {
    pub name: Rc<str>,
    pub base: Option<Expr>,
    /// (name, value, static)
    pub members: Vec<(Expr, Expr, bool)>,
}

#[derive(Debug)]
pub enum Stmt {
    Expr(Expr),
    Local(Vec<(Rc<str>, Option<Expr>)>),
    /// `function a.b.c()` / `function ::f()` / `function f()`: assign into a
    /// path (new slot).
    FuncDecl(Expr, Rc<FuncDef>),
    ClassDecl(Expr, Rc<ClassDef>),
    If(Expr, Vec<Stmt>, Vec<Stmt>),
    While(Expr, Vec<Stmt>),
    DoWhile(Vec<Stmt>, Expr),
    For(Vec<Stmt>, Option<Expr>, Vec<Expr>, Vec<Stmt>),
    Foreach(Option<Rc<str>>, Rc<str>, Expr, Vec<Stmt>),
    Switch(Expr, Vec<(Option<Expr>, Vec<Stmt>)>),
    Return(Option<Expr>),
    Break,
    Continue,
    Throw(Expr),
    Try(Vec<Stmt>, Rc<str>, Vec<Stmt>),
    Block(Vec<Stmt>),
    /// `const X = 1` / `enum`: a root constant.
    Const(Rc<str>, Expr),
}
