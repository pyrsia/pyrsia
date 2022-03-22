# Pyrsia Library

Principal set of modules which make up the [Node](../pyrsia_node) and [CLI](../pyrsia_cli) components.

## Terminology

- **Artifact Manager**: A component of the node responsible for tracking software components and other artifacts on the local machine and finding it on other nodes
- **Metadata**: The information surrounding and describing the "data"
- **Network**: Refers to the distributed topology of Nodes connected over the internet
- **Node API**: Contains all the RESTful endpoints which allow integration with Pyrsia, primarily for the CLI

## Sub-Crates

- **[signed](signed/)**: Algorithms and notion of a signature
- **[signed_struct](signed_struct/)**: Procedural macro to make any `struct` sign-able
- **[blockchain](blockchain/)**: Encompases the network communication and consensus engine along with the datastructures that make a distrubte ledge possible

## Modules

- TODO
