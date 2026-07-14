use kaon::prelude::*;

fn main() {
    let mut scope = Scope::default();
    let mut engine = Engine::<()>::default_std();

    engine.register(
        "print",
        FunctionBuilder::new()
            .desc("Print the message to stdout")
            .arg("message", "The message to print", Some(Type::Str))
            .build(|args| {
                println!("{}", args.str("message")?);
                Ok(Value::Null)
            }),
    );

    let mut namespace = engine.namespace("hello");
    namespace.register(
        "hi",
        FunctionBuilder::new().build(|_args| {
            println!("hi");
            Ok(Value::Null)
        }),
    );

    engine
        .exec(
            r#"
            let x = 5;
            if x == 5 {
                print("Hello, world");
                hello::hi();
            };
            "#,
            &mut scope,
            &mut (),
        )
        .expect("Working");
}
