use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    start: usize,
    end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn start(&self) -> usize {
        self.start
    }

    pub fn end(&self) -> usize {
        self.end
    }
}

impl From<Span> for std::ops::Range<usize> {
    fn from(span: Span) -> Self {
        span.start..span.end
    }
}

#[derive(Clone, PartialEq, Eq, Hash)]
pub enum TokenKind {
    Fn,
    Print,
    Return,
    Identifier(String),
    Number(String),
    StringLiteral(String),
    Colon,
    Newline,
    Indent,
    Dedent,
    LParen,
    RParen,
    Comma,
    Equals,
    Plus,
    Minus,
    Star,
    Slash,
    Eof,
}

impl TokenKind {
    pub fn name(&self) -> &'static str {
        match self {
            TokenKind::Fn => "fn",
            TokenKind::Print => "print",
            TokenKind::Return => "return",
            TokenKind::Identifier(_) => "identifier",
            TokenKind::Number(_) => "number",
            TokenKind::StringLiteral(_) => "string",
            TokenKind::Colon => ":",
            TokenKind::Newline => "newline",
            TokenKind::Indent => "indent",
            TokenKind::Dedent => "dedent",
            TokenKind::LParen => "(",
            TokenKind::RParen => ")",
            TokenKind::Comma => ",",
            TokenKind::Equals => "=",
            TokenKind::Plus => "+",
            TokenKind::Minus => "-",
            TokenKind::Star => "*",
            TokenKind::Slash => "/",
            TokenKind::Eof => "eof",
        }
    }
}

impl fmt::Debug for TokenKind {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TokenKind::Identifier(name) => write!(f, "Identifier({name})"),
            TokenKind::Number(number) => write!(f, "Number({number})"),
            TokenKind::StringLiteral(value) => write!(f, "StringLiteral(\"{value}\")"),
            kind => f.write_str(kind.name()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Token {
    pub kind: TokenKind,
    pub span: Span,
}

impl Token {
    pub fn new(kind: TokenKind, span: Span) -> Self {
        Self { kind, span }
    }
}
