# !/bin/sh
echo "Welcome to the Pyrsia local tests.\nThis might take sometime, donot interrupt it if the screen is blank."
cargo clean;
cargo install cargo-audit;
cargo audit;
cargo clippy;   
rustup component add rustfmt;
cargo fmt --check;
cargo "test --workspace";
cargo build --workspace;
