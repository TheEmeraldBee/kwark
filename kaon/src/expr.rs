use crate::{error::Error, spanned::Spanned, value::Value};

pub type SpannedExpr = Spanned<Box<Expr>>;

#[derive(Clone, Debug, PartialEq)]
pub enum Expr {
    Null,
    Literal(Value),

    Error(Spanned<Error>),

    List(Vec<SpannedExpr>),

    Let {
        name: Spanned<String>,
        body: SpannedExpr,
    },

    Assign {
        name: Spanned<String>,
        body: SpannedExpr,
    },

    Local {
        name: Spanned<String>,
    },

    UnaryOp {
        op: String,
        body: SpannedExpr,
    },

    BinOp {
        left: SpannedExpr,
        op: String,
        right: SpannedExpr,
    },

    If {
        cond: SpannedExpr,
        then: SpannedExpr,
        else_: Option<SpannedExpr>,
    },
    For {
        name: Option<Spanned<String>>,
        iterator: SpannedExpr,
        body: SpannedExpr,
    },

    Func {
        args: Vec<Spanned<String>>,
        body: SpannedExpr,
    },

    Then {
        first: SpannedExpr,
        next: SpannedExpr,
    },

    Block {
        body: SpannedExpr,
    },

    Call {
        body: SpannedExpr,
        args: Vec<SpannedExpr>,
    },
}

impl Expr {
    /// Reports whether a trailing `;` is optional after this expr in a statement sequence
    pub fn ends_in_block(&self) -> bool {
        matches!(self, Expr::If { .. } | Expr::For { .. } | Expr::Block { .. })
    }
}
