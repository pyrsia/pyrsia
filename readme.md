# Pyrsia

[logo (broken link since we are private)](https://raw.githubusercontent.com/pyrsia/.github/main/images/logo-color.svg)

> Zero-Trust Decentralized Package Network

## Current Development Phase

_📢 We are looking for your feedback!_

This project is currently in the "sandbox" 🏖️. We are actively exploring the concepts and tools that are used during development.
The code, workflows, and ideas are subject to breaking changes at any time in this early stage of development.

### Primary Focus

To get off the ground the focus is strictly on the peer-to-peer distribution of Docker images backed by a blockchain of identifiers.

## Contributing

Before getting started, take a moment to review our [contributing guidelines](https://github.com/pyrsia/.github/blob/main/contributing.md).

## Pyrsia Node

### Getting Started

1. Setup rust on your local machine as described in [Rust's getting started guide](https://www.rust-lang.org/learn/get-started)
2. `cd pyrsia-node`
3. `cargo run`

### Generating Test Coverage Report
1. `cd pyrsia-node`
2. `docker build -t code_coverage:1.0 .`
3. `docker run --security-opt seccomp=unconfined -it code_coverage:1.0 cargo tarpaulin -v`


### Setting Up Visual Studio Code Debugger

[How to Debug Rust with Visual Studio Code](https://www.forrestthewoods.com/blog/how-to-debug-rust-with-visual-studio-code/)

### Running the docker integraion:

1. open a terminal and start a pyrsia node with: `RUST_LOG=pyrsia cargo run -q`
2. open a second terminal:
   * pull the alpine docker image from docker hub: `docker pull alpine`
   * tag it to prepare for push to pyrsia node: `docker tag alpine localhost:7878/alpine`
   * push it to pyrsia node: `docker push localhost:7878/alpine`
   * remove all local alpine images: `docker rmi alpine and docker rmi localhost:7878/alpine`
   * pull the image again, this time from pyrsia node: `docker pull localhost:7878/alpine`
   * verify it works: `docker run -it localhost:7878/alpine cat /etc/issue`
