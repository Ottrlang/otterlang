#[derive(Debug, Clone)]
pub struct Program {
    pub statements: Vec<Statement>,
}

impl Program {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self { statements }
    }

    /// Get all function definitions in the program
    pub fn functions(&self) -> impl Iterator<Item = &Function> {
        self.statements.iter().filter_map(|stmt| {
            if let Statement::Function(func) = stmt {
                Some(func)
            } else {
                None
            }
        })
    }

    /// Count the total number of statements recursively
    pub fn statement_count(&self) -> usize {
        self.statements.iter().map(|s| s.recursive_count()).sum()
    }
}

#[derive(Debug, Clone)]
pub struct Function {
    pub name: String,
    pub params: Vec<Param>,
    pub ret_ty: Option<Type>,
    pub body: Block,
}

impl Function {
    pub fn new(
        name: impl Into<String>,
        params: Vec<Param>,
        ret_ty: Option<Type>,
        body: Block,
    ) -> Self {
        Self {
            name: name.into(),
            params,
            ret_ty,
            body,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Simple(String),
    Generic {
        base: String,
        args: Vec<Type>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Param {
    pub name: String,
    pub ty: Option<Type>,
}

impl Param {
    pub fn new(name: impl Into<String>, ty: Option<Type>) -> Self {
        Self {
            name: name.into(),
            ty,
        }
    }
}

#[derive(Debug, Clone)]
pub struct Block {
    pub statements: Vec<Statement>,
}

impl Block {
    pub fn new(statements: Vec<Statement>) -> Self {
        Self { statements }
    }
}

#[derive(Debug, Clone)]
pub enum Statement {
    // Variable declarations and assignments
    Let {
        name: String,
        expr: Expr,
    },
    Assignment {
        name: String,
        expr: Expr,
    },

    // Control flow
    If {
        cond: Box<Expr>,
        then_block: Block,
        elif_blocks: Vec<(Expr, Block)>,
        else_block: Option<Block>,
    },
    For {
        var: String,
        iterable: Expr,
        body: Block,
    },
    While {
        cond: Expr,
        body: Block,
    },
    Break,
    Continue,
    Return(Option<Expr>),

    // Function definitions
    Function(Function),

    // Expressions as statements
    Expr(Expr),

    // Module imports
    Use {
        module: String,
        alias: Option<String>,
    },

    // Blocks (for grouping)
    Block(Block),
}

impl Statement {
    /// Recursively count statements
    pub fn recursive_count(&self) -> usize {
        match self {
            Statement::Let { .. }
            | Statement::Assignment { .. }
            | Statement::Break
            | Statement::Continue
            | Statement::Return(_)
            | Statement::Expr(_)
            | Statement::Use { .. } => 1,

            Statement::If {
                then_block,
                elif_blocks,
                else_block,
                ..
            } => {
                let mut count = 1;
                count += then_block.recursive_count();
                for (_, block) in elif_blocks {
                    count += block.recursive_count();
                }
                if let Some(block) = else_block {
                    count += block.recursive_count();
                }
                count
            }
            Statement::For { body, .. } | Statement::While { body, .. } => {
                1 + body.recursive_count()
            }
            Statement::Function(func) => 1 + func.body.recursive_count(),
            Statement::Block(block) => block.recursive_count(),
        }
    }

    /// Check if statement is pure (has no side effects)
    pub fn is_pure(&self) -> bool {
        matches!(
            self,
            Statement::Let { .. } | Statement::Break | Statement::Continue
        )
    }
}

impl Block {
    /// Recursively count statements
    pub fn recursive_count(&self) -> usize {
        self.statements.iter().map(|s| s.recursive_count()).sum()
    }

    /// Check if block is empty
    pub fn is_empty(&self) -> bool {
        self.statements.is_empty()
    }
}

#[derive(Debug, Clone)]
pub enum Expr {
    // Literals
    Literal(Literal),

    // Variables and access
    Identifier(String),
    Member {
        object: Box<Expr>,
        field: String,
    },

    // Function calls
    Call {
        func: Box<Expr>,
        args: Vec<Expr>,
    },

    // Binary operations
    Binary {
        op: BinaryOp,
        left: Box<Expr>,
        right: Box<Expr>,
    },

    // Unary operations
    Unary {
        op: UnaryOp,
        expr: Box<Expr>,
    },

    // Control flow expressions
    If {
        cond: Box<Expr>,
        then_branch: Box<Expr>,
        else_branch: Option<Box<Expr>>,
    },

    // Range expressions
    Range {
        start: Box<Expr>,
        end: Box<Expr>,
    },

    // String interpolation
    FString {
        parts: Vec<FStringPart>,
    },

    // Lambda expressions
    Lambda {
        params: Vec<Param>,
        ret_ty: Option<Type>,
        body: Block,
    },

    // Async operations
    Await(Box<Expr>),
    Spawn(Box<Expr>),
}

#[derive(Debug, Clone)]
pub enum FStringPart {
    Text(String),
    Expr(Box<Expr>),
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum BinaryOp {
    // Arithmetic
    Add,
    Sub,
    Mul,
    Div,
    Mod,

    // Comparison
    Eq,
    Ne,
    Lt,
    Gt,
    LtEq,
    GtEq,

    // Logical
    And,
    Or,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum UnaryOp {
    Neg,
    Not,
}

#[derive(Debug, Clone)]
pub enum Literal {
    String(String),
    Number(f64),
    Bool(bool),
}

impl PartialEq for Literal {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Literal::String(a), Literal::String(b)) => a == b,
            (Literal::Bool(a), Literal::Bool(b)) => a == b,
            (Literal::Number(a), Literal::Number(b)) => a.to_bits() == b.to_bits(), // Compare f64 by bits
            _ => false,
        }
    }
}

impl Eq for Literal {}

use std::hash::{Hash, Hasher};

impl Hash for Literal {
    fn hash<H: Hasher>(&self, state: &mut H) {
        match self {
            Literal::String(s) => {
                0u8.hash(state);
                s.hash(state);
            }
            Literal::Number(n) => {
                1u8.hash(state);
                n.to_bits().hash(state);
            }
            Literal::Bool(b) => {
                2u8.hash(state);
                b.hash(state);
            }
        }
    }
}
