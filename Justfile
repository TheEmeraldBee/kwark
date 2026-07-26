export RUST_LOG := "debug"

run:
    cd flavors/kwark && cargo run

repl:
    cargo run --example repl

test:
    cargo test --workspace
