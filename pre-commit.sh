# !/bin/sh
cargo install cargo-audit;
cargo audit;
rustup component add rustfmt;
cargo fmt --check;
cargo "test";