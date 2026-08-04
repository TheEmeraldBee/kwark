use crate::{
    error::Error,
    expr::{Expr, SpannedExpr},
    op_registry::OpRegistry,
    spanned::Spanned,
    token::Token,
    value::Value,
};

pub struct Parser<'src> {
    tokens: &'src [Spanned<Token>],

    start: bool,
    cursor: usize,

    checkpoints: Vec<usize>,

    registry: &'src OpRegistry,
}

pub type ExprResult = Result<SpannedExpr, Spanned<Error>>;

impl<'src> Parser<'src> {
    pub fn parse(registry: &OpRegistry, tokens: &'src [Spanned<Token>]) -> ExprResult {
        let (tree, diagnostics) = Self::parse_recovering(registry, tokens);

        match diagnostics.into_iter().next() {
            Some(first) => Err(first),
            None => Ok(tree),
        }
    }

    pub fn parse_recovering(
        registry: &OpRegistry,
        tokens: &'src [Spanned<Token>],
    ) -> (SpannedExpr, Vec<Spanned<Error>>) {
        if tokens.is_empty() {
            return (Spanned::new(0, 0, Box::new(Expr::Null)), vec![]);
        }

        let mut parser = Parser {
            tokens,

            start: true,
            cursor: 0,

            checkpoints: vec![],

            registry,
        };

        parser.advance();

        parser.exprs_recovering()
    }

    fn advance(&mut self) -> bool {
        if self.start {
            self.start = false;
            return true;
        }

        if self.cursor + 1 >= self.tokens.len() {
            return false;
        }

        self.cursor += 1;

        true
    }

    fn expect_advance(&mut self, label: impl ToString) -> Result<(), Spanned<Error>> {
        self.checkpoint();

        if !self.advance() {
            self.restore_checkpoint();

            return Err(self.create_checkpoint(Error::ExpectedFoundEOF(label.to_string())));
        }

        self.remove_checkpoint();

        Ok(())
    }

    fn skip(&mut self, tok: &Token, after_label: impl ToString) -> Result<(), Spanned<Error>> {
        self.expect_advance(tok)?;

        self.expect(tok)?;

        self.expect_advance(after_label)?;

        Ok(())
    }

    fn expect(&self, tok: &Token) -> Result<(), Spanned<Error>> {
        if self.get() != tok {
            return Err(
                self.create_checkpoint(Error::Expected(tok.to_string(), self.get().to_string()))
            );
        }

        Ok(())
    }

    fn back(&mut self) {
        self.cursor -= 1;
    }

    fn checkpoint(&mut self) {
        self.checkpoints.push(self.cursor)
    }

    fn remove_checkpoint(&mut self) {
        self.checkpoints.pop();
    }

    fn restore_checkpoint(&mut self) {
        if let Some(cursor) = self.checkpoints.pop() {
            self.cursor = cursor;
        }
    }

    fn create<T>(&self, value: T) -> Spanned<T> {
        Spanned::new(
            self.tokens[self.cursor].start,
            self.tokens[self.cursor].end,
            value,
        )
    }

    fn create_checkpoint<T>(&self, value: T) -> Spanned<T> {
        Spanned::new(
            self.tokens[*self.checkpoints.last().unwrap_or(&self.cursor)].start,
            self.tokens[self.cursor].end,
            value,
        )
    }

    fn expected(&self, label: impl Into<String>, found: impl ToString) -> Spanned<Error> {
        self.create_checkpoint(Error::Expected(label.into(), found.to_string()))
    }

    fn get(&self) -> &Token {
        &self.tokens[self.cursor]
    }

    fn seperated<T>(
        separator: &Token,
        end: &Token,
        allow_trailing: bool,

        func: impl Fn(&mut Self) -> Result<T, Spanned<Error>>,
    ) -> impl Fn(&mut Self) -> Result<(Vec<T>, bool), Spanned<Error>> {
        move |parser| {
            if parser.get() == end {
                parser.back();
                return Ok((vec![], false));
            }

            let mut out = vec![func(parser)?];
            let mut trailing = false;

            loop {
                if !parser.advance() {
                    break;
                }

                if parser.get() != separator {
                    parser.back();
                    break;
                }

                if !parser.advance() {
                    trailing = true;
                    break;
                }

                if allow_trailing && parser.get() == end {
                    parser.back();
                    trailing = true;
                    break;
                }

                out.push(func(parser)?);
                trailing = false;
            }

            Ok((out, trailing))
        }
    }

    fn wrapped<T>(
        &mut self,
        start: &Token,
        end: &Token,

        label: impl Into<String>,
        func: impl Fn(&mut Self) -> Result<T, Spanned<Error>>,
    ) -> Result<T, Spanned<Error>> {
        self.checkpoint();

        if self.get() != start {
            return Err(
                self.create_checkpoint(Error::Expected(start.to_string(), self.get().to_string()))
            );
        }

        if !self.advance() {
            return Err(self.create_checkpoint(Error::ExpectedFoundEOF(label.into())));
        }

        let res = func(self)?;

        if !self.advance() {
            return Err(self.create_checkpoint(Error::ExpectedFoundEOF(end.to_string())));
        }

        if self.get() != end {
            return Err(
                self.create_checkpoint(Error::Expected(end.to_string(), self.get().to_string()))
            );
        }

        Ok(res)
    }

    fn exprs_recovering(&mut self) -> (SpannedExpr, Vec<Spanned<Error>>) {
        let mut diagnostics = vec![];
        let mut items = vec![];
        let mut trailing = false;

        loop {
            let stmt_start = self.cursor;
            let mut optional_semi = false;

            match self.expr() {
                Ok(item) => {
                    optional_semi = item.ends_in_block();
                    items.push(item);
                }
                Err(err) => {
                    self.cursor = stmt_start;
                    let end = self.skip_to_semicolon();

                    items.push(Spanned::new(
                        self.tokens[stmt_start].start,
                        end,
                        Box::new(Expr::Error(err.clone())),
                    ));
                    diagnostics.push(err);
                }
            }

            if !self.advance() {
                break;
            }

            if self.get() != &Token::Ctrl(';') {
                if optional_semi {
                    continue;
                }

                diagnostics.push(self.create(Error::Expected(
                    Token::Ctrl(';').to_string(),
                    self.get().to_string(),
                )));
                continue;
            }

            if !self.advance() {
                trailing = true;
                break;
            }
        }

        if trailing {
            items.push(self.create(Box::new(Expr::Literal(Value::Null))));
        }

        let mut iter = items.into_iter().rev();
        let mut acc = iter
            .next()
            .expect("exprs_recovering always yields at least one item");

        for item in iter {
            acc = Spanned::new(
                item.start,
                acc.end,
                Box::new(Expr::Then {
                    first: item,
                    next: acc,
                }),
            );
        }

        (acc, diagnostics)
    }

    fn skip_to_semicolon(&mut self) -> usize {
        loop {
            if self.cursor + 1 >= self.tokens.len() {
                break;
            }

            if *self.tokens[self.cursor + 1] == Token::Ctrl(';') {
                break;
            }

            self.cursor += 1;
        }

        self.tokens[self.cursor].end
    }

    fn expr(&mut self) -> ExprResult {
        self.checkpoint();

        let out = (|| -> ExprResult {
            Ok(match self.get() {
                Token::Let => {
                    self.expect_advance("name")?;

                    let Token::Ident(name) = self.get().clone() else {
                        return Err(self.expected("name", self.get()));
                    };
                    let name = self.create(name);

                    self.skip(&Token::Op("=".to_string()), "expr")?;

                    let body = self.expr()?;

                    self.create_checkpoint(Box::new(Expr::Let { name, body }))
                }

                Token::If => {
                    self.expect_advance("condition")?;
                    let cond = self.binary(0)?;

                    self.expect_advance("block")?;
                    let then = self.block()?;

                    self.checkpoint();
                    let else_ = if self.advance() && self.get() == &Token::Else {
                        self.remove_checkpoint();
                        self.expect_advance("if or block")?;

                        Some(if self.get() == &Token::If {
                            self.expr()?
                        } else {
                            self.block()?
                        })
                    } else {
                        self.restore_checkpoint();
                        None
                    };

                    self.create_checkpoint(Box::new(Expr::If { cond, then, else_ }))
                }

                Token::For => {
                    self.expect_advance("iterator or loop variable")?;

                    self.checkpoint();
                    let name = match self.get().clone() {
                        Token::Ident(n) => {
                            let name_span = self.create(n);

                            if self.advance() && self.get() == &Token::In {
                                self.remove_checkpoint();
                                self.expect_advance("expr")?;

                                Some(name_span)
                            } else {
                                self.restore_checkpoint();
                                None
                            }
                        }
                        _ => {
                            self.remove_checkpoint();
                            None
                        }
                    };

                    let iterator = self.expr()?;

                    self.expect_advance("block")?;
                    let body = self.block()?;

                    self.create_checkpoint(Box::new(Expr::For {
                        name,
                        iterator,
                        body,
                    }))
                }

                _ => self.binary(0)?,
            })
        })();

        self.remove_checkpoint();

        out
    }

    fn binary(&mut self, min_precedence: u16) -> ExprResult {
        let mut lhs = self.unary()?;

        loop {
            if !self.advance() {
                break;
            }

            let Token::Op(op) = self.get() else {
                self.back();
                break;
            };

            if op == "=" {
                if min_precedence > 0 {
                    self.back();
                    break;
                }

                let Expr::Local { name } = &**lhs else {
                    return Err(self.expected("assignable", self.get()));
                };
                let name = name.clone();

                self.expect_advance("expr")?;
                let body = self.binary(0)?;

                lhs = Spanned::new(lhs.start, body.end, Box::new(Expr::Assign { name, body }));

                continue;
            }

            let Some(entry) = self.registry.binary_ops.get(op) else {
                self.back();
                break;
            };

            let prec = entry.1;

            if prec < min_precedence {
                self.back();
                break;
            }

            let op = op.clone();

            self.expect_advance("expr")?;
            let rhs = self.binary(prec + 1)?;

            lhs = Spanned::new(
                lhs.start,
                rhs.end,
                Box::new(Expr::BinOp {
                    left: lhs,
                    op,
                    right: rhs,
                }),
            );
        }

        Ok(lhs)
    }

    fn unary(&mut self) -> ExprResult {
        if let Token::Op(op) = self.get() {
            let op_exists = self.registry.unary_ops.contains_key(op);

            if !op_exists {
                return Err(self.expected("valid operator", op));
            }

            let op = op.clone();

            self.expect_advance("expr")?;
            let body = self.unary()?;

            return Ok(self.create_checkpoint(Box::new(Expr::UnaryOp { op, body })));
        }

        self.postfix()
    }

    fn postfix(&mut self) -> ExprResult {
        let mut expr = self.atom()?;

        loop {
            self.checkpoint();

            if !self.advance() || self.get() != &Token::Ctrl('(') {
                self.restore_checkpoint();
                break;
            }

            self.remove_checkpoint();

            let (args, _) = self.list(
                &Token::Ctrl('('),
                &Token::Ctrl(')'),
                &Token::Ctrl(','),
                Self::expr,
                "argument",
            )?;

            expr = Spanned::new(
                expr.start,
                self.tokens[self.cursor].end,
                Box::new(Expr::Call { body: expr, args }),
            );
        }

        Ok(expr)
    }

    fn atom(&mut self) -> ExprResult {
        Ok(match self.get() {
            Token::Int(i) => self.create(Box::new(Expr::Literal(Value::Int(*i)))),
            Token::Float(f) => self.create(Box::new(Expr::Literal(Value::Float(*f)))),
            Token::Bool(b) => self.create(Box::new(Expr::Literal(Value::Bool(*b)))),
            Token::Str(s) => self.create(Box::new(Expr::Literal(Value::Str(s.clone())))),
            Token::Null => self.create(Box::new(Expr::Literal(Value::Null))),

            Token::Ident(text) => {
                let name = self.create(text.clone());
                self.create(Box::new(Expr::Local { name }))
            }

            Token::Ctrl('(') => {
                self.wrapped(&Token::Ctrl('('), &Token::Ctrl(')'), "paren", Self::expr)?
            }
            Token::Ctrl('{') => self.block()?,
            Token::Ctrl('[') => {
                let expr = Box::new(Expr::List(
                    self.list(
                        &Token::Ctrl('['),
                        &Token::Ctrl(']'),
                        &Token::Ctrl(','),
                        Self::expr,
                        "expr",
                    )?
                    .0,
                ));

                self.create_checkpoint(expr)
            }

            Token::Fn => {
                self.expect_advance("args")?;
                let (args, _) = self.list(
                    &Token::Ctrl('('),
                    &Token::Ctrl(')'),
                    &Token::Ctrl(','),
                    |p| {
                        let Token::Ident(t) = p.get().clone() else {
                            return Err(p.expected("ident", p.get()));
                        };

                        Ok(p.create(t))
                    },
                    "argument".to_string(),
                )?;

                self.expect_advance("block")?;

                let body = self.block()?;

                self.create_checkpoint(Box::new(Expr::Func { args, body }))
            }

            tok => return Err(self.create(Error::Expected("atom".to_string(), tok.to_string()))),
        })
    }

    fn list<T>(
        &mut self,
        left: &Token,
        right: &Token,
        separator: &Token,
        each: impl Fn(&mut Self) -> Result<Spanned<T>, Spanned<Error>>,
        item_msg: impl Into<String>,
    ) -> Result<(Vec<Spanned<T>>, bool), Spanned<Error>> {
        self.wrapped(
            left,
            right,
            item_msg,
            Self::seperated(separator, right, true, each),
        )
    }

    fn block_items(&mut self) -> Result<(Vec<SpannedExpr>, bool), Spanned<Error>> {
        if self.get() == &Token::Ctrl('}') {
            self.back();
            return Ok((vec![], false));
        }

        let mut out = vec![];
        let mut trailing;

        loop {
            let item = self.expr()?;
            let optional_semi = item.ends_in_block();
            out.push(item);
            trailing = false;

            if !self.advance() {
                break;
            }

            if self.get() == &Token::Ctrl(';') {
                self.checkpoint();

                if !self.advance() {
                    self.remove_checkpoint();
                    trailing = true;
                    break;
                }

                if self.get() == &Token::Ctrl('}') {
                    self.restore_checkpoint();
                    trailing = true;
                    break;
                }

                self.remove_checkpoint();
                continue;
            }

            if self.get() == &Token::Ctrl('}') {
                self.back();
                break;
            }

            if optional_semi {
                continue;
            }

            return Err(self.expected(Token::Ctrl(';').to_string(), self.get().to_string()));
        }

        Ok((out, trailing))
    }

    fn block(&mut self) -> ExprResult {
        let (mut res, trailing) = self.wrapped(
            &Token::Ctrl('{'),
            &Token::Ctrl('}'),
            "expr",
            Self::block_items,
        )?;

        if trailing {
            res.push(self.create(Box::new(Expr::Null)));
        }

        let res = res
            .into_iter()
            .reduce(|a, b| Spanned::new(a.start, b.end, Box::new(Expr::Then { first: a, next: b })))
            .unwrap_or(self.create(Box::new(Expr::Null)));

        Ok(Spanned::new(
            res.start,
            res.end,
            Box::new(Expr::Block { body: res }),
        ))
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::lex::Lexer;

    fn registry() -> OpRegistry {
        let mut binary_ops = HashMap::new();
        binary_ops.insert("==".to_string(), ("eq".to_string(), 0));
        binary_ops.insert("+".to_string(), ("add".to_string(), 1));
        binary_ops.insert("-".to_string(), ("sub".to_string(), 1));
        binary_ops.insert("*".to_string(), ("mul".to_string(), 2));
        binary_ops.insert("/".to_string(), ("div".to_string(), 2));

        let mut unary_ops = HashMap::new();
        unary_ops.insert("-".to_string(), "neg".to_string());
        unary_ops.insert("!".to_string(), "not".to_string());

        OpRegistry {
            binary_ops,
            unary_ops,
        }
    }

    fn parse(src: &str) -> SpannedExpr {
        let registry = registry();
        let tokens = Lexer::lex(src, &registry).unwrap();
        Parser::parse(&registry, &tokens).unwrap()
    }

    // Rebuilds an expr tree with every span zeroed
    fn zero(e: SpannedExpr) -> SpannedExpr {
        let expr = match *e.into_inner() {
            Expr::Null => Expr::Null,
            Expr::Literal(v) => Expr::Literal(v),

            Expr::Error(err) => Expr::Error(Spanned::new(0, 0, err.into_inner())),

            Expr::List(items) => Expr::List(items.into_iter().map(zero).collect()),

            Expr::Let { name, body } => Expr::Let {
                name: Spanned::new(0, 0, name.into_inner()),
                body: zero(body),
            },
            Expr::Assign { name, body } => Expr::Assign {
                name: Spanned::new(0, 0, name.into_inner()),
                body: zero(body),
            },
            Expr::Local { name } => Expr::Local {
                name: Spanned::new(0, 0, name.into_inner()),
            },

            Expr::UnaryOp { op, body } => Expr::UnaryOp {
                op,
                body: zero(body),
            },
            Expr::BinOp { left, op, right } => Expr::BinOp {
                left: zero(left),
                op,
                right: zero(right),
            },

            Expr::If { cond, then, else_ } => Expr::If {
                cond: zero(cond),
                then: zero(then),
                else_: else_.map(zero),
            },
            Expr::For {
                name,
                iterator,
                body,
            } => Expr::For {
                name: name.map(|n| Spanned::new(0, 0, n.into_inner())),
                iterator: zero(iterator),
                body: zero(body),
            },

            Expr::Func { args, body } => Expr::Func {
                args: args
                    .into_iter()
                    .map(|a| Spanned::new(0, 0, a.into_inner()))
                    .collect(),
                body: zero(body),
            },

            Expr::Then { first, next } => Expr::Then {
                first: zero(first),
                next: zero(next),
            },
            Expr::Block { body } => Expr::Block { body: zero(body) },

            Expr::Call { body, args } => Expr::Call {
                body: zero(body),
                args: args.into_iter().map(zero).collect(),
            },
        };

        Spanned::new(0, 0, Box::new(expr))
    }

    fn sp<T>(value: T) -> Spanned<T> {
        Spanned::new(0, 0, value)
    }

    fn assert_parses_to(src: &str, expected: Expr) {
        assert_eq!(zero(parse(src)), sp(Box::new(expected)));
    }

    #[test]
    fn test_parse_int_literal() {
        assert_parses_to("1", Expr::Literal(Value::Int(1)));
    }

    #[test]
    fn test_parse_float_literal() {
        assert_parses_to("1.5", Expr::Literal(Value::Float(1.5)));
    }

    #[test]
    fn test_parse_bool_literal() {
        assert_parses_to("true", Expr::Literal(Value::Bool(true)));
    }

    #[test]
    fn test_parse_str_literal() {
        assert_parses_to(r#""hi""#, Expr::Literal(Value::Str("hi".to_string())));
    }

    #[test]
    fn test_parse_local() {
        assert_parses_to(
            "foo",
            Expr::Local {
                name: sp("foo".to_string()),
            },
        );
    }

    #[test]
    fn test_parse_paren() {
        assert_parses_to("(1)", Expr::Literal(Value::Int(1)));
    }

    #[test]
    fn test_parse_list() {
        assert_parses_to(
            "[1, 2, 3]",
            Expr::List(vec![
                sp(Box::new(Expr::Literal(Value::Int(1)))),
                sp(Box::new(Expr::Literal(Value::Int(2)))),
                sp(Box::new(Expr::Literal(Value::Int(3)))),
            ]),
        );
    }

    #[test]
    fn test_parse_unary() {
        assert_parses_to(
            "-1",
            Expr::UnaryOp {
                op: "-".to_string(),
                body: sp(Box::new(Expr::Literal(Value::Int(1)))),
            },
        );
    }

    #[test]
    fn test_parse_unary_stacked() {
        assert_parses_to(
            "!!true",
            Expr::UnaryOp {
                op: "!".to_string(),
                body: sp(Box::new(Expr::UnaryOp {
                    op: "!".to_string(),
                    body: sp(Box::new(Expr::Literal(Value::Bool(true)))),
                })),
            },
        );
    }

    #[test]
    fn test_parse_binary_simple() {
        assert_parses_to(
            "1 + 2",
            Expr::BinOp {
                left: sp(Box::new(Expr::Literal(Value::Int(1)))),
                op: "+".to_string(),
                right: sp(Box::new(Expr::Literal(Value::Int(2)))),
            },
        );
    }

    #[test]
    fn test_parse_binary_precedence() {
        // 1 + 2 * 3  ==  1 + (2 * 3)
        assert_parses_to(
            "1 + 2 * 3",
            Expr::BinOp {
                left: sp(Box::new(Expr::Literal(Value::Int(1)))),
                op: "+".to_string(),
                right: sp(Box::new(Expr::BinOp {
                    left: sp(Box::new(Expr::Literal(Value::Int(2)))),
                    op: "*".to_string(),
                    right: sp(Box::new(Expr::Literal(Value::Int(3)))),
                })),
            },
        );
    }

    #[test]
    fn test_parse_binary_left_associative() {
        // 1 - 2 - 3  ==  (1 - 2) - 3
        assert_parses_to(
            "1 - 2 - 3",
            Expr::BinOp {
                left: sp(Box::new(Expr::BinOp {
                    left: sp(Box::new(Expr::Literal(Value::Int(1)))),
                    op: "-".to_string(),
                    right: sp(Box::new(Expr::Literal(Value::Int(2)))),
                })),
                op: "-".to_string(),
                right: sp(Box::new(Expr::Literal(Value::Int(3)))),
            },
        );
    }

    #[test]
    fn test_parse_let() {
        assert_parses_to(
            "let x = 1",
            Expr::Let {
                name: sp("x".to_string()),
                body: sp(Box::new(Expr::Literal(Value::Int(1)))),
            },
        );
    }

    #[test]
    fn test_parse_assign() {
        assert_parses_to(
            "x = 1",
            Expr::Assign {
                name: sp("x".to_string()),
                body: sp(Box::new(Expr::Literal(Value::Int(1)))),
            },
        );
    }

    #[test]
    fn test_parse_assign_right_associative() {
        // x = y = 1  ==  x = (y = 1)
        assert_parses_to(
            "x = y = 1",
            Expr::Assign {
                name: sp("x".to_string()),
                body: sp(Box::new(Expr::Assign {
                    name: sp("y".to_string()),
                    body: sp(Box::new(Expr::Literal(Value::Int(1)))),
                })),
            },
        );
    }

    #[test]
    fn test_parse_sequence() {
        assert_parses_to(
            "1; 2",
            Expr::Then {
                first: sp(Box::new(Expr::Literal(Value::Int(1)))),
                next: sp(Box::new(Expr::Literal(Value::Int(2)))),
            },
        );
    }

    #[test]
    fn test_parse_block() {
        assert_parses_to(
            "{ 1; 2 }",
            Expr::Block {
                body: sp(Box::new(Expr::Then {
                    first: sp(Box::new(Expr::Literal(Value::Int(1)))),
                    next: sp(Box::new(Expr::Literal(Value::Int(2)))),
                })),
            },
        );
    }

    #[test]
    fn test_parse_empty_input_is_null() {
        assert_parses_to("", Expr::Null);
    }

    #[test]
    fn test_parse_fn() {
        assert_parses_to(
            "fn(a, b) { a }",
            Expr::Func {
                args: vec![sp("a".to_string()), sp("b".to_string())],
                body: sp(Box::new(Expr::Block {
                    body: sp(Box::new(Expr::Local {
                        name: sp("a".to_string()),
                    })),
                })),
            },
        );
    }

    #[test]
    fn test_span_binary_covers_both_operands() {
        // "1 + 22"
        //  0123456
        let expr = parse("1 + 22");

        assert_eq!((expr.start, expr.end), (0, 5));

        let Expr::BinOp { left, right, .. } = &**expr else {
            panic!("expected BinOp");
        };
        assert_eq!((left.start, left.end), (0, 0));
        assert_eq!((right.start, right.end), (4, 5));
    }

    #[test]
    fn test_span_binary_left_associative_nests_correctly() {
        // "1 - 2 - 3"
        //  012345678
        let expr = parse("1 - 2 - 3");

        assert_eq!((expr.start, expr.end), (0, 8));

        let Expr::BinOp { left, right, .. } = &**expr else {
            panic!("expected BinOp");
        };
        assert_eq!((right.start, right.end), (8, 8));

        let Expr::BinOp { left, right, .. } = &***left else {
            panic!("expected nested BinOp");
        };
        assert_eq!((left.start, left.end), (0, 0));
        assert_eq!((right.start, right.end), (4, 4));
    }

    #[test]
    fn test_span_unary_covers_operator_and_operand() {
        // "-5"
        //  01
        let expr = parse("-5");
        assert_eq!((expr.start, expr.end), (0, 1));
    }

    #[test]
    fn test_span_let_covers_keyword_through_body() {
        // "let x = 5"
        //  012345678
        let expr = parse("let x = 5");
        assert_eq!((expr.start, expr.end), (0, 8));

        let Expr::Let { body, .. } = &**expr else {
            panic!("expected Let");
        };
        assert_eq!((body.start, body.end), (8, 8));
    }

    #[test]
    fn test_span_assign_covers_target_through_body() {
        // "x = 5"
        //  01234
        let expr = parse("x = 5");
        assert_eq!((expr.start, expr.end), (0, 4));
    }

    #[test]
    fn test_span_sequence_covers_both_statements() {
        // "1; 2"
        //  0123
        let expr = parse("1; 2");
        assert_eq!((expr.start, expr.end), (0, 3));

        let Expr::Then { first, next } = &**expr else {
            panic!("expected Then");
        };
        assert_eq!((first.start, first.end), (0, 0));
        assert_eq!((next.start, next.end), (3, 3));
    }

    fn recovering(src: &str) -> (SpannedExpr, Vec<Spanned<Error>>) {
        let registry = registry();
        let tokens = Lexer::lex(src, &registry).unwrap();
        Parser::parse_recovering(&registry, &tokens)
    }

    #[test]
    fn test_recovering_valid_input_has_no_diagnostics() {
        let (_, diagnostics) = recovering("1 + 2");
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_recovering_truncated_let_yields_error_node_and_diagnostic() {
        let (tree, diagnostics) = recovering("let x = ");
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(**tree, Expr::Error(_)));
    }

    #[test]
    fn test_recovering_trailing_operator_yields_error_node_and_diagnostic() {
        let (tree, diagnostics) = recovering("1 + ");
        assert_eq!(diagnostics.len(), 1);
        assert!(matches!(**tree, Expr::Error(_)));
    }

    #[test]
    fn test_recovering_resyncs_at_semicolon_and_parses_the_rest() {
        let (tree, diagnostics) = recovering("1 + ; 2");
        assert_eq!(diagnostics.len(), 1);

        let Expr::Then { first, next } = &**tree else {
            panic!("expected Then");
        };
        assert!(matches!(***first, Expr::Error(_)));
        assert_eq!(***next, Expr::Literal(Value::Int(2)));
    }

    #[test]
    fn test_recovering_reports_every_broken_statement() {
        let (_, diagnostics) = recovering("1 + ; 2 + ; 3");
        assert_eq!(diagnostics.len(), 2);
    }

    #[test]
    fn test_recovering_missing_semicolon_between_statements_is_diagnosed() {
        let (tree, diagnostics) = recovering("1 2");
        assert_eq!(diagnostics.len(), 1);

        let Expr::Then { first, next } = &**tree else {
            panic!("expected Then");
        };
        assert_eq!(***first, Expr::Literal(Value::Int(1)));
        assert_eq!(***next, Expr::Literal(Value::Int(2)));
    }

    #[test]
    fn test_strict_parse_errors_on_missing_semicolon_between_statements() {
        let registry = registry();
        let tokens = Lexer::lex("1 2", &registry).unwrap();
        let err = Parser::parse(&registry, &tokens).unwrap_err();
        assert_eq!(
            *err,
            Error::Expected("<ctrl>`;`".to_string(), "<int>`2`".to_string())
        );
    }

    #[test]
    fn test_parse_block_with_trailing_semicolon() {
        assert_parses_to(
            "{ 1; }",
            Expr::Block {
                body: sp(Box::new(Expr::Then {
                    first: sp(Box::new(Expr::Literal(Value::Int(1)))),
                    next: sp(Box::new(Expr::Null)),
                })),
            },
        );
    }

    #[test]
    fn test_parse_optional_semicolon_after_if_in_sequence() {
        assert_parses_to(
            "if true { 1 } 2",
            Expr::Then {
                first: sp(Box::new(Expr::If {
                    cond: sp(Box::new(Expr::Literal(Value::Bool(true)))),
                    then: sp(Box::new(Expr::Block {
                        body: sp(Box::new(Expr::Literal(Value::Int(1)))),
                    })),
                    else_: None,
                })),
                next: sp(Box::new(Expr::Literal(Value::Int(2)))),
            },
        );
    }

    #[test]
    fn test_parse_optional_semicolon_after_for_in_block() {
        assert_parses_to(
            "{ for x in list { x } 2 }",
            Expr::Block {
                body: sp(Box::new(Expr::Then {
                    first: sp(Box::new(Expr::For {
                        name: Some(sp("x".to_string())),
                        iterator: sp(Box::new(Expr::Local {
                            name: sp("list".to_string()),
                        })),
                        body: sp(Box::new(Expr::Block {
                            body: sp(Box::new(Expr::Local {
                                name: sp("x".to_string()),
                            })),
                        })),
                    })),
                    next: sp(Box::new(Expr::Literal(Value::Int(2)))),
                })),
            },
        );
    }

    #[test]
    fn test_parse_explicit_semicolon_after_if_still_works() {
        assert_eq!(
            zero(parse("if true { 1 } 2")),
            zero(parse("if true { 1 }; 2")),
        );
    }

    #[test]
    fn test_recovering_still_parses_fully_valid_sequences() {
        let (tree, diagnostics) = recovering("let x = 1; x + 1");
        assert!(diagnostics.is_empty());
        assert_eq!(zero(tree), zero(parse("let x = 1; x + 1")));
    }
}
