#!/bin/sh

echo "Home directory: ${HOME}"
echo "Cargo Version: $(cargo --version)"
echo "Rust Version: $(rustc --version)"
echo "Cargo Tarpaulin Version: $(cargo tarpaulin --version)"
cargo clean
cargo build --all-targets --workspace
cargo tarpaulin --skip-clean --all-targets --no-fail-fast --workspace --out Html --output-dir /home/pyrsiaapp/pyrsia/code-coverage-data
