#!/usr/bin/env bash
exit_on_error() {
    exit_msg=$1
    if [[ "$exit_msg" != "" ]]; then
        >&2 printf "$exit_msg\n"
        exit 1
    fi
}

rmi_and_exit_on_error() {
  docker rmi pyrsia/test_node:latest

  exit_msg=$1
  if [[ "$exit_msg" != "" ]]; then
      >&2 printf "$exit_msg\n"
      exit 1
  fi
}

printf "Running Pyrsia pre-commit validation.\n"
printf "This might take sometime, please do not interrupt if the screen is blank.\n"
if [[ "$1" == "clean" || $2 == "clean" ]] ; then
	printf "Cleaning old build artifacts.\n"
	cargo clean
fi

cargo install cargo-audit || exit_on_error "Could not install cargo audit."
cargo audit --ignore RUSTSEC-2022-0040 --ignore RUSTSEC-2020-0071 || exit_on_error "Cargo audit failed."
cargo clippy || exit_on_error "Cargo clippy failed."
cargo fmt --check || exit_on_error "Cargo format failed."
if [[ "$1" == "ignore-it" || $2 == "ignore-it" ]] ; then
    printf "Ignore integration tests.\n"
    cargo test --lib || exit_on_error "Cargo lib test failed."
    cargo test --doc || exit_on_error "Cargo doc test failed."
  else
    printf "Run all tests.\n"
    docker rmi pyrsia/test_node:latest
    DOCKER_BUILDKIT=1 docker build -t=pyrsia/test_node .
    cargo test --workspace || rmi_and_exit_on_error "Cargo test failed."
    docker rmi pyrsia/test_node:latest
fi

cargo build --workspace --all-targets
