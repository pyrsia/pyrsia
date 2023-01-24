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

Go to the [build pipeline prototype repo](https://github.com/tiainen/pyrsia_build_pipeline_prototype.git). Checkout to
you local machine and run `cargo run` inside the root directory of the repo.

## Terminal D

First authorize node A

```sh
NODE_A_PEER_ID=`curl -s http://localhost:7881/status | jq -r .peer_id`
echo $NODE_A_PEER_ID
./target/debug/pyrsia config -e --port 7881
./target/debug/pyrsia authorize --peer $NODE_A_PEER_ID
```

Expect similar logs from Node A

```text
2023-01-11T07:56:20.695Z DEBUG pyrsia::blockchain_service::service  > Blockchain sends broadcast block #1: Block { header: Header { parent_hash: HashDigest { multihash: Multihash { code: 27, size: 32, digest: [172, 78, 84, 188, 64, 66, 46, 173, 247, 208, 165, 56, 26, 194, 243, 226, 209, 61, 41, 221, 64, 174, 144, 222, 229, 9, 85, 92, 202, 250, 252, 28, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] } }, transactions_hash: HashDigest { multihash: Multihash { code: 27, size: 32, digest: [50, 13, 132, 67, 147, 75, 242, 24, 8, 144, 82, 195, 131, 46, 124, 74, 82, 216, 176, 37, 66, 255, 215, 244, 201, 137, 175, 143, 51, 82, 77, 90, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] } }, committer: Address { peer_id: Multihash { code: 0, size: 36, digest: [8, 1, 18, 32, 174, 37, 57, 117, 115, 116, 147, 66, 114, 16, 228, 188, 148, 111, 179, 161, 254, 14, 9, 164, 221, 248, 146, 17, 194, 32, 42, 75, 225, 184, 49, 120, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] } }, timestamp: 1673423780, ordinal: 1, nonce: 142300474092487816405627126037197891864, hash: HashDigest { multihash: Multihash { code: 27, size: 32, digest: [140, 66, 242, 68, 112, 117, 151, 217, 160, 108, 99, 72, 137, 230, 141, 229, 74, 37, 86, 109, 216, 145, 190, 171, 52, 131, 87, 250, 68, 161, 207, 242, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] } } }, transactions: [Transaction { type_id: Create, submitter: Address { peer_id: Multihash { code: 0, size: 36, digest: [8, 1, 18, 32, 174, 37, 57, 117, 115, 116, 147, 66, 114, 16, 228, 188, 148, 111, 179, 161, 254, 14, 9, 164, 221, 248, 146, 17, 194, 32, 42, 75, 225, 184, 49, 120, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] } }, timestamp: 1673423780, payload: [123, 34, 105, 100, 34, 58, 34, 101, 52, 51, 54, 53, 51, 56, 56, 45, 99, 102, 100, 100, 45, 52, 102, 56, 57, 45, 97, 57, 52, 53, 45, 50, 100, 53, 98, 49, 101, 49, 102, 55, 97, 101, 57, 34, 44, 34, 112, 97, 99, 107, 97, 103, 101, 95, 116, 121, 112, 101, 34, 58, 110, 117, 108, 108, 44, 34, 112, 97, 99, 107, 97, 103, 101, 95, 115, 112, 101, 99, 105, 102, 105, 99, 95, 105, 100, 34, 58, 34, 34, 44, 34, 110, 117, 109, 95, 97, 114, 116, 105, 102, 97, 99, 116, 115, 34, 58, 48, 44, 34, 112, 97, 99, 107, 97, 103, 101, 95, 115, 112, 101, 99, 105, 102, 105, 99, 95, 97, 114, 116, 105, 102, 97, 99, 116, 95, 105, 100, 34, 58, 34, 34, 44, 34, 97, 114, 116, 105, 102, 97, 99, 116, 95, 104, 97, 115, 104, 34, 58, 34, 34, 44, 34, 115, 111, 117, 114, 99, 101, 95, 104, 97, 115, 104, 34, 58, 34, 34, 44, 34, 97, 114, 116, 105, 102, 97, 99, 116, 95, 105, 100, 34, 58, 34, 34, 44, 34, 115, 111, 117, 114, 99, 101, 95, 105, 100, 34, 58, 34, 34, 44, 34, 116, 105, 109, 101, 115, 116, 97, 109, 112, 34, 58, 49, 54, 55, 51, 52, 50, 51, 55, 56, 48, 44, 34, 111, 112, 101, 114, 97, 116, 105, 111, 110, 34, 58, 34, 65, 100, 100, 78, 111, 100, 101, 34, 44, 34, 110, 111, 100, 101, 95, 105, 100, 34, 58, 34, 49, 50, 68, 51, 75, 111, 111, 87, 77, 89, 65, 50, 103, 67, 109, 69, 90, 99, 53, 117, 68, 85, 119, 116, 81, 115, 49, 51, 70, 85, 103, 104, 99, 119, 97, 111, 57, 106, 57, 107, 84, 74, 115, 54, 50, 75, 80, 106, 82, 109, 69, 119, 34, 44, 34, 110, 111, 100, 101, 95, 112, 117, 98, 108, 105, 99, 95, 107, 101, 121, 34, 58, 34, 98, 48, 100, 49, 51, 53, 51, 99, 45, 102, 53, 102, 51, 45, 52, 54, 100, 99, 45, 56, 98, 55, 97, 45, 52, 100, 49, 54, 56, 52, 56, 53, 97, 50, 50, 50, 34, 125], nonce: 66265093712336186262573032855735193755, hash: HashDigest { multihash: Multihash { code: 27, size: 32, digest: [31, 44, 109, 232, 217, 227, 4, 249, 66, 180, 174, 141, 193, 47, 248, 249, 162, 207, 14, 57, 164, 40, 172, 188, 251, 64, 4, 39, 1, 167, 190, 202, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0] } }, signature: Signature { signature: ed25519::Signature(0B5CB1CA19952CFC17A88B405FB357317A16F0E097F63481B86DE728F1A4DC37DC9FF90A8DA3C69270941663AF4D8CE8593C5C32DDE6ADBF8C62F54A6D929C0B) } }], block_signature: BlockSignature { signature: Signature { signature: ed25519::Signature(8F4E2E5E3D8C0129BF2080D2155947FE7145F7BC43D5876CA76B84FA257028197298CB1A94517533708CAB69FC80F5B821BE5A36219FAD0395F6A79F8D0ECB0C) }, public_key: [174, 37, 57, 117, 115, 116, 147, 66, 114, 16, 228, 188, 148, 111, 179, 161, 254, 14, 9, 164, 221, 248, 146, 17, 194, 32, 42, 75, 225, 184, 49, 120] } }
2023-01-11T07:56:20.697Z DEBUG pyrsia::network::client              > p2p::Client::broadcast_block sent
2023-01-11T07:56:20.703Z DEBUG pyrsia::transparency_log::log        > Transparency log inserted into database with id: e4365388-cfdd-4f89-a945-2d5b1e1f7ae9
2023-01-11T07:56:20.703Z INFO  pyrsia_registry                      > 127.0.0.1:63060 "POST /authorized_node HTTP/1.1" 201 "-" "-" 62.023002ms
```

Then configure to use node B from now on:

```sh
./target/debug/pyrsia config -e --port 7882
```

Then trigger a build:

```sh
./target/debug/pyrsia build docker --image alpine:3.16.0
```

Expect similar logs from Build Prototype service

```text
Requesting build of Docker for alpine:3.16.0
Starting build with ID 54d72bb8-7cd6-4300-a669-467e4375cf4c
#######################################################
#
# Starting Docker build for:
#   alpine:3.16.0
#
#######################################################
  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                 Dload  Upload   Total   Spent    Left  Speed
100  4395    0  4395    0     0  16717      0 --:--:-- --:--:-- --:--:-- 17167
  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                 Dload  Upload   Total   Spent    Left  Speed
100   528  100   528    0     0   1673      0 --:--:-- --:--:-- --:--:--  1708
  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                 Dload  Upload   Total   Spent    Left  Speed
  0     0    0     0    0     0      0      0 --:--:-- --:--:-- --:--:--     0
100  1472  100  1472    0     0   4389      0 --:--:-- --:--:-- --:--:--  4389
  % Total    % Received % Xferd  Average Speed   Time    Time     Time  Current
                                 Dload  Upload   Total   Spent    Left  Speed
  0     0    0     0    0     0      0      0 --:--:-- --:--:-- --:--:--     0
100 2733k  100 2733k    0     0  2165k      0  0:00:01  0:00:01 --:--:-- 3808k
```

And inspect the logs:

```sh
./target/debug/pyrsia inspect-log docker --image alpine:3.16.0
```

... test any of the newly added features ...

## Run Authority Node, Build Pipeline & Agent Node on Docker Desktop

Using the following docker related files with the given contents will enable you to run Authority Node, Build Pipeline
and Agent node in your Docker Desktop

```text
.
├── Dockerfile
└── docker-compose.yml
```

### build_pipeline_prototype/Dockerfile

```dockerfile
FROM rust:1.66.1-buster
ARG RUST_VERSION
RUN apt-get update; \
    apt-get -y install git-all curl clang llvm libclang-dev jq protobuf-compiler;
RUN rustup default ${RUST_VERSION}; \
    git clone https://github.com/tiainen/pyrsia_build_pipeline_prototype.git;
WORKDIR pyrsia_build_pipeline_prototype
```

### docker-compose.yml

```dockerfile
version: "3.9"
networks:
  pyrsia-network:
    external: false
    name: pyrsia-network
services:
  build-pipeline-prototype:
    build:
      context: ./
      dockerfile: Dockerfile
      args:
        - RUST_VERSION=1.66.1
    entrypoint: ["cargo", "run"]
    networks:
      - pyrsia-network
    ports:
      - "8080:8080"
    container_name: build-pipeline-prototype
  authorize-node:
    image: pyrsiaoss/pyrsia-node:0.2.3-2805
    environment:
      RUST_LOG: "info,pyrsia=debug"
    entrypoint: [ "pyrsia_node", "--pipeline-service-endpoint", "http://build-pipeline-prototype:8080", "--listen-only", "-H", "0.0.0.0", "-p", "7881", "--listen", "/ip4/0.0.0.0/tcp/44001", "--init-blockchain" ]
    networks:
      - pyrsia-network
    ports:
      - "7881:7881"
      - "44001:44001"
    container_name: authorize-node
  agent-node:
    image: pyrsiaoss/pyrsia-node:0.2.3-2805
    environment:
      RUST_LOG: "info,pyrsia=debug"
    entrypoint: [ "pyrsia_node", "--bootstrap-url", "http://authorize-node:7881/status", "-H", "0.0.0.0", "-p", "7882", "--listen", "/ip4/0.0.0.0/tcp/44002" ]
#    The following entrypoint is to connect peer node to existing authorize node.
#    entrypoint: [ "pyrsia_node", "--bootstrap-url", "http://boot.pyrsia.link/status", "-H", "0.0.0.0", "-p", "7882", "--listen", "/ip4/0.0.0.0/tcp/44002" ]
    networks:
      - pyrsia-network
    ports:
      - "7882:7882"
      - "44002:44002"
    container_name: agent-node
```

Use `docker compose up` to bring up all 3 containers (i.e. Build Pipeline, Authority Node, Agent Node) and can perform
required test from agent node by logging into `agent-node` container.
