export RUST_LOG := "debug"

run:
    cargo run -p kwark

test:
    cargo test --workspace
