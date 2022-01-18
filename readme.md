![logo](https://raw.githubusercontent.com/pyrsia/.github/main/images/logo-color.svg)

> Zero-Trust Decentralized Package Network

## Current Development Phase

_📢 We are looking for your feedback!_

This project is currently in the "sandbox" 🏖️. We are actively exploring new concepts and tools.
The code, workflows, and ideas are subject to breaking changes at any time in this early stage of development.

### Primary Focus

To get off the ground the focus is strictly on the peer-to-peer distribution of Docker images backed by a blockchain of identifiers.

## Contributing

Before getting started, take a moment to review our [contributing guidelines](https://github.com/pyrsia/.github/blob/main/contributing.md).

## Node and CLI

There are two components of this project

- **[CLI](pyrsia_cli/)**: A basic interface which communicates with a node.
- **[Node](pyrsia_node/)**: An instance of the Pyrsia daemon which can participate in the network with other nodes.

### Getting Started

1. Setup rust on your local machine as described in [Rust's getting started guide](https://www.rust-lang.org/learn/get-started)
2. `cd pyrsia_node`
3. `cargo run`

### Setting Up Visual Studio Code Debugger

[How to Debug Rust with Visual Studio Code](https://www.forrestthewoods.com/blog/how-to-debug-rust-with-visual-studio-code/)

### Building and running the Pyrsia Node with Docker

1. Install [Docker](https://www.docker.com/get-started)
2. Run `docker compose up`
    * macOS and Windows: Compose is included in Docker Desktop
    * Linux: [Downloaded Compose](https://github.com/docker/compose#linux)

The Pyrsia node will then be running on localhost:7878 both on the host and 
inside the VM, available to Docker Engine, in the case of Docker Desktop.
