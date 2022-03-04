#!/usr/bin/env bash
printf "Welcome to the Pyrsia local tests.\nThis might take sometime, please do not interrupt if the screen is blank.\n"
if [[ "$1" == "clean" ]] ; then
	printf "Cleaning old build artifacts.\n"
	cargo clean
fi

cargo install cargo-audit || exit_on_error "Could not install cargo audit."
cargo audit || exit_on_error "Cargo audit failed."
# Clippy is failing on macOS right now.
cargo clippy
rustup component add rustfmt || exit_on_error "Could not install rustfmt."
cargo fmt --check || exit_on_error "Cargo format failed."
cargo test --workspace || exit_on_error "Cargo test failed."
cargo build --workspace
