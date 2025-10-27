#[derive(Debug, Clone)]
pub struct Program {
    pub functions: Vec<Function>,
}

impl Program {
    pub fn new(functions: Vec<Function>) -> Self {
        Self { functions }
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub body: Vec<Statement>,
}

impl Function {
    pub fn new(name: impl Into<String>, body: Vec<Statement>) -> Self {
        Self {
            name: name.into(),
            body,
        }
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    Print(Expr),
    Assignment { name: String, expr: Expr },
    Return(Option<Expr>),
}

#[derive(Debug, Clone)]
pub enum Expr {
    Literal(Literal),
    Identifier(String),
    Binary {
        left: Box<Expr>,
        op: BinaryOp,
        right: Box<Expr>,
    },
    Call {
        callee: Box<Expr>,
        args: Vec<Expr>,
    },
}

#[derive(Debug, Clone, Copy)]
pub enum BinaryOp {
    Add,
    Sub,
    Mul,
    Div,
}

#[derive(Debug, Clone)]
pub enum Literal {
    String(String),
    Number(f64),
}
