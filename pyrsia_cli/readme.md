# Pyrsia CLI

parse command line commands, subcommands, communnicates to a [Node](../pyrsia_node) and perform some actions.

## Building

1. setup rust on your local machine as described in the [Rust getting started guide](https://www.rust-lang.org/learn/get-started)
2. Run build release command for package pyrsia_cli which will generate executable in : ./target/release/pyrsia

```sh
cargo build -p pyrsia_cli --release
```

## Installing

Copy-paste above generated executable in some folder and put that in your PATH

OR

run install command which will install cli in "~/.cargo/bin", make sure that is included in path

```sh
cargo install --path .
```

## Usage

```console
pyrsia -h
pyrsia -V
pyrsia config --add
pyrsia config -s
```
