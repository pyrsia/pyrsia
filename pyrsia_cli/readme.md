# Pyrsia CLI

Parses command lines and subcommands, then communicates the parsed requests to a Pyrsia node.

## Building

1. Follow our [Developer Environment Setup](https://pyrsia.io/docs/developer/local_dev_setup/) section
2. Run the build release command for package `pyrsia_cli` which generates an executable in `./target/release/pyrsia`

```sh
cargo build -p pyrsia_cli --release
```

## Installing

Copy-paste the above generated executable in some folder and add to folder to your `PATH`

OR

Run the install command below from the current `pyrsia_cli/` folder which installs the CLI in `~/.cargo/bin`. Make sure `~/.cargo/bin` is included in your `PATH`.

```sh
cargo install --path .
```

## Usage

```console
pyrsia -h
pyrsia -V
pyrsia ping
pyrsia status or pyrsia -s
pyrsia config --add
pyrsia config -s
```
