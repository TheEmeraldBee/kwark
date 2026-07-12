use std::fmt::{Debug, Display};

/// The generic token type that's returned out of the lexer
#[derive(Clone, PartialEq)]
pub enum Token {
    Int(i32),
    Float(f32),
    Str(String),
    Bool(bool),

    Ident(String),

    Let,

    For,
    In,
    If,
    Else,

    Return,
    Break,

    Fn,

    Ctrl(char),
    Op(String),
}

impl Display for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Int(n) => write!(f, "<int>`{}`", n),
            Self::Float(n) => write!(f, "<float>`{}`", n),
            Self::Str(s) => write!(f, "<str>`{}`", s.escape_default()),
            Self::Bool(b) => write!(f, "<bool>`{}`", b),

            Self::Ident(i) => write!(f, "<ident>`{}`", i),

            Self::Let => write!(f, "<let>"),

            Self::For => write!(f, "<for>"),
            Self::In => write!(f, "<in>"),
            Self::If => write!(f, "<if>"),
            Self::Else => write!(f, "<else>"),

            Self::Return => write!(f, "<return>"),
            Self::Break => write!(f, "<break>"),

            Self::Fn => write!(f, "<fn>"),

            Self::Ctrl(c) => write!(f, "<ctrl>`{}`", c),
            Self::Op(o) => write!(f, "<op>`{}`", o),
        }
    }
}

impl Debug for Token {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self)
    }
}
