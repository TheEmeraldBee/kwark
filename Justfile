export RUST_LOG := "debug"

run:
    cd flavors/default && cargo run

run-release:
    cd flavors/default && cargo run --release

test:
    cargo test --workspace

open-book:
    mdbook serve book --open
