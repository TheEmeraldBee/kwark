use std::collections::HashMap;

use indexmap::IndexMap;

use crate::{
    error::Error,
    expr::{Expr, SpannedExpr},
    lex::Lexer,
    parse::Parser,
    prelude::OpRegistry,
    scope::Scope,
    spanned::Spanned,
    value::{Type, Value},
};

type NativeMethod<Cx> = Box<dyn for<'a> Fn(&mut Args<'a, Cx>) -> Result<Value, Error>>;

#[derive(Debug, Clone, PartialEq)]
pub struct ArgSpec {
    pub desc: String,
    pub ty: Option<Type>,
}

#[derive(Clone)]
enum ArgSlot<'a> {
    Single(&'a SpannedExpr),
    Many(Vec<&'a SpannedExpr>),
    Value(Value),
}

pub struct Args<'a, Cx> {
    engine: &'a Engine<Cx>,
    scope: &'a mut Scope,
    cx: &'a mut Cx,
    slots: IndexMap<String, ArgSlot<'a>>,
}

impl<'a, Cx> Args<'a, Cx> {
    pub fn cx(&mut self) -> &mut Cx {
        self.cx
    }

    pub fn value(&mut self, name: &str) -> Result<Value, Error> {
        let slot = self
            .slots
            .get(name)
            .ok_or_else(|| Error::UnknownArg(name.to_string()))?;

        let resolved = match slot {
            ArgSlot::Value(v) => return Ok(v.clone()),
            ArgSlot::Single(expr) => {
                let expr = *expr;
                self.engine
                    .solve(expr, self.scope, self.cx)
                    .map_err(Spanned::into_inner)?
            }
            ArgSlot::Many(exprs) => {
                let exprs = exprs.clone();
                let mut items = Vec::with_capacity(exprs.len());
                for expr in exprs {
                    items.push(
                        self.engine
                            .solve(expr, self.scope, self.cx)
                            .map_err(Spanned::into_inner)?,
                    );
                }
                Value::List(items)
            }
        };

        self.slots
            .insert(name.to_string(), ArgSlot::Value(resolved.clone()));

        Ok(resolved)
    }

    pub fn null(&mut self, name: &str) -> Result<bool, Error> {
        Ok(self.value(name)?.null())
    }

    pub fn bool(&mut self, name: &str) -> Result<bool, Error> {
        self.value(name)?.bool()
    }

    pub fn int(&mut self, name: &str) -> Result<i32, Error> {
        self.value(name)?.int()
    }

    pub fn float(&mut self, name: &str) -> Result<f32, Error> {
        self.value(name)?.float()
    }

    pub fn str(&mut self, name: &str) -> Result<String, Error> {
        self.value(name)?.str().map(str::to_string)
    }

    pub fn list(&mut self, name: &str) -> Result<Vec<Value>, Error> {
        self.value(name)?.list()
    }

    pub fn method(&mut self, name: &str) -> Result<(Vec<String>, SpannedExpr), Error> {
        self.value(name)?.method()
    }

    pub fn mapped_list<T>(
        &mut self,
        name: &str,
        mapper: impl Fn(&Value) -> Result<T, Error>,
    ) -> Result<Vec<T>, Error> {
        self.value(name)?.mapped_list(mapper)
    }
}

// A Callable Rust Function
pub struct Function<Cx> {
    desc: String,

    args: IndexMap<String, ArgSpec>,

    variadic: Option<(String, ArgSpec)>,

    method: NativeMethod<Cx>,
}

impl<Cx> Function<Cx> {
    pub fn desc(&self) -> &str {
        &self.desc
    }

    pub fn args(&self) -> &IndexMap<String, ArgSpec> {
        &self.args
    }

    pub fn variadic(&self) -> Option<(&str, &ArgSpec)> {
        self.variadic
            .as_ref()
            .map(|(name, spec)| (name.as_str(), spec))
    }

    fn validate_args(&self, args: &[SpannedExpr]) -> Result<(), Error> {
        let expected = self.args.len();

        let satisfied = match self.variadic {
            Some(_) => args.len() >= expected,
            None => args.len() == expected,
        };

        if !satisfied {
            return Err(Error::WrongArgCount {
                expected,
                got: args.len(),
            });
        }

        Ok(())
    }

    fn bind_args<'a>(&self, args: &'a [SpannedExpr]) -> IndexMap<String, ArgSlot<'a>> {
        let (fixed, tail) = args.split_at(self.args.len());
        let mut named: IndexMap<String, ArgSlot<'a>> = self
            .args
            .keys()
            .cloned()
            .zip(fixed.iter().map(ArgSlot::Single))
            .collect();

        if let Some((name, _)) = &self.variadic {
            named.insert(name.clone(), ArgSlot::Many(tail.iter().collect()));
        }

        named
    }
}

fn numeric_binop(
    a: &Value,
    b: &Value,
    op: &str,
    int_op: impl Fn(i32, i32) -> i32,
    float_op: impl Fn(f32, f32) -> f32,
) -> Result<Value, Error> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(float_op(*a, *b))),
        (a, b) => Err(Error::TypeMismatch(format!(
            "{op} expected two numbers of the same type, found {a:?} and {b:?}"
        ))),
    }
}

fn numeric_cmp(
    a: &Value,
    b: &Value,
    op: &str,
    int_op: impl Fn(i32, i32) -> bool,
    float_op: impl Fn(f32, f32) -> bool,
) -> Result<Value, Error> {
    match (a, b) {
        (Value::Int(a), Value::Int(b)) => Ok(Value::Bool(int_op(*a, *b))),
        (Value::Float(a), Value::Float(b)) => Ok(Value::Bool(float_op(*a, *b))),
        (a, b) => Err(Error::TypeMismatch(format!(
            "{op} expected two numbers of the same type, found {a:?} and {b:?}"
        ))),
    }
}

#[derive(Default)]
pub struct FunctionBuilder {
    desc: String,
    args: IndexMap<String, ArgSpec>,
    variadic: Option<(String, ArgSpec)>,
}

impl FunctionBuilder {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn desc(mut self, desc: impl Into<String>) -> Self {
        self.desc = desc.into();
        self
    }

    pub fn arg(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        ty: Option<Type>,
    ) -> Self {
        self.args.insert(
            name.into(),
            ArgSpec {
                desc: desc.into(),
                ty,
            },
        );
        self
    }

    pub fn variadic(
        mut self,
        name: impl Into<String>,
        desc: impl Into<String>,
        ty: Option<Type>,
    ) -> Self {
        self.variadic = Some((
            name.into(),
            ArgSpec {
                desc: desc.into(),
                ty,
            },
        ));
        self
    }

    pub fn build<Cx>(
        self,
        method: impl for<'a> Fn(&mut Args<'a, Cx>) -> Result<Value, Error> + 'static,
    ) -> Function<Cx> {
        Function {
            desc: self.desc,
            args: self.args,
            variadic: self.variadic,
            method: Box::new(method),
        }
    }
}

pub struct NamespaceBuilder<'engine, Cx> {
    name: String,
    engine: &'engine mut Engine<Cx>,
}

impl<'engine, Cx> NamespaceBuilder<'engine, Cx> {
    pub fn register(&mut self, name: impl Into<String>, func: Function<Cx>) -> &mut Self {
        self.engine
            .register(format!("{}::{}", self.name, name.into()), func);
        self
    }
}

pub struct Engine<Cx> {
    ops: OpRegistry,

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
    pub fn default_std() -> Self {
        let mut engine = Self::default();

        engine
            .ops
            .binary_ops
            .insert("+".to_string(), ("add".to_string(), 3));
        engine
            .ops
            .binary_ops
            .insert("-".to_string(), ("sub".to_string(), 3));
        engine
            .ops
            .binary_ops
            .insert("*".to_string(), ("mul".to_string(), 4));
        engine
            .ops
            .binary_ops
            .insert("/".to_string(), ("div".to_string(), 4));
        engine
            .ops
            .binary_ops
            .insert("==".to_string(), ("eq".to_string(), 2));
        engine
            .ops
            .binary_ops
            .insert("!=".to_string(), ("neq".to_string(), 2));
        engine
            .ops
            .binary_ops
            .insert("<".to_string(), ("lt".to_string(), 2));
        engine
            .ops
            .binary_ops
            .insert(">".to_string(), ("gt".to_string(), 2));
        engine
            .ops
            .binary_ops
            .insert("&&".to_string(), ("and".to_string(), 1));
        engine
            .ops
            .binary_ops
            .insert("||".to_string(), ("or".to_string(), 0));

        engine
            .ops
            .unary_ops
            .insert("-".to_string(), "neg".to_string());
        engine
            .ops
            .unary_ops
            .insert("!".to_string(), "not".to_string());

        engine.register(
            "add",
            FunctionBuilder::new()
                .desc("Adds two numbers")
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| {
                    numeric_binop(
                        &args.value("a")?,
                        &args.value("b")?,
                        "add",
                        |a, b| a + b,
                        |a, b| a + b,
                    )
                }),
        );
        engine.register(
            "sub",
            FunctionBuilder::new()
                .desc("Subtracts two numbers")
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| {
                    numeric_binop(
                        &args.value("a")?,
                        &args.value("b")?,
                        "sub",
                        |a, b| a - b,
                        |a, b| a - b,
                    )
                }),
        );
        engine.register(
            "mul",
            FunctionBuilder::new()
                .desc("Multiplies two numbers")
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| {
                    numeric_binop(
                        &args.value("a")?,
                        &args.value("b")?,
                        "mul",
                        |a, b| a * b,
                        |a, b| a * b,
                    )
                }),
        );
        engine.register(
            "div",
            FunctionBuilder::new()
                .desc("Divides two numbers")
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| {
                    numeric_binop(
                        &args.value("a")?,
                        &args.value("b")?,
                        "div",
                        |a, b| a / b,
                        |a, b| a / b,
                    )
                }),
        );
        engine.register(
            "eq",
            FunctionBuilder::new()
                .desc("Checks two values for equality")
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| Ok(Value::Bool(args.value("a")? == args.value("b")?))),
        );
        engine.register(
            "neq",
            FunctionBuilder::new()
                .desc("Checks two values for inequality")
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| Ok(Value::Bool(args.value("a")? != args.value("b")?))),
        );
        engine.register(
            "lt",
            FunctionBuilder::new()
                .desc("Checks if a is less than b")
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| {
                    numeric_cmp(
                        &args.value("a")?,
                        &args.value("b")?,
                        "lt",
                        |a, b| a < b,
                        |a, b| a < b,
                    )
                }),
        );
        engine.register(
            "gt",
            FunctionBuilder::new()
                .desc("Checks if a is greater than b")
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| {
                    numeric_cmp(
                        &args.value("a")?,
                        &args.value("b")?,
                        "gt",
                        |a, b| a > b,
                        |a, b| a > b,
                    )
                }),
        );
        engine.register(
            "neg",
            FunctionBuilder::new()
                .desc("Negates a number")
                .arg("a", "operand", None)
                .build(|args| match args.value("a")? {
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
                .arg("a", "operand", Some(Type::Bool))
                .build(|args| Ok(Value::Bool(!args.bool("a")?))),
        );
        engine.register(
            "and",
            FunctionBuilder::new()
                .desc("Logical and of two bools")
                .arg("a", "left operand", Some(Type::Bool))
                .arg("b", "right operand", Some(Type::Bool))
                .build(|args| Ok(Value::Bool(args.bool("a")? && args.bool("b")?))),
        );
        engine.register(
            "or",
            FunctionBuilder::new()
                .desc("Logical or of two bools")
                .arg("a", "left operand", Some(Type::Bool))
                .arg("b", "right operand", Some(Type::Bool))
                .build(|args| Ok(Value::Bool(args.bool("a")? || args.bool("b")?))),
        );

        engine.register(
            "len",
            FunctionBuilder::new()
                .desc("The length of the passed in array or string")
                .arg("val", "the value to get the length of", None)
                .build(|args| {
                    let value = args.value("val")?;
                    let length = match value {
                        Value::Null => 0,
                        Value::Str(s) => s.len() as i32,
                        Value::List(l) => l.len() as i32,
                        _ => {
                            return Err(Error::Expected(
                                "null, str, or list".to_string(),
                                value.type_of().to_string(),
                            ));
                        }
                    };

                    Ok(Value::Int(length))
                }),
        );

        engine
    }

    pub fn register(&mut self, name: impl Into<String>, func: Function<Cx>) -> &mut Self {
        self.methods.insert(name.into(), func);
        self
    }

    pub fn namespace(&mut self, name: impl Into<String>) -> NamespaceBuilder<'_, Cx> {
        NamespaceBuilder {
            name: name.into(),
            engine: self,
        }
    }

    pub fn function(&self, name: &str) -> Option<&Function<Cx>> {
        self.methods.get(name)
    }

    pub fn ops(&self) -> &OpRegistry {
        &self.ops
    }

    pub fn function_names(&self) -> impl Iterator<Item = &str> {
        self.methods.keys().map(String::as_str)
    }

    fn call_native(
        &self,
        name: &str,
        arg_exprs: &[SpannedExpr],
        scope: &mut Scope,
        cx: &mut Cx,
    ) -> Result<Value, Error> {
        let func = self
            .methods
            .get(name)
            .ok_or_else(|| Error::UnknownFunction(name.to_string()))?;

        func.validate_args(arg_exprs)?;

        let mut args = Args {
            engine: self,
            scope,
            cx,
            slots: func.bind_args(arg_exprs),
        };

        (func.method)(&mut args)
    }

    pub fn exec(
        &self,
        source: &str,
        scope: &mut Scope,
        cx: &mut Cx,
    ) -> Result<Value, Spanned<Error>> {
        let tokens = Lexer::lex(source, &self.ops)?;
        let expr = Parser::parse(&self.ops, &tokens)?;

        self.solve(&expr, scope, cx)
    }

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

            Expr::Error(err) => return Err(err.clone()),

            Expr::List(items) => {
                let mut out = Vec::with_capacity(items.len());
                for item in items {
                    out.push(self.solve(item, scope, cx)?);
                }
                Value::List(out)
            }

            Expr::Let { name, body } => {
                let value = self.solve(body, scope, cx)?;
                scope.register(name.clone().into_inner(), value);
                Value::Null
            }

            Expr::Assign { name, body } => {
                let value = self.solve(body, scope, cx)?;
                let name = name.clone().into_inner();
                if !scope.set(name.clone(), value.clone()) {
                    return Err(at(Error::UnknownLocal(name)));
                }
                value
            }

            Expr::Local { name } => {
                let name = name.clone().into_inner();
                scope
                    .get(name.clone())
                    .cloned()
                    .ok_or_else(|| at(Error::UnknownLocal(name)))?
            }

            Expr::UnaryOp { op, body } => {
                let name = self
                    .ops
                    .unary_ops
                    .get(op)
                    .ok_or_else(|| at(Error::UnknownOperator(op.clone())))?
                    .clone();

                self.call_native(&name, std::slice::from_ref(body), scope, cx)
                    .map_err(at)?
            }

            Expr::BinOp { left, op, right } => {
                let name = self
                    .ops
                    .binary_ops
                    .get(op)
                    .ok_or_else(|| at(Error::UnknownOperator(op.clone())))?
                    .0
                    .clone();

                let arg_exprs = [left.clone(), right.clone()];
                self.call_native(&name, &arg_exprs, scope, cx).map_err(at)?
            }

            Expr::If { cond, then, else_ } => {
                let cond = self.solve(cond, scope, cx)?.bool().map_err(at)?;

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
                while self.solve(iterator, scope, cx)?.bool().map_err(at)? {
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
                let native_name = match &***body {
                    Expr::Local { name } => {
                        let name = name.clone().into_inner();

                        (scope.get(name.clone()).is_none() && self.function(&name).is_some())
                            .then_some(name)
                    }
                    _ => None,
                };

                if let Some(name) = native_name {
                    self.call_native(&name, args, scope, cx).map_err(at)?
                } else {
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

                    scope.push_blocking_frame();
                    for (param, value) in params.into_iter().zip(values) {
                        scope.register(param, value);
                    }
                    let res = self.solve(&fn_body, scope, cx);
                    scope.pop_frame();
                    res?
                }
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

    fn lit(value: Value) -> SpannedExpr {
        Spanned::new(0, 0, Box::new(Expr::Literal(value)))
    }

    fn native(
        arity: usize,
        f: impl for<'a> Fn(&mut Args<'a, ()>) -> Result<Value, Error> + 'static,
    ) -> Function<()> {
        let args = (0..arity)
            .map(|i| {
                (
                    format!("arg{i}"),
                    ArgSpec {
                        desc: String::new(),
                        ty: None,
                    },
                )
            })
            .collect();

        Function {
            desc: String::new(),
            args,
            variadic: None,
            method: Box::new(f),
        }
    }

    fn test_engine() -> Engine<()> {
        let mut methods = HashMap::new();

        methods.insert(
            "add".to_string(),
            native(2, |args| {
                Ok(Value::Int(args.int("arg0")? + args.int("arg1")?))
            }),
        );
        methods.insert(
            "sub".to_string(),
            native(2, |args| {
                Ok(Value::Int(args.int("arg0")? - args.int("arg1")?))
            }),
        );
        methods.insert(
            "mul".to_string(),
            native(2, |args| {
                Ok(Value::Int(args.int("arg0")? * args.int("arg1")?))
            }),
        );
        methods.insert(
            "lt".to_string(),
            native(2, |args| {
                Ok(Value::Bool(args.int("arg0")? < args.int("arg1")?))
            }),
        );
        methods.insert(
            "eq".to_string(),
            native(2, |args| {
                Ok(Value::Bool(args.value("arg0")? == args.value("arg1")?))
            }),
        );
        methods.insert(
            "neg".to_string(),
            native(1, |args| Ok(Value::Int(-args.int("arg0")?))),
        );
        methods.insert(
            "not".to_string(),
            native(1, |args| Ok(Value::Bool(!args.bool("arg0").unwrap()))),
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
    fn test_exec_lexes_parses_and_evaluates() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        assert_eq!(
            engine
                .exec("let x = 1; x + 2 * 3", &mut scope, &mut ())
                .unwrap(),
            Value::Int(7)
        );
    }

    #[test]
    fn test_exec_surfaces_a_lex_error() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        let err = engine
            .exec(r#""unterminated"#, &mut scope, &mut ())
            .unwrap_err();
        assert_eq!(*err, Error::UnclosedString);
    }

    #[test]
    fn test_exec_surfaces_a_parse_error() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        let err = engine.exec("let x = ", &mut scope, &mut ()).unwrap_err();
        assert_eq!(*err, Error::ExpectedFoundEOF("expr".to_string()));
    }

    #[test]
    fn test_exec_surfaces_a_solve_error() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        let err = engine.exec("missing", &mut scope, &mut ()).unwrap_err();
        assert_eq!(*err, Error::UnknownLocal("missing".to_string()));
    }

    #[test]
    fn test_exec_shares_scope_across_calls() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        engine.exec("let x = 5", &mut scope, &mut ()).unwrap();
        assert_eq!(
            engine.exec("x + 1", &mut scope, &mut ()).unwrap(),
            Value::Int(6)
        );
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
        assert_eq!(eval("let n = 0; for n < 3 { n = n + 1 }; n"), Value::Int(3));
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
        assert_eq!(
            eval("let add_one = fn(a) { a + 1 }; add_one(41)"),
            Value::Int(42)
        );
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
    fn test_solve_call_native_function_by_name() {
        assert_eq!(eval("add(1, 2)"), Value::Int(3));
    }

    #[test]
    fn test_solve_call_prefers_a_scope_variable_over_a_native_function() {
        assert_eq!(eval("let add = fn(a) { a }; add(5)"), Value::Int(5));
    }

    #[test]
    fn test_solve_call_native_function_wrong_arg_count_errors() {
        let registry = registry();
        let tokens = Lexer::lex("add(1)", &registry).unwrap();
        let expr = Parser::parse(&registry, &tokens).unwrap();

        let engine = test_engine();
        let mut scope = Scope::default();

        let err = engine.solve(&expr, &mut scope, &mut ()).unwrap_err();
        assert_eq!(
            *err,
            Error::WrongArgCount {
                expected: 2,
                got: 1
            }
        );
    }

    #[test]
    fn test_solve_call_unknown_name_errors() {
        let registry = registry();
        let tokens = Lexer::lex("missing(1)", &registry).unwrap();
        let expr = Parser::parse(&registry, &tokens).unwrap();

        let engine = test_engine();
        let mut scope = Scope::default();

        let err = engine.solve(&expr, &mut scope, &mut ()).unwrap_err();
        assert_eq!(*err, Error::UnknownLocal("missing".to_string()));
    }

    #[test]
    fn test_function_builder_stores_desc_and_args() {
        let func: Function<()> = FunctionBuilder::new()
            .desc("adds two numbers")
            .arg("a", "left operand", Some(Type::Int))
            .arg("b", "right operand", Some(Type::Int))
            .build(|args| Ok(Value::Int(args.int("a")? + args.int("b")?)));

        assert_eq!(func.desc(), "adds two numbers");
        assert_eq!(
            func.args()
                .iter()
                .map(|(n, spec)| (n.as_str(), spec.desc.as_str(), spec.ty))
                .collect::<Vec<_>>(),
            vec![
                ("a", "left operand", Some(Type::Int)),
                ("b", "right operand", Some(Type::Int)),
            ]
        );
    }

    #[test]
    fn test_function_builder_stores_variadic() {
        let func: Function<()> = FunctionBuilder::new()
            .arg("a", "first operand", None)
            .variadic("rest", "the remaining operands", Some(Type::Int))
            .build(|_| Ok(Value::Null));

        let (name, spec) = func.variadic().unwrap();
        assert_eq!(name, "rest");
        assert_eq!(spec.desc, "the remaining operands");
        assert_eq!(spec.ty, Some(Type::Int));
    }

    #[test]
    fn test_call_native_errors_on_wrong_arg_type() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "not",
            FunctionBuilder::new()
                .arg("a", "operand", Some(Type::Bool))
                .build(|args| Ok(Value::Bool(!args.bool("a")?))),
        );

        let err = engine
            .call_native("not", &[lit(Value::Int(1))], &mut Scope::default(), &mut ())
            .unwrap_err();
        assert_eq!(err, Error::ExpectedType(Type::Bool, Type::Int));
    }

    #[test]
    fn test_call_native_accepts_a_matching_arg_type() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "not",
            FunctionBuilder::new()
                .arg("a", "operand", Some(Type::Bool))
                .build(|args| Ok(Value::Bool(!args.bool("a")?))),
        );

        let result = engine.call_native(
            "not",
            &[lit(Value::Bool(true))],
            &mut Scope::default(),
            &mut (),
        );
        assert_eq!(result, Ok(Value::Bool(false)));
    }

    #[test]
    fn test_call_native_checks_the_variadic_type() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "sum",
            FunctionBuilder::new()
                .arg("first", "always required", Some(Type::Int))
                .variadic("rest", "everything else", Some(Type::Int))
                .build(|args| {
                    let rest: i32 = args.mapped_list("rest", Value::int)?.iter().sum();
                    Ok(Value::Int(args.int("first")? + rest))
                }),
        );

        let err = engine
            .call_native(
                "sum",
                &[
                    lit(Value::Int(1)),
                    lit(Value::Int(2)),
                    lit(Value::Bool(true)),
                ],
                &mut Scope::default(),
                &mut (),
            )
            .unwrap_err();
        assert_eq!(err, Error::ExpectedType(Type::Int, Type::Bool));
    }

    #[test]
    fn test_call_native_errors_on_too_few_args() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "add",
            FunctionBuilder::new()
                .arg("a", "left operand", None)
                .arg("b", "right operand", None)
                .build(|args| Ok(Value::Int(args.int("a")? + args.int("b")?))),
        );

        let err = engine
            .call_native("add", &[lit(Value::Int(1))], &mut Scope::default(), &mut ())
            .unwrap_err();
        assert_eq!(
            err,
            Error::WrongArgCount {
                expected: 2,
                got: 1
            }
        );
    }

    #[test]
    fn test_call_native_errors_on_too_many_args_without_variadic() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "id",
            FunctionBuilder::new()
                .arg("a", "operand", None)
                .build(|args| Ok(args.value("a")?.clone())),
        );

        let err = engine
            .call_native(
                "id",
                &[lit(Value::Int(1)), lit(Value::Int(2))],
                &mut Scope::default(),
                &mut (),
            )
            .unwrap_err();
        assert_eq!(
            err,
            Error::WrongArgCount {
                expected: 1,
                got: 2
            }
        );
    }

    #[test]
    fn test_call_native_variadic_accepts_extra_args() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "sum",
            FunctionBuilder::new()
                .arg("first", "always required", None)
                .variadic("rest", "everything else", None)
                .build(|args| {
                    let rest: i32 = args.mapped_list("rest", Value::int)?.iter().sum();
                    Ok(Value::Int(args.int("first")? + rest))
                }),
        );

        let result = engine.call_native(
            "sum",
            &[lit(Value::Int(1)), lit(Value::Int(2)), lit(Value::Int(3))],
            &mut Scope::default(),
            &mut (),
        );
        assert_eq!(result, Ok(Value::Int(6)));
    }

    #[test]
    fn test_call_native_variadic_still_requires_the_fixed_args() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "sum",
            FunctionBuilder::new()
                .arg("first", "always required", None)
                .variadic("rest", "everything else", None)
                .build(|args| {
                    let rest: i32 = args.mapped_list("rest", Value::int)?.iter().sum();
                    Ok(Value::Int(args.int("first")? + rest))
                }),
        );

        let err = engine
            .call_native("sum", &[], &mut Scope::default(), &mut ())
            .unwrap_err();
        assert_eq!(
            err,
            Error::WrongArgCount {
                expected: 1,
                got: 0
            }
        );
    }

    #[test]
    fn test_engine_register_and_function_lookup() {
        let mut engine: Engine<()> = Engine::default();
        engine.register(
            "double",
            FunctionBuilder::new()
                .arg("a", "operand", None)
                .build(|args| Ok(Value::Int(args.int("a")? * 2))),
        );

        assert!(engine.function("double").is_some());
        assert!(engine.function("missing").is_none());

        let result = engine.call_native(
            "double",
            &[lit(Value::Int(21))],
            &mut Scope::default(),
            &mut (),
        );
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

    #[test]
    fn test_default_std_and_or_evaluate_correctly() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        assert_eq!(
            engine.exec("true && true", &mut scope, &mut ()).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            engine.exec("true && false", &mut scope, &mut ()).unwrap(),
            Value::Bool(false)
        );
        assert_eq!(
            engine.exec("false || true", &mut scope, &mut ()).unwrap(),
            Value::Bool(true)
        );
        assert_eq!(
            engine.exec("false || false", &mut scope, &mut ()).unwrap(),
            Value::Bool(false)
        );
    }

    #[test]
    fn test_default_std_and_or_short_circuit() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        engine.exec("let x = false", &mut scope, &mut ()).unwrap();

        engine
            .exec("true || (x = true)", &mut scope, &mut ())
            .unwrap();
        assert_eq!(scope.get("x"), Some(&Value::Bool(false)));

        engine
            .exec("false && (x = true)", &mut scope, &mut ())
            .unwrap();
        assert_eq!(scope.get("x"), Some(&Value::Bool(false)));

        engine
            .exec("false || (x = true)", &mut scope, &mut ())
            .unwrap();
        assert_eq!(scope.get("x"), Some(&Value::Bool(true)));

        engine.exec("x = false", &mut scope, &mut ()).unwrap();
        engine
            .exec("true && (x = true)", &mut scope, &mut ())
            .unwrap();
        assert_eq!(scope.get("x"), Some(&Value::Bool(true)));
    }

    #[test]
    fn test_default_std_and_or_precedence() {
        let engine: Engine<()> = Engine::default_std();
        let mut scope = Scope::default();

        assert_eq!(
            engine
                .exec("true || false && false", &mut scope, &mut ())
                .unwrap(),
            Value::Bool(true)
        );

        assert_eq!(
            engine.exec("1 < 2 && 3 < 4", &mut scope, &mut ()).unwrap(),
            Value::Bool(true)
        );
    }
}
