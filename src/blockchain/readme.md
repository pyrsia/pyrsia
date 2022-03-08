# Pyrsia Blockchain Network Crate

This crate provides the "input" and "output" trait for interacting with the network to perform consensus over blocks.

## Requirements

- Track Authorities, may come and go, with no external dependencies
- Immutable record of artifacts

## Philosophy

For the user of this crate, they should be able to:

- recording new "transaction" accepting
  1. generic payload (any struct that implements our trait)
  2. few concrete types for the core funcationality of the blockchain
- returning finalized blocks

This crate will be responsible for:

- definining the network communication (Trait requiremnents most likely)
- performing network communitcation to achieve consensus
- consensus 
  - Proof of Authority (starter may grow)
    - Authorities are any keys which are valid and recorded on the blockchain
  - `get_list_authority()` -- always a valid list as of now
    - some open temporal questions -- do confirmed blocks get invalidated when authorities are revoked?

This crate will **not** provide

- permenat storage of the data (see example for how to save to Disk)
- searching or index the blockchain
  - this will be package specific
  - the user will be able to maintain all the history of a single package how ever it sees fit to do so

## Getting started

This can be built from the project root since it's apart of the workspace.

### Running the example Node

```
cargo build --example simple_node
```
