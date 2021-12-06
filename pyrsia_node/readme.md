# Pyrsia Node

The daemon running everything.

## Terminology

- **Artifact Manager**: A component of the node responsible for tracking software components and other artifacts on the local machine and finding it on other nodes.
- **Metadata**: The information surrounding and describing the "data"

## Running the docker integration:

1. Open a terminal and start a pyrsia node with: `RUST_LOG=pyrsia cargo run -q`
2. Open a second terminal:
   * pull the alpine docker image from docker hub: `docker pull alpine`
   * tag it to prepare for push to pyrsia node: `docker tag alpine localhost:7878/alpine`
   * push it to pyrsia node: `docker push localhost:7878/alpine`
   * remove all local alpine images: `docker rmi alpine and docker rmi localhost:7878/alpine`
   * pull the image again, this time from pyrsia node: `docker pull localhost:7878/alpine`
   * verify it works: `docker run -it localhost:7878/alpine cat /etc/issue`
