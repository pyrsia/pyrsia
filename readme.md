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
2. `cd pyrsia_node`
3. `cargo run`
4. **configure Docker** to use Pyrsia, which is running on localhost port 7878\
    open your Docker daemon settings and add this entry in the root object:
    ```
      "registry-mirrors": ["http://localhost:7878"]
    ```    
    On linux you can usually find the Docker daemon settings here `/etc/docker/daemon.json`. On other platforms you can find them in Docker Desktop -> Preferences -> Docker Engine.

    **Important note**: If you're not running on Linux, please see the note on configuring Docker on other platforms below

5. using another terminal, use `docker` to pull an image from Pyrsia: 
    ```
    docker pull ubuntu
    ```
   (or pull any other Docker image of your choice)

    Optionally, you can inspect the Pyrsia node logs to check where the image came from. This can be either: 
    - locally (if it was cached by Pyrsia before)
    - from the Pyrsia network
    - or from Docker Hub (if it wasn't previously available in the Pyrsia network)



**Configuring Docker on other platforms**

If you're not running on Linux, you cannot use `localhost` because that will point to the Docker Desktop VM. Instead you have to use the hostname of your host machine. If you don't know/have that, you can add this to `/etc/hosts` (on Mac) or `c:\windows\system32\drivers\etc\hosts` (on Windows):

```
127.0.0.1       my-pyrsia-host
```

And then use that name in the Docker configuration file like this:
```
"registry-mirrors": ["http://my-pyrsia-host:7878"]
```


### Setting Up Visual Studio Code Debugger

[How to Debug Rust with Visual Studio Code](https://www.forrestthewoods.com/blog/how-to-debug-rust-with-visual-studio-code/)

### Building and running the Pyrsia Node with Docker

1. Install [Docker](https://www.docker.com/get-started)
2. Run `docker compose up`
    * macOS and Windows: Compose is included in Docker Desktop
    * Linux: [Downloaded Compose](https://github.com/docker/compose#linux)

The Pyrsia node will then be running on localhost:7878 both on the host and 
inside the VM, available to Docker Engine, in the case of Docker Desktop.
