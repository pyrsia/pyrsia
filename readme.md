# Welcome to Pyrsia

![logo](https://raw.githubusercontent.com/pyrsia/.github/main/images/logo-color.svg)

> Decentralized Package Network

## Current Development Phase

_📢 We are looking for your feedback!_

This project is currently in "early alpha". We are actively building on our minimal viable product which will continue
to evolve over time as we add new features and support more workflows.

Have a use case or workflow you would like to see supported? Open an issue or share on [Slack](https://cdeliveryfdn.slack.com/join/shared_invite/zt-1eryue9cw-9YpgrfIfsTcDS~hGHchURg).
Check out our [Get Involved](/docs/get_involved/) page for more ways to connect.

### Primary Focus

To get off the ground the focus is strictly on the peer-to-peer distribution of Docker images backed by a blockchain of identifiers.

## Looking to Contribute

Take a moment to review our [contributing guidelines](https://github.com/pyrsia/.github/blob/main/contributing.md).
You can join our community on [Slack](https://cdeliveryfdn.slack.com/join/shared_invite/zt-1eryue9cw-9YpgrfIfsTcDS~hGHchURg) or participate in a [meeting](https://pyrsia.io/docs/social/#calendar) to pick up an issue. We also have our [Local Setup Guide](/docs/community/get_involved/local_dev_setup/) to help.

## Install Pyrsia and Join the Network

There are mutiple options to run Pyrsia:

- [Build Pyrsia from source](/docs/community/get_involved/local_dev_setup.md).

- [Use a pre-built installer](/docs/tutorials/quick-installation/)

- [Run Pyrsia inside Docker](/docs/tutorials/quick-installation/#run-pyrsia-in-docker)

Once you have a `pyrsia_node` binary, just run it like this:

```shell
pyrsia_node
```

Optionally setting an environment variable `RUST_LOG=debug` first if you want to
see debug output.

### Downloading Your First Artifact

Let's exercise the [Docker](https://www.docker.com/) integration.

Configure your Docker installation to use Pyrsia as a registry mirror.

On Windows or macOS, open your Docker Desktop -> Settings ->
Docker Engine where Docker allows you to set registry-mirrors. Configure your node
as a registry mirror by adding/editing the following in the configuration:

```jsonc
 "registry-mirrors": [
   "http://0.0.0.0:7888"
 ]
```

On Linux, you'll find this configuration in the file `/etc/docker/daemon.json`.

See [this page](/docs/tutorials/docker/#configure-docker) for more information about
configuring Docker.

Let's try to pull an artifact from the Pyrsia network, but first make sure it is
not yet in your local Docker cache:

```sh
docker rmi alpine:3.16.2
```

Then pull the image:

```sh
docker pull alpine:3.16.2
```

Congratulations! The alpine Docker image was now retrieved from the Pyrsia network.
You can verify this in the Pyrsia logs.

### Connecting with other Nodes

The Pyrsia node will always join the Pyrsia network and connect with other peers.
You can see this in the logs or use the CLI's "status" command:

```sh
$ ./pyrsia status
Connected Peers Count:   1
```

### Integration Tests

- **[Repository](https://github.com/pyrsia/pyrsia-integration-tests)**: Pyrsia integration tests git repository.
- **[Test Results](https://github.com/pyrsia/pyrsia-integration-tests/actions/workflows/run-bats-tests.yml)**: Pyrsia integration tests (daily) results.

### Cloud Deployment

Pyrsia nodes can be deployed on the cloud using [pyrsia_node helmcharts](https://artifacthub.io/packages/helm/pyrsia-nightly/pyrsia-node). These nodes will act as the Authority nodes and participate as boot nodes on the network.
