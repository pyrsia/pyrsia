![logo](https://raw.githubusercontent.com/pyrsia/.github/main/images/logo-color.svg)

> Zero-Trust Decentralized Package Network

## Current Development Phase

_📢 We are looking for your feedback!_

This project is currently in the "sandbox" 🏖️. We are actively exploring new concepts and tools.
The code, workflows, and ideas are subject to breaking changes at any time in this early stage of development.

### Primary Focus

To get off the ground the focus is strictly on the peer-to-peer distribution of Docker images backed by a blockchain of identifiers.

## Looking to Contribute?

Take a moment to review our [contributing guidelines](https://github.com/pyrsia/.github/blob/main/contributing.md).
You can join our community on [Slack](https://openssf.slack.com/archives/C02RC7Y5EUV) or participate in a [meeting](https://pyrsia.io/events/) to pick up an issue. We also have our [Local Setup Guide](docs/local_dev_setup.md) to help.

## Install Pyrsia and Joining the Network

There's a web script that will set everything up.

```sh
curl -sS https://pyrsia.io/install.sh | sh
```

For more options and information, checkout our [online tutorial](https://pyrsia.io/guides/userguide/installation-and-support/ubuntu-installation/)

### Downloading Your First Artifact

Let's exercies the [Docker](https://www.docker.com/) and [DockerHub](https://hub.docker.com/) integration.

```sh
docker pull ubuntu
```

### Node and CLI

There are two components of this project

-   **[CLI](pyrsia_cli/)**: A basic interface which communicates with a node.
-   **[Node](pyrsia_node/)**: An instance of the Pyrsia daemon which can participate in the network with other nodes.

### Connecting with other Nodes

The Pyrsia node will always join the "main net" and connect with other peers. You can see this using the CLI's "status" command:

```sh
$ ./pyrsia node -s
Connected Peers Count:   17 # Shows the number of visible peers
Artifacts Count:         12 # Total number of artifacts cached locally
Total Disk Available:    10.5 GB
```
