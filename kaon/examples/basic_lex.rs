use std::collections::HashMap;
use std::io::{Write, stdin, stdout};

use kaon::prelude::*;

fn main() {
    print!("<lex>: ");
    stdout().flush().expect("stdout should have flushed");

    let mut text = String::new();
    stdin().read_line(&mut text).expect("Line should have read");

    let registry = OpRegistry {
        binary_ops: HashMap::new(),
        unary_ops: HashMap::new(),
    };

    let tokens = match Lexer::lex(&text, &registry) {
        Ok(t) => t,
        Err(e) => {
            eprintln!("Lexing Failed:\n{}", e.point_at(&e, &text));

            return;
        }
    };
    println!(
        "Your tokenized output: {}",
        tokens.iter().map(|x| x.to_string()).collect::<String>()
    )
}
