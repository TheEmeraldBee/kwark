type BoxedExpr = Box<Expr>;

pub enum Expr {
    Let {
        name: String,
        body: BoxedExpr,
    },

    Assign {
        name: String,
        body: BoxedExpr,
    },

    Local {
        name: String,
    },

    UnaryOp {
        op: String,
        body: BoxedExpr,
    },

    BinOp {
        left: BoxedExpr,
        op: String,
        right: BoxedExpr,
    },

    If {
        cond: BoxedExpr,
        then: BoxedExpr,
        else_: Option<BoxedExpr>,
    },
    For {
        iterator: BoxedExpr,
        body: BoxedExpr,
    },

    Then {
        first: BoxedExpr,
        next: BoxedExpr,
    },
}
