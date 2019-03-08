# install dependencies
rustup component add rustfmt
rustup component add clippy
cargo install --force cargo-audit

# tests
cargo fmt --all -- --check
cargo build 
cargo test --verbose
cargo clippy -- -D warnings
cargo audit