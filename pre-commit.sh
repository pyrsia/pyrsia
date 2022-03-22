#!/usr/bin/env bash
exit_on_error() {
    exit_msg=$1
    if [[ "$exit_msg" != "" ]]; then
        >&2 printf "$exit_msg\n"
        exit 1
    fi
}

printf "Running Pyrsia pre-commit validation.\n"
printf "This might take sometime, please do not interrupt if the screen is blank.\n"
if [[ "$1" == "clean" ]] ; then
	printf "Cleaning old build artifacts.\n"
	cargo clean
fi

cargo install cargo-audit || exit_on_error "Could not install cargo audit."
cargo audit || exit_on_error "Cargo audit failed."
cargo clippy || exit_on_error "Cargo clippy failed."
rustup component add rustfmt || exit_on_error "Could not install rustfmt."
cargo fmt --check || exit_on_error "Cargo format failed."
cargo test --workspace || exit_on_error "Cargo test failed."
cargo build --workspace
