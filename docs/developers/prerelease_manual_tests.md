---
sidebar_position: 20
---

# Pre-release checks

Before a release, make sure to

- [ ] run the integration tests
- [ ] start Pyrsia on every suppored platform
  - Windows
  - Linux
  - MacOS
  - Docker
- [ ] run the following manual checks on a network of local nodes

## Terminal A

```sh
cargo build --workspace
mkdir -p /tmp/pyrsia-manual-tests/nodeA
mkdir -p /tmp/pyrsia-manual-tests/nodeB
cp target/debug/pyrsia_node /tmp/pyrsia-manual-tests/nodeA
cp target/debug/pyrsia_node /tmp/pyrsia-manual-tests/nodeB
cd /tmp/pyrsia-manual-tests/nodeA
RUST_LOG=pyrsia=debug DEV_MODE=on ./pyrsia_node --pipeline-service-endpoint http://localhost:8080  --listen-only -H 0.0.0.0 -p 7881 --init-blockchain
```

## Terminal B

```sh
cd /tmp/pyrsia-manual-tests/nodeB
RUST_LOG=debug ./pyrsia_node --bootstrap-url http://localhost:7881/status -p 7882
```

## Terminal C

Go to the build pipeline prototype repo and run

```sh
cargo run
```

## Terminal D

First authorize node A

```sh
NODE_A_PEER_ID=`curl -s http://localhost:7881/status | jq -r .peer_id`
echo $NODE_A_PEER_ID
./target/debug/pyrsia config -e --port 7881
./target/debug/pyrsia authorize --peer $NODE_A_PEER_ID
```

Then configure to use node B from now on:

```sh
./target/debug/pyrsia config -e --port 7882
```

Then trigger a build:

```sh
./target/debug/pyrsia build docker --image alpine:3.16.0
```

And inspect the logs:

```sh
./target/debug/pyrsia inspect-log docker --image alpine:3.16.0
```

... test any of the newly added features ...
