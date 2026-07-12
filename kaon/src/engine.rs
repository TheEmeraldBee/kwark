use std::collections::HashMap;

use crate::{
    error::Error,
    expr::{Expr, SpannedExpr},
    prelude::OpRegistry,
    scope::Scope,
    spanned::Spanned,
    value::Value,
};

type NativeMethod<Cx> = Box<dyn Fn(&mut Cx, Vec<Value>) -> Result<Value, Error>>;

/// A function that describes a callable rust function
pub struct Function<Cx> {
    /// The description for arguments
    desc: String,

    /// A type, desc for each argument
    args: Vec<(String, String)>,

    /// The actual rust-call for the method
    method: NativeMethod<Cx>,
}

impl<Cx> Function<Cx> {
    /// The function's description
    pub fn desc(&self) -> &str {
        &self.desc
    }

    /// The name and description of each argument, in call order
    pub fn args(&self) -> &[(String, String)] {
        &self.args
    }
}

/// Applies an int or float op to a two-argument numeric call, erroring on a type mismatch
fn numeric_binop(
    args: &[Value],
    op: &str,
    int_op: impl Fn(i32, i32) -> i32,
    float_op: impl Fn(f32, f32) -> f32,
) -> Result<Value, Error> {
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a, *b))),
        (a, b) => Err(Error::TypeMismatch(format!(
            "{op} expected two numbers of the same type, found {a:?} and {b:?}"
        ))),
    }
}

/// Applies an int or float comparison to a two-argument numeric call, erroring on a type mismatch
fn numeric_cmp(
    args: &[Value],
    op: &str,
    int_op: impl Fn(i32, i32) -> bool,
    float_op: impl Fn(f32, f32) -> bool,
) -> Result<Value, Error> {
    match (&args[0], &args[1]) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(float_op(*a, *b))),
        (a, b) => Err(Error::TypeMismatch(format!(
            "{op} expected two numbers of the same type, found {a:?} and {b:?}"
        ))),
    }
}

/// Builds a [`Function`] with a fluent `.arg(..).desc(..).build(..)` chain
#[derive(Default)]
pub struct FunctionBuilder {
    desc: String,
    args: Vec<(String, String)>,
}

impl FunctionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    /// Sets the description for this function
    pub fn desc(mut self, desc: impl Into<String>) -> Self {
        self.desc = desc.into();
        self
    }

    /// Adds an argument's name and description, in call order
    pub fn arg(mut self, name: impl Into<String>, desc: impl Into<String>) -> Self {
        self.args.push((name.into(), desc.into()));
        self
    }

    /// Finalizes the builder into a callable [`Function`]
    pub fn build<Cx>(
        self,
        method: impl Fn(&mut Cx, Vec<Value>) -> Result<Value, Error> + 'static,
    ) -> Function<Cx> {
        Function {
            desc: self.desc,
            args: self.args,
            method: Box::new(method),
        }
    }
}

pub struct Engine<Cx> {
    /// The operators registered
    ops: OpRegistry,

    /// The methods that can be run
    methods: HashMap<String, Function<Cx>>,
}

impl<Cx> Default for Engine<Cx> {
    fn default() -> Self {
        Self {
            ops: OpRegistry::default(),

            methods: HashMap::new(),
        }
    }
}

impl<Cx> Engine<Cx> {
    /// The engine with the full std library initialized
    ///
    /// The standard library consists of simple functions and operators for math on values.
    pub fn default_std() -> Self {
        let mut engine = Self::default();

        engine.ops.binary_ops.insert("+".to_string(), ("add".to_string(), 1));
        engine.ops.binary_ops.insert("-".to_string(), ("sub".to_string(), 1));
        engine.ops.binary_ops.insert("*".to_string(), ("mul".to_string(), 2));
        engine.ops.binary_ops.insert("/".to_string(), ("div".to_string(), 2));
        engine.ops.binary_ops.insert("==".to_string(), ("eq".to_string(), 0));
        engine.ops.binary_ops.insert("!=".to_string(), ("neq".to_string(), 0));
        engine.ops.binary_ops.insert("<".to_string(), ("lt".to_string(), 0));
        engine.ops.binary_ops.insert(">".to_string(), ("gt".to_string(), 0));

        engine.ops.unary_ops.insert("-".to_string(), "neg".to_string());
        engine.ops.unary_ops.insert("!".to_string(), "not".to_string());

        engine.register(
            "add",
            FunctionBuilder::new()
                .desc("Adds two numbers")
                .arg("a", "left operand")
                .arg("b", "right operand")
                .build(|_, args| numeric_binop(&args, "add", |a, b| a + b, |a, b| a + b)),
        );
        engine.register(
            "sub",
            FunctionBuilder::new()
                .desc("Subtracts two numbers")
                .arg("a", "left operand")
                .arg("b", "right operand")
                .build(|_, args| numeric_binop(&args, "sub", |a, b| a - b, |a, b| a - b)),
        );
        engine.register(
            "mul",
            FunctionBuilder::new()
                .desc("Multiplies two numbers")
                .arg("a", "left operand")
                .arg("b", "right operand")
                .build(|_, args| numeric_binop(&args, "mul", |a, b| a * b, |a, b| a * b)),
        );
        engine.register(
            "div",
            FunctionBuilder::new()
                .desc("Divides two numbers")
                .arg("a", "left operand")
                .arg("b", "right operand")
                .build(|_, args| numeric_binop(&args, "div", |a, b| a / b, |a, b| a / b)),
        );
        engine.register(
            "eq",
            FunctionBuilder::new()
                .desc("Checks two values for equality")
                .arg("a", "left operand")
                .arg("b", "right operand")
                .build(|_, args| Ok(Value::Bool(args[0] == args[1]))),
        );
        engine.register(
            "neq",
            FunctionBuilder::new()
                .desc("Checks two values for inequality")
                .arg("a", "left operand")
                .arg("b", "right operand")
                .build(|_, args| Ok(Value::Bool(args[0] != args[1]))),
        );
        engine.register(
            "lt",
            FunctionBuilder::new()
                .desc("Checks if a is less than b")
                .arg("a", "left operand")
                .arg("b", "right operand")
                .build(|_, args| numeric_cmp(&args, "lt", |a, b| a < b, |a, b| a < b)),
        );
        engine.register(
            "gt",
            FunctionBuilder::new()
                .desc("Checks if a is greater than b")
                .arg("a", "left operand")
                .arg("b", "right operand")
                .build(|_, args| numeric_cmp(&args, "gt", |a, b| a > b, |a, b| a > b)),
        );
        engine.register(
            "neg",
            FunctionBuilder::new()
                .desc("Negates a number")
                .arg("a", "operand")
                .build(|_, args| match &args[0] {
                    Value::Int(i) => Ok(Value::Int(-i)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    other => Err(Error::TypeMismatch(format!(
                        "neg expected a number, found {other:?}"
                    ))),
                }),
        );
        engine.register(
            "not",
            FunctionBuilder::new()
                .desc("Negates a bool")
                .arg("a", "operand")
                .build(|_, args| Ok(Value::Bool(!args[0].truthy()?))),
        );

        engine
    }

    /// Registers a native rust function under `name`, callable from operators
    pub fn register(&mut self, name: impl Into<String>, func: Function<Cx>) {
        self.methods.insert(name.into(), func);
    }

    /// Looks up a registered native function by name
    pub fn function(&self, name: &str) -> Option<&Function<Cx>> {
        self.methods.get(name)
    }

    /// Calls a native rust function registered under `name`
    fn call_native(&self, name: &str, args: Vec<Value>, cx: &mut Cx) -> Result<Value, Error> {
        let func = self
            .methods
            .get(name)
            .ok_or_else(|| Error::UnknownFunction(name.to_string()))?;

        (func.method)(cx, args)
    }

    /// Evaluates an expression to a value
    pub fn solve(
        &self,
        expr: &SpannedExpr,
        scope: &mut Scope,
        cx: &mut Cx,
    ) -> Result<Value, Spanned<Error>> {
        let at = |e: Error| Spanned::new(expr.start, expr.end, e);

        Ok(match &***expr {
            Expr::Null => Value::Null,
            Expr::Literal(value) => value.clone(),

            Expr::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.solve(item, scope, cx)?);
                }
                Value::List(out)
            }

            Expr::Let { name, body } => {
                let value = self.solve(body, scope, cx)?;
                scope.register(name.clone(), value);
                Value::Null
            }

            Expr::Assign { name, body } => {
                let value = self.solve(body, scope, cx)?;
                if !scope.set(name.clone(), value.clone()) {
                    return Err(at(Error::UnknownLocal(name.clone())));
                }
                value
            }

            Expr::Local { name } => scope
                .get(name.clone())
                .cloned()
                .ok_or_else(|| at(Error::UnknownLocal(name.clone())))?,

            Expr::UnaryOp { op, body } => {
                let value = self.solve(body, scope, cx)?;

                let name = self
                    .ops
                    .unary_ops
                    .get(op)
                    .ok_or_else(|| at(Error::UnknownOperator(op.clone())))?
                    .clone();

                self.call_native(&name, vec![value], cx).map_err(at)?
            }

            Expr::BinOp { left, op, right } => {
                let left = self.solve(left, scope, cx)?;
                let right = self.solve(right, scope, cx)?;

                let name = self
                    .ops
                    .binary_ops
                    .get(op)
                    .ok_or_else(|| at(Error::UnknownOperator(op.clone())))?
                    .0
                    .clone();

                self.call_native(&name, vec![left, right], cx).map_err(at)?
            }

            Expr::If { cond, then, else_ } => {
                let cond = self.solve(cond, scope, cx)?.truthy().map_err(at)?;

                if cond {
                    self.solve(then, scope, cx)?
                } else if let Some(else_) = else_ {
                    self.solve(else_, scope, cx)?
                } else {
                    Value::Null
                }
            }

            Expr::For {
                name: None,
                iterator,
                body,
            } => {
                while self.solve(iterator, scope, cx)?.truthy().map_err(at)? {
                    self.solve(body, scope, cx)?;
                }

                Value::Null
            }
            Expr::For {
                name: Some(name),
                iterator,
                body,
            } => {
                let iter = self.solve(iterator, scope, cx)?.into_values().map_err(at)?;

                for item in iter {
                    scope.push_frame();
                    scope.register(name.clone().into_inner(), item);
                    let res = self.solve(body, scope, cx);
                    scope.pop_frame();
                    res?;
                }

                Value::Null
            }

            Expr::Func { args, body } => Value::Method {
                args: args.iter().map(|a| (**a).clone()).collect(),
                body: body.clone(),
            },

            Expr::Then { first, next } => {
                self.solve(first, scope, cx)?;
                self.solve(next, scope, cx)?
            }

            Expr::Block { body } => {
                scope.push_frame();
                let res = self.solve(body, scope, cx);
                scope.pop_frame();
                res?
            }

            Expr::Call { body, args } => {
                let callee = self.solve(body, scope, cx)?;

                let Value::Method {
                    args: params,
                    body: fn_body,
                } = callee
                else {
                    return Err(at(Error::NotCallable(callee)));
                };

                if params.len() != args.len() {
                    return Err(at(Error::WrongArgCount {
                        expected: params.len(),
                        got: args.len(),
                    }));
                }

                let mut values = Vec::with_capacity(args.len());
                for arg in args {
                    values.push(self.solve(arg, scope, cx)?);
                }

                scope.push_frame();
                for (param, value) in params.into_iter().zip(values) {
                    scope.register(param, value);
                }
                let res = self.solve(&fn_body, scope, cx);
                scope.pop_frame();
                res?
            }
        })
    }
}

#[cfg(test)]
mod test {
    use std::collections::HashMap;

    use super::*;
    use crate::lex::Lexer;
    use crate::parse::Parser;

    fn registry() -> OpRegistry {
        let mut binary_ops = HashMap::new();
        binary_ops.insert("+".to_string(), ("add".to_string(), 1));
        binary_ops.insert("-".to_string(), ("sub".to_string(), 1));
        binary_ops.insert("*".to_string(), ("mul".to_string(), 2));
        binary_ops.insert("<".to_string(), ("lt".to_string(), 0));
        binary_ops.insert("==".to_string(), ("eq".to_string(), 0));

        let mut unary_ops = HashMap::new();
        unary_ops.insert("-".to_string(), "neg".to_string());
        unary_ops.insert("!".to_string(), "not".to_string());

        OpRegistry {
            binary_ops,
            unary_ops,
        }
    }

    fn int(v: &Value) -> i32 {
        match v {
            Value::Int(i) => *i,
            other => panic!("expected int, found {other:?}"),
        }
    }

    fn native(f: impl Fn(&mut (), Vec<Value>) -> Result<Value, Error> + 'static) -> Function<()> {
        Function {
            desc: String::new(),
            args: vec![],
            method: Box::new(f),
        }
    }

    fn test_engine() -> Engine<()> {
        let mut methods = HashMap::new();

        methods.insert(
            "add".to_string(),
            native(|_, args| Ok(Value::Int(int(&args[0]) + int(&args[1])))),
        );
        methods.insert(
            "sub".to_string(),
            native(|_, args| Ok(Value::Int(int(&args[0]) - int(&args[1])))),
        );
        methods.insert(
            "mul".to_string(),
            native(|_, args| Ok(Value::Int(int(&args[0]) * int(&args[1])))),
        );
        methods.insert(
            "lt".to_string(),
            native(|_, args| Ok(Value::Bool(int(&args[0]) < int(&args[1])))),
        );
        methods.insert(
            "eq".to_string(),
            native(|_, args| Ok(Value::Bool(args[0] == args[1]))),
        );
        methods.insert(
            "neg".to_string(),
            native(|_, args| Ok(Value::Int(-int(&args[0])))),
        );
        methods.insert(
            "not".to_string(),
            native(|_, args| Ok(Value::Bool(!args[0].truthy().unwrap()))),
        );

        Engine {
            ops: registry(),
            methods,
        }
    }

    fn eval(src: &str) -> Value {
        let registry = registry();
        let tokens = Lexer::lex(src, &registry).unwrap();
        let expr = Parser::parse(&registry, &tokens).unwrap();

        let engine = test_engine();
        let mut scope = Scope::default();

        engine.solve(&expr, &mut scope, &mut ()).unwrap()
    }

    #[test]
    fn test_solve_literal() {
        assert_eq!(eval("1"), Value::Int(1));
    }

    #[test]
    fn test_solve_binop_dispatches_through_registry() {
        assert_eq!(eval("1 + 2 * 3"), Value::Int(7));
    }

    #[test]
    fn test_solve_unary() {
        assert_eq!(eval("-5"), Value::Int(-5));
    }

    #[test]
    fn test_solve_let_and_local() {
        assert_eq!(eval("let x = 1; x + 1"), Value::Int(2));
    }

    #[test]
    fn test_solve_assign_mutates_existing() {
        assert_eq!(eval("let x = 1; x = 2; x"), Value::Int(2));
    }

    #[test]
    fn test_solve_assign_to_unknown_local_errors() {
        let registry = registry();
        let tokens = Lexer::lex("x = 1", &registry).unwrap();
        let expr = Parser::parse(&registry, &tokens).unwrap();

        let engine = test_engine();
        let mut scope = Scope::default();

        let err = engine.solve(&expr, &mut scope, &mut ()).unwrap_err();
        assert_eq!(*err, Error::UnknownLocal("x".to_string()));
    }

    #[test]
    fn test_solve_if_true_branch() {
        assert_eq!(eval("if 1 < 2 { 10 } else { 20 }"), Value::Int(10));
    }

    #[test]
    fn test_solve_if_false_branch() {
        assert_eq!(eval("if 2 < 1 { 10 } else { 20 }"), Value::Int(20));
    }

    #[test]
    fn test_solve_if_without_else_is_null() {
        assert_eq!(eval("if 2 < 1 { 10 }"), Value::Null);
    }

    #[test]
    fn test_solve_block_scopes_lets() {
        // the `let x` inside the block must not leak into the outer scope
        let registry = registry();
        let tokens = Lexer::lex("{ let x = 1 }; x", &registry).unwrap();
        let expr = Parser::parse(&registry, &tokens).unwrap();

        let engine = test_engine();
        let mut scope = Scope::default();

        let err = engine.solve(&expr, &mut scope, &mut ()).unwrap_err();
        assert_eq!(*err, Error::UnknownLocal("x".to_string()));
    }

    #[test]
    fn test_solve_for_while_style() {
        assert_eq!(
            eval("let n = 0; for n < 3 { n = n + 1 }; n"),
            Value::Int(3)
        );
    }

    #[test]
    fn test_solve_for_in_list_sums_items() {
        assert_eq!(
            eval("let total = 0; for x in [1, 2, 3] { total = total + x }; total"),
            Value::Int(6)
        );
    }

    #[test]
    fn test_solve_for_in_binds_fresh_scope_per_iteration() {
        // `x` from the loop must not leak past the loop
        let registry = registry();
        let tokens = Lexer::lex("for x in [1] { x }; x", &registry).unwrap();
        let expr = Parser::parse(&registry, &tokens).unwrap();

        let engine = test_engine();
        let mut scope = Scope::default();

        let err = engine.solve(&expr, &mut scope, &mut ()).unwrap_err();
        assert_eq!(*err, Error::UnknownLocal("x".to_string()));
    }

    #[test]
    fn test_solve_call_user_function() {
        assert_eq!(eval("let add_one = fn(a) { a + 1 }; add_one(41)"), Value::Int(42));
    }

    #[test]
    fn test_solve_call_wrong_arg_count_errors() {
        let registry = registry();
        let tokens = Lexer::lex("let f = fn(a) { a }; f(1, 2)", &registry).unwrap();
        let expr = Parser::parse(&registry, &tokens).unwrap();

        let engine = test_engine();
        let mut scope = Scope::default();

        let err = engine.solve(&expr, &mut scope, &mut ()).unwrap_err();
        assert_eq!(
            *err,
            Error::WrongArgCount {
                expected: 1,
                got: 2
            }
        );
    }

    #[test]
    fn test_function_builder_stores_desc_and_args() {
        let func: Function<()> = FunctionBuilder::new()
            .desc("adds two numbers")
            .arg("a", "left operand")
            .arg("b", "right operand")
            .build(|_, args| Ok(Value::Int(int(&args[0]) + int(&args[1]))));

        assert_eq!(func.desc(), "adds two numbers");
        assert_eq!(
            func.args(),
            &[
                ("a".to_string(), "left operand".to_string()),
                ("b".to_string(), "right operand".to_string()),
            ]
        );
    }

    #[test]
    fn test_engine_register_and_function_lookup() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "double",
            FunctionBuilder::new().build(|_, args| Ok(Value::Int(int(&args[0]) * 2))),
        );

        assert!(engine.function("double").is_some());
        assert!(engine.function("missing").is_none());

        let result = engine.call_native("double", vec![Value::Int(21)], &mut ());
        assert_eq!(result, Ok(Value::Int(42)));
    }

    #[test]
    fn test_default_std_arithmetic_and_comparison() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        let tokens = Lexer::lex("1 + 2 * 3", &engine.ops).unwrap();
        let expr = Parser::parse(&engine.ops, &tokens).unwrap();
        assert_eq!(
            engine.solve(&expr, &mut scope, &mut ()).unwrap(),
            Value::Int(7)
        );

        let tokens = Lexer::lex("7 == 7", &engine.ops).unwrap();
        let expr = Parser::parse(&engine.ops, &tokens).unwrap();
        assert_eq!(
            engine.solve(&expr, &mut scope, &mut ()).unwrap(),
            Value::Bool(true)
        );

        let tokens = Lexer::lex("-3 < 0", &engine.ops).unwrap();
        let expr = Parser::parse(&engine.ops, &tokens).unwrap();
        assert_eq!(
            engine.solve(&expr, &mut scope, &mut ()).unwrap(),
            Value::Bool(true)
        );

        let tokens = Lexer::lex("!false", &engine.ops).unwrap();
        let expr = Parser::parse(&engine.ops, &tokens).unwrap();
        assert_eq!(
            engine.solve(&expr, &mut scope, &mut ()).unwrap(),
            Value::Bool(true)
        );
    }

    #[test]
    fn test_default_std_type_mismatch_errors() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        let tokens = Lexer::lex("1 + 1.0", &engine.ops).unwrap();
        let expr = Parser::parse(&engine.ops, &tokens).unwrap();

        let err = engine.solve(&expr, &mut scope, &mut ()).unwrap_err();
        assert!(matches!(*err, Error::TypeMismatch(_)));
    }
}
