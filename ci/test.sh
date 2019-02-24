# install dependencies
rustup component add rustfmt
rustup component add clippy
cargo install --force cargo-audit
cargo install --force --git https://github.com/kbknapp/cargo-outdated

# tests
cargo fmt --all -- --check
cargo build 
cargo test --verbose
cargo clippy -- -D warnings
cargo audit
cargo outdated