pub mod error;

pub mod lex;
pub mod token;

pub mod expr;
pub mod parse;

pub mod op_registry;

pub mod value;

pub mod spanned;

pub mod engine;
pub mod scope;

pub use kaon_macros::module;

pub mod prelude {
    pub use crate::error::Error as KaonError;

    pub use crate::lex::Lexer;
    pub use crate::op_registry::OpRegistry;
    pub use crate::parse::Parser;
    pub use crate::token::Token;

    pub use crate::value::*;

    pub use crate::engine::*;
    pub use crate::scope::Scope;

    pub use crate::spanned::Spanned;

    pub use kaon_macros::module;
}
