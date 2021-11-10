# Pyrsia Sandbox

This is a sandbox project for Pyrsia to become familiar with the concepts and tools that are used during development.

## Pyrsia Node

### Getting started

1. setup rust on your local machine as described in the [Rust getting started guide](https://www.rust-lang.org/learn/get-started)
2. cd pyrsia-node
3. cargo run

### Running the docker integraion:

1. open a terminal and start a pyrsia node with: RUST_LOG=pyrsia cargo run -q

2. open a second terminal:
* pull the alpine docker image from docker hub: docker pull alpine
* tag it to prepare for push to pyrsia node: docker tag alpine localhost:7878/alpine
* push it to pyrsia node: docker push localhost:7878/alpine
* remove all local alpine images: docker rmi alpine and docker rmi localhost:7878/alpine
* pull the image again, this time from pyrsia node: docker pull localhost:7878/alpine
* verify it works: docker run -it localhost:7878/alpine cat /etc/issue
