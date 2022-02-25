# Getting Started

For now Pyrsia only supports Docker artifacts.

## Install Dependencies

### Building with Rust

Setup rust on your local machine as described in [Rust's getting started guide](https://www.rust-lang.org/learn/get-started).
You may also follow [How to Debug Rust with Visual Studio Code](https://www.forrestthewoods.com/blog/how-to-debug-rust-with-visual-studio-code/)
if you are loking to make code changes.

### Integrating and Building with Docker

- Install [Docker](https://www.docker.com/get-started)
    * macOS and Windows: Compose is included in Docker Desktop
    * Linux: [Downloaded Compose](https://github.com/docker/compose#linux)

The Pyrsia Node will then be running on `http://localhost:7888`.

## Obtain the Source Code

- Clone this repo 

```sh
git clone https://github.com/pyrsia/pyrsia.git
```

## Configure Docker Daemon for Pyrsia Network

Follow these steps to run a Pyrsia node and use it as the registry for all Docker Hub content.

1. `cd pyrsia/pyrsia_node`
2. You need to start the Pyrsia Node. To do so, you have 2 options:
   - Rust: `DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run`
   - Docker Compose: `docker-compose up --build`

*Note*: Do not to stop this process, a running node is required for the 
following steps:

3. **configure Docker** to use Pyrsia, which is running at `http://localhost:7888`,
    open your Docker daemon settings and add this entry in the root JSON object:

    **On Linux platforms**:

    ```
      "registry-mirrors": ["http://localhost:7888"]
    ```

    By default you can find the Docker daemon settings here `/etc/docker/daemon.json`.

    **On other platforms**:

    You can find the Docker daemon settings in Docker Desktop -> Preferences -> Docker Engine.

    If you're running Pyrsia inside Docker, the `registry-mirrors` settings can be set to `http://localhost:7888` as well. However if you're not running inside Docker, you cannot use `localhost` because localhost will point to the Docker Desktop enviroment. Instead you have to use the hostname of your host machine. If you don't know/have that, you can add this to `/etc/hosts` (on Mac) or `c:\windows\system32\drivers\etc\hosts` (on Windows):

    ```
    127.0.0.1       my-pyrsia-host
    ```

    And then use that name in the Docker configuration file like this:
    ```
    "registry-mirrors": ["http://my-pyrsia-host:7888"]
    ```

7. using another terminal, use `docker` to pull an image from Pyrsia:

    ```
    docker pull ubuntu
    ```
   (or pull any other Docker image of your choice)

    Optionally, you can inspect the Pyrsia node logs to check where the image came from. This can be either:

    - locally (if it was cached by Pyrsia before)
    - from the Pyrsia network
    - or from Docker Hub (if it wasn't previously available in the Pyrsia network)

## Using the CLI

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

    ```sh
    $ ./pyrsia node -s
    Connected Peers Count:   0
    Artifacts Count:         12 # reflects the number of artifacts that the pyrsia_node has stored on the network
    Total Disk Available:    983112
    ```

If you see a status message similar to:

```
Error: error sending request for url (http://localhost:7888/v2): error trying to connect: tcp connect error: Connection refused (os error 111)
```

then your node is likely not running. Go back to step 3 to make sure the Pyrsia Node can be started.

## Simulating a network

Multiple Pyrsia Nodes can be started on the same computer by changing the ports they use as follows


- Node 1:

   ```
   DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 7888

   # RUST_LOG=debug DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 7888 # Use this environment variable if you would like to see debug logs
   ```

- Node 2:

   ```
   DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 8181

   # RUST_LOG=debug DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 8181 # Use this environment variable if you would like to see debug logs
   ```

Re-running the status command, there should be an connect peer.

```sh 
$ ./pyrsia node -s
Connected Peers Count:   1 # Shows the additional node that joined the list of peers
Artifacts Count:         12
Total Disk Available:    983112
```

In a real life deployment these nodes will be spread over the network and will all run on their own 7888 port.

> ⚠️ Word of caution: Running the peers for a few hours does generate network traffic and hence can drain your computer power. Ensure you are plugged into power if you are running multiple peers for a long time`

## Testing a Node directly

To test the pyrsia_node status you can use `curl`  and

```
curl --location --request GET 'http://localhost:7888/status'
```
