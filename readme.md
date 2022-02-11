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

For now Pyrsia only supports Docker artifacts. Follow these steps to run a Pyrsia node and use it as the registry for all Docker Hub content.

1. Setup rust on your local machine as described in [Rust's getting started guide](https://www.rust-lang.org/learn/get-started)
2. Clone this repo `git clone https://github.com/pyrsia/pyrsia.git`
3. `cd pyrsia/pyrsia_node`
4. You need to start the Pyrsia Node. To do so, you have 2 options:
   - regular build: `DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run`
   - build in docker: `docker-compose up --build`

*Note*: Do not to stop this process, a running node is required for the 
following steps:

5. **configure Docker** to use Pyrsia, which is running on localhost port 7888\
    open your Docker daemon settings and add this entry in the root JSON object:

    **On Linux platforms**:

    ```
      "registry-mirrors": ["http://localhost:7888"]
    ```
    By default you can find the Docker daemon settings here `/etc/docker/daemon.json`.

    **On other platforms**:

    You can find the Docker daemon settings in Docker Desktop -> Preferences -> Docker Engine.

    If you're running Pyrsia inside Docker, the `registry-mirrors` settings can be set to `http://localhost:7888` as well. However if you're not running inside Docker, you cannot use `localhost` because localhost will point to the Docker Desktop VM. Instead you have to use the hostname of your host machine. If you don't know/have that, you can add this to `/etc/hosts` (on Mac) or `c:\windows\system32\drivers\etc\hosts` (on Windows):

    ```
    127.0.0.1       my-pyrsia-host
    ```

    And then use that name in the Docker configuration file like this:
    ```
    "registry-mirrors": ["http://my-pyrsia-host:7888"]
    ```

6. using another terminal, use `docker` to pull an image from Pyrsia:
    ```
    docker pull ubuntu
    ```
   (or pull any other Docker image of your choice)

    Optionally, you can inspect the Pyrsia node logs to check where the image came from. This can be either:
    - locally (if it was cached by Pyrsia before)
    - from the Pyrsia network
    - or from Docker Hub (if it wasn't previously available in the Pyrsia network)


7. Build the CLI tool
   ```
   cd ../pyrsia_cli
   cargo build
   cd ../target/debug
   ```

8. Configure the CLI tool
    ```
    ./pyrsia config --add localhost:7888
    ```

9. Ping the Pyrsia node and list the status
    ```
    ./pyrsia node -p
    Connection Successfull !! {}
    ```

    ```
    ./pyrsia node -s
    Connected Peers Count:   0
    Artifacts Count:         12 # reflects the number of artifacts that the pyrsia_node has stored on the network
    Total Disk Available:    983112
    ```

If you see a status message similar to 
   ```
Error: error sending request for url (http://localhost:7888/v2): error trying to connect: tcp connect error: Connection refused (os error 111)
   ```
then your node is not running. Go back to step 3 to make sure
the Pyrsia node is running.

10. Multiple Pyrsia nodes can be started on the same computer by changing the ports they use as follows

    ```
    DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 7888

    # RUST_LOG=debug DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 7888 # Use this environment variable if you would like to see debug logs
    ```

    ```
    DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 8181

    # RUST_LOG=debug DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 8181 # Use this environment variable if you would like to see debug logs
    ```

    ```
    ./pyrsia node -s
    Connected Peers Count:   1 # Shows the additional node that joined the list of peers
    Artifacts Count:         12
    Total Disk Available:    983112
    ```

    In a real life deployment these nodes will be spread over the network and will all run on their own 7888 port.
    `Word of caution: Running the peers for a few hours does generate network traffic and hence can drain your computer power. Ensure you are plugged into power if you are running multiple peers for a long time`



### Setting Up Visual Studio Code Debugger

[How to Debug Rust with Visual Studio Code](https://www.forrestthewoods.com/blog/how-to-debug-rust-with-visual-studio-code/)

### Building and running the Pyrsia Node with Docker

1. Install [Docker](https://www.docker.com/get-started)
2. Run `docker compose up`
    * macOS and Windows: Compose is included in Docker Desktop
    * Linux: [Downloaded Compose](https://github.com/docker/compose#linux)

The Pyrsia node will then be running on localhost:7888 both on the host and
inside the VM, available to Docker Engine, in the case of Docker Desktop.
