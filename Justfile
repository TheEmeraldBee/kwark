export RUST_LOG := "debug"

run:
    cd flavors/default && cargo run

test:
    cargo test --workspace
