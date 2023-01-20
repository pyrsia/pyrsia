# Setup your development environment

Pyrsia uses [Rust programming language](https://www.rust-lang.org/) and will require you to setup Rust and it's dependencies.

## Download the Source Code

Clone the project repo

```sh
git clone https://github.com/pyrsia/pyrsia.git
```

We will call this directory `$PYRSIA_HOME`

## Install Rust

Setup rust on your local machine as described in [Rust's getting started guide](https://www.rust-lang.org/learn/get-started).
You may also follow [How to Debug Rust with Visual Studio Code](https://www.forrestthewoods.com/blog/how-to-debug-rust-with-visual-studio-code/)
if you are looking to make code changes.

### Install System Dependencies

- Install Clang and OpenSSL
  - macOS: use [homebrew](https://brew.sh/) to install OpenSSL

    ```sh
    brew install openssl@1.1
    ```

  - Linux (ubuntu): use `apt` to install Clang, OpenSSL and CMake and pkg-config

    ```sh
    apt install clang libssl-dev cmake pkg-config
    ```

- Install protoc

  Pyrsia has a dependency on the `libp2p` crate which requires `protoc` to be installed.
  - Mac OS
    If you have [Homebrew](https://brew.sh/), just run:

    ```sh
    brew install protobuf
    ```

    Alternately, run the following commands ([protobuf releases page](https://github.com/protocolbuffers/protobuf/releases)):

    ```sh
    PROTOC_ZIP= protoc-3.14.0-osx-x86_64.zip
    curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v3.14.0/$PROTOC_ZIP
    sudo unzip -o $PROTOC_ZIP -d /usr/local bin/protoc
    sudo unzip -o $PROTOC_ZIP -d /usr/local 'include/*'
    rm -f $PROTOC_ZIP
    ```

  - Linux
    Run the following commands ([protobuf releases page](https://github.com/protocolbuffers/protobuf/releases)):

    ```sh
    PROTOC_ZIP=protoc-3.14.0-linux-x86_64.zip
    curl -OL https://github.com/protocolbuffers/protobuf/releases/download/v3.14.0/$PROTOC_ZIP
    sudo unzip -o $PROTOC_ZIP -d /usr/local bin/protoc
    sudo unzip -o $PROTOC_ZIP -d /usr/local 'include/*'
    rm -f $PROTOC_ZIP
    ```

  For more detail, please read the [protobuf installation guide](https://github.com/protocolbuffers/protobuf#protocol-compiler-installation).

## Build code and run tests

The base line is making sure all the code compiles and every test passes.

```sh
cd $PYRSIA_HOME
cargo build --all-targets
cargo test --workspace
```

These commands should run successfully. If you have issues with these please reach out to the team [on slack](https://app.slack.com/client/TJWP1JXK6/C03U383HU1Z)
and report [an issue](https://github.com/pyrsia/pyrsia/issues)/submit a PR.

You can also check the code coverage.

```sh
cd $PYRSIA_HOME
sh ./tests/test_code_coverage.sh
```

## Pyrsia node docker image

Make sure [Docker engine](https://docs.docker.com/engine/install/) is installed (18.09 or higher) and running.

### Build the Pyrsia node docker image

```sh
cd $PYRSIA_HOME
DOCKER_BUILDKIT=1 docker build -t=pyrsia/node .
```

If everything works as expected, after a while, a new image "pyrsia/node" should appear in the local docker images list.

### Build the Pyrsia node docker image (if not present) and start the node

```sh
cd $PYRSIA_HOME
docker compose up
```

If everything works as expected, a new docker container should be started using the "node/pyrsia" image.

## Simulate a network

Once you have compiled the Pyrsia code you are ready to build a Pyrsia network for testing. Pyrsia nodes are run on a peer to peer network and will require port separation if you would like to run multiple nodes on the same computer.

Follow the instructions below to setup a test network.

- Node 1:

   ```sh
   RUST_LOG=info,pyrsia=debug DEV_MODE=on cargo run --package pyrsia_node -- --listen-only -p 7888 --init-blockchain
   ```

   Test the pyrsia_node status using `curl` (notice the port number for Node 1)

   ```sh
   curl --location --request GET 'http://localhost:7888/status'
   ```

   Download or clone the [prototype repo](https://github.com/tiainen/pyrsia_build_pipeline_prototype)
   and run as follows (`jq` must be installed locally before):

   ```sh
   cd pyrsia_build_pipeline_prototype
   RUST_LOG=debug cargo run
   ```

In a real life deployment these nodes will be spread over the network and will all run on their own 7888 port.

- Node 2:

   ```sh
   RUST_LOG=info,pyrsia=debug DEV_MODE=on PYRSIA_BLOCKCHAIN_PATH=pyrsia_node_2/blockchain PYRSIA_ARTIFACT_PATH=pyrsia_node_2 PYRSIA_KEYPAIR=pyrsia_node_2/p2p_keypair.ser cargo run --package pyrsia_node -- -p 7889 --bootstrap-url http://localhost:7888/status
   ```

   We have to set explicit values for `PYRSIA_BLOCKCHAIN_PATH`, `PYRSIA_ARTIFACT_PATH` and `PYRSIA_KEYPAIR` to prevent
   collisions with the files already created by Node 1. Another way of dealing with this, is to copy the `pyrsia_node`
   binary to a separate location and start the second node there. The default values use a relative path against the current directory.

   Test the pyrsia_node status using `curl` (notice the port number for Node 2)

   ```sh
   curl --location --request GET 'http://localhost:7889/status'
   ```

Now you have confirmed that the individual nodes are running.

## Interact using the CLI

You can use the Pyrsia CLI to ensure that the peers are connected.

1. Build the CLI tool

   ```sh
   cd ../pyrsia_cli
   cargo build
   cd ../target/debug
   ```

2. Configure the CLI tool for your node using interactive subcommand "config"

   ```sh
    ./pyrsia config -e
   ```

   You can find the config file in these OS specific locations:

   - Mac:  $HOME/Library/Preferences/rs.pyrsia-cli/pyrsia-cli.toml
   - Linux: $HOME/.config/rs.pyrsia-cli/pyrsia-cli.toml or $XDG_CONFIG_HOME/rs.pyrsia-cli/pyrsia-cli.toml
   - Windows: %APPDATA%\\Roaming\\pyrsia-cli\\config\\pyrsia-cli.toml

   You can easily switch the CLI to use one of your two nodes by using one of these commands:
   - Node 1:

   ```sh
    ./pyrsia config -e --port 7888
   ```

   - Node 2:

   ```sh
    ./pyrsia config -e --port 7889
   ```

3. Ping the Pyrsia node and list the status

   ```sh
   $ ./pyrsia ping
   Connection Successfull !! {}
   ```

   ```sh
   $ ./pyrsia -s
   Connected Peers Count:       1
   ```

   If you see a status message similar to:

   ```text
   Error: error sending request for url (http://localhost:7888/v2): error trying to connect: tcp connect error: Connection refused (os error 111)
   ```

   then your node is likely not running. Go back to step 3 to make sure the Pyrsia Node can be started.

4. Authorize Node 1 as your build node

    ```sh
    ./pyrsia config -e --port 7888
    NODE1_PEER_ID=`curl -s http://localhost:7888/status | jq -r .peer_id`
    echo Authorizing peer id $NODE1_PEER_ID
    ./pyrsia authorize --peer $NODE1_PEER_ID
    ```

    This will output something like this:

    ```text
    Authorizing peer id 12D3KooWFiC9Xdx77HJSLv6B1muauoxTvjWrVNcUgE4d8YRsRWkT
    Authorize request successfully handled.
    ```

5. Trigger a build from source

    Configure your pyrsia CLI to use either Node 1 (port 7888) or Node 2 (port 7889) and then run this:

    ```sh
    ./pyrsia build docker --image alpine:3.16.0
    ```

    When triggering the build from Node 1, it will use the build pipeline. When triggering the build from Node 2,
    it will send a build request to Node 1, that will use the build pipeline.

6. Inspect logs

    ```sh
    ./pyrsia inspect-log docker --image alpine:3.16.0
    ```

    This will print the transparency logs for alpine:3.16.0 in JSON format.

Congratulations! You have now set up your developer environment and are ready to write code and submit a PR to Pyrsia.
Head over to [contributing guidelines](https://github.com/pyrsia/.github/blob/main/contributing.md) to start contributing to the project.

> ⚠️ Word of caution: Running the peers for a few hours does generate network traffic and hence can drain your computer power. Ensure you are plugged into power if you are running multiple peers for a long time`
