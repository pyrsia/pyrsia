# Pyrsia Node

The daemon running everything.

## Terminology

- **Artifact Manager**: A component of the node responsible for tracking software components and other artifacts on the local machine and finding it on other nodes.
- **Metadata**: The information surrounding and describing the "data"

### Generating Test Coverage Report

1. From the root directory of the repository
2. Run `sh ./tests/test_code_coverage.sh`

## Running the Docker integration

1. Open a terminal and start a pyrsia node with: `RUST_LOG=pyrsia cargo run -q`
2. Open a second terminal:
   - pull the alpine docker image from docker hub: `docker pull alpine`
   - tag it to prepare for push to pyrsia node: `docker tag alpine localhost:7878/alpine`
   - push it to pyrsia node: `docker push localhost:7878/alpine`
   - remove all local alpine images: `docker rmi alpine and docker rmi localhost:7878/alpine`
   - pull the image again, this time from pyrsia node: `docker pull localhost:7878/alpine`
   - verify it works: `docker run -it localhost:7878/alpine cat /etc/issue`

### Manually interacting with Docker API

1. Open a terminal and start a pyrsia node with: `RUST_LOG=pyrsia cargo run -q`
2. Start 2 more nodes with different ports by adding `-p ####` to the command above
3. Try running the following commands:

   ```sh
   $ curl -X POST "http://localhost:7878/v2/hello/blobs/uploads"
   TRACE pyrsia_node::docker::v2::handlers::blobs    > Getting ready to start new upload for hello - 0dc2f7e1-d943-481e-93a8-227c4909c632
   $ curl "http://localhost:7878/v2/hello/blobs/ab2b79d4-45dd-462f-a1bf-8b863944156e"
   DEBUG pyrsia_node::docker::error_util             > ErrorMessage: ErrorMessage { code: BlobDoesNotExist("445e800d-3da0-4d7f-8644-e590931f526d"), message: "" }
   ```
