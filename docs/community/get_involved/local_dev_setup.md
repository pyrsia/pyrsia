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

## Build code and run tests

The base line is making sure all the code compiles and every test passes.

```sh
cd $PYRSIA_HOME
cargo build --all-targets
cargo test --workspace
```

These commands should run successfully. If you have issues with these please reach out to the team on slack and report an issue/submit a PR.

## Pyrsia node docker image

Make sure [Docker engine](https://docs.docker.com/engine/install/) is installed (18.09 or higher) and running.

### Build the Pyrsia node docker image

```sh
cd $PYRSIA_HOME
DOCKER_BUILDKIT=1 docker build -t=pyrsia/node .
```

If everything works as expected, after a while, a new image "pyrsia/node" should appear in the docker images list.

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
   DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 7888

   # RUST_LOG=debug DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 7888 # Use this environment variable if you would like to see debug logs
   ```

Test the pyrsia_node status using `curl` (notice the port number for Node 1)

```sh
curl --location --request GET 'http://localhost:7888/status'
```

In a real life deployment these nodes will be spread over the network and will all run on their own 7888 port.

- Node 2:

   ```sh
   DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 7889

   # RUST_LOG=debug DEV_MODE=on PYRSIA_ARTIFACT_PATH=pyrsia cargo run --bin pyrsia_node -- -p 8181 # Use this environment variable if you would like to see debug logs
   ```

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
    ./pyrsia config --add
    ```

   OR place the config file in these OS specific locations:

   Mac:  $HOME/Library/Preferences/rs.pyrsia-cli/pyrsia-cli.toml
   Linux: $HOME/.config/rs.pyrsia-cli/pyrsia-cli.toml or $XDG_CONFIG_HOME/rs.pyrsia-cli/pyrsia-cli.toml
   Windows: %APPDATA%\\Roaming\\pyrsia-cli\\config\\pyrsia-cli.toml

3. Ping the Pyrsia node and list the status

    ```sh
    $ ./pyrsia ping
    Connection Successfull !! {}
    ```

    ```sh
    $ ./pyrsia -s
    Connected Peers Count:       1
    Artifacts Count:             3 {"manifests": 1, "blobs": 2}
    Total Disk Space Allocated:  5.84 GB
    Disk Space Used:             0.0002%
    ```

If you see a status message similar to:

```text
Error: error sending request for url (http://localhost:7888/v2): error trying to connect: tcp connect error: Connection refused (os error 111)
```

then your node is likely not running. Go back to step 3 to make sure the Pyrsia Node can be started.

Congratulations! You have now setup your developer environment and are ready to write code and submit a PR to Pyrsia. Head over to [contributing guidelines](/docs/community/get_involved/contributing/) to start contributing to the project.

> ⚠️ Word of caution: Running the peers for a few hours does generate network traffic and hence can drain your computer power. Ensure you are plugged into power if you are running multiple peers for a long time`
