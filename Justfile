export RUST_LOG := "debug"

run-release:
    cd flavors/default && cargo run --release

run:
    cd flavors/default && cargo run

test:
    cargo build --workspace --all-targets
    cargo test --workspace

open-book:
    mdbook serve book --open
