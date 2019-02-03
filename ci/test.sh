rustup component add clippy
cargo install --force cargo-audit
cargo install --force --git https://github.com/kbknapp/cargo-outdated
cargo build 
cargo test --verbose
cargo clippy
cargo audit
cargo outdated