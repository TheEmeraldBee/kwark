export RUST_LOG := "debug"

run:
    cargo run -p kwark

repl:
    cargo run --example repl

test:
    cargo test --workspace
