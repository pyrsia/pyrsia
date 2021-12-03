# Pyrsia CLI

Parse command line commands, subcommands, communnicates to a [Node](../pyrsia_node) to perform some actions.

## Building

1. Follow our [Getting Started](../readme.md#getting-started) section
2. Run build release command for package `pyrsia_cli` which will generate executable in `./target/release/pyrsia`

```sh
cargo build -p pyrsia_cli --release
```

## Installing

Copy-paste above generated executable in some folder and put that in your `PATH`

OR

run install command which will install the CLI in `~/.cargo/bin`, make sure that is included in your `PATH`

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
