---
sidebar_position: 1
---

# Pyrsia demo: build Docker images from source

> **Warning:** The build-from-source demo is still work-in-progress.

This tutorial describes how to setup two Pyrsia nodes: one that acts as the authorized
node and builds Docker images from source and makes them available in the Pyrsia network,
and another one that acts as a regular Pyrsia node, retrieving the transparency
logs and the Docker image from the Pyrsia network. \

> Note: in the current prototype, the build pipeline does not actually build the
image, but downloads them from Docker Hub instead.

The following scenario will be used:

- Setup an 'authorized' node: node A
- Setup a regular node: node B
- Set up a build pipeline (prototype) for node A and configure it to use it.
- Trigger a build from source for a given artifact
- Wait for the build to finish in the build pipeline
- Try to reach consensus with the other authorized nodes, which have to run the
  same build and verify they produce the same build result. (There's only one
  authorized node, so this is 'light' consensus for now)
- Create a transparency log about the artifact publication
- Publish the artifact on the p2p network
- Inspect the transparency log on any node
- Use docker pull using node B to pull the docker image from the Pyrsia network

```mermaid
sequenceDiagram
    participant User
    participant nodeB as Node B
    participant nodeA as Node A
    participant Build pipeline
    User->>nodeA: Trigger build
    nodeA->>Build pipeline: Request build
    nodeA->>nodeA: Wait for build to finish
    nodeA->>Build pipeline: Fetch build result
    nodeA->>nodeA: Reach consensus and<br>create transparency log
    nodeA->>nodeB: Distribute new logs
    User->>nodeB: docker pull
    nodeB->>nodeB: Check transparency logs
    nodeB->>nodeA: Fetch artifacts
    nodeB->>User: return docker image
```

See the [architecture and use-cases](pyrsia-architecture-and-use-cases.md)
document for more information.

## Compile Pyrsia

Download a fresh copy of the codebase by cloning the repo or updating to the
HEAD of `main`.

```sh
git clone https://github.com/pyrsia/pyrsia.git
```

Let's call this folder `PYRSIA_HOME`. We will refer to this
name in the following steps.

Build binaries by running:

```sh
cd $PYRSIA_HOME
cargo build --workspace
```

See the [Development Environment](../community/get_involved/local_dev_setup.md)
document for more information.

## Run Pyrsia node A

Now we will set the following env vars and start a pyrsia node:

- RUST_LOG: to make sure we can see all the debug logs
- DEV_MODE: to make sure all non-existing directories are created on-the-fly
- PYRSIA_ARTIFACT_PATH: pointing to a directory to store artifacts. optionally
  remove this directory prior to starting Pyrsia if you want to start from an
  empty state.

For the purpose of this demo, let's create temporary directories to clearly
separate our two nodes:

```sh
mkdir nodeA
cp target/debug/pyrsia_node nodeA
cd nodeA
```

And then run node A in listen-only and init mode, listening on a non-default port (because
we will run node B with default settings).

```sh
RUST_LOG=pyrsia=debug DEV_MODE=on \
./pyrsia_node --pipeline-service-endpoint http://localhost:8080 -p 7889 --listen-only --init-blockchain
```

Watch out for this kind of log:

```text
INFO  pyrsia::network::event_loop > Local node is listening on "/ip4/127.0.0.1/tcp/56662/p2p/12D3KooWBgWeXNT1EKXo2omRhZVmkbvPgzZ5BcGjTfgKr586BSAn"
```

It contains the p2p multiaddress of node A, which we will need when starting node
B later in this tutorial.

As you can see, we specified the `--pipeline-service-endpoint` argument to point
to `http://localhost:8080`, which is where we will run our build pipeline prototype
(see below). In a production setup, the build pipeline needs to run on its
own isolated infrastructure.

## Run build pipeline prototype

In this demo, we use a build pipeline prototype. Eventually, Pyrsia will use
industry-standard build pipeline solutions. Integration with such solutions is
currently being investigated.

The most important features of the build pipeline are:

- it runs completely separate from the Pyrsia node
- it exposes an interface so Pyrsia can start a build
- it exposes an interface so Pyrsia can download the build output

The current build pipeline prototype supports Maven and Docker builds.

Download or clone the [prototype repo](https://github.com/tiainen/pyrsia_build_pipeline_prototype)
and run as follows (`jq` must be installed locally before):

```sh
cd pyrsia_build_pipeline_prototype
RUST_LOG=debug cargo run
```

By default, this prototype listens on http port 8080. If you run it on a different
host or port, make sure to specify its location when starting the Pyrsia node
with `--pipeline-service-endpoint` (see above).

You will see the following output indicating that the build pipeline is ready
for use

```text
   Finished dev [unoptimized + debuginfo] target(s) in 1m 07s
     Running `target/debug/pyrsia_build`
 INFO  actix_server::builder > Starting 8 workers
 INFO  actix_server::server  > Tokio runtime found; starting in existing Tokio runtime
```

## Authorize node A as a build node

We will use the Pyrsia CLI to authorize node A as a build node.
Since node A is running on port 7889, we will have to edit the Pyrsia CLI config:

In a new terminal, while the Pyrsia nodes and the build pipeline prototype are
running, run:

```sh
 ./pyrsia config --edit
 ```

And enter the correct values:

```text
Enter host:
localhost
Enter port:
7889
Enter disk space to be allocated to pyrsia(Please enter with units ex: 10 GB):
10GB

Node configuration Saved !!
```

Next you'll need to find out the peer id of node A. You can see that in its logs
or you can query the `/status` endpoint like this: (assuming you have `jq` installed)

```shell
curl -s http://localhost:7888/status | jq  .peer_id
```

Once you know the peer id, authorize it like this:

```shell
./pyrsia authorize --peer <PEER_ID>
```

## Run Pyrsia node B

Now it's time to run our regular node: node B. Let's create another temporary
directory to clearly separate it from node A.

```sh
mkdir nodeB
cp target/debug/pyrsia_node nodeB
cd nodeB
```

And then run node B with default settings and connecting it to the multiaddress
of node A. This multiaddress can be found in the logs of node A (see section
"Run Pyrsia node A" above).

```sh
RUST_LOG=pyrsia=debug DEV_MODE=on \
./pyrsia_node -H 0.0.0.0 --peer /ip4/127.0.0.1/tcp/56662/p2p/12D3KooWBgWeXNT1EKXo2omRhZVmkbvPgzZ5BcGjTfgKr586BSAn
```

**Important**: do not simply copy/paste this command, the multiaddress on your
local system will be different.

At this point, we are running a Pyrsia network consisting of two nodes, so
let's continue building an artifact and providing it on the network.

## Trigger a build from source for a given artifact

In this section we will trigger a build for `alpine:3.16` on node A.

We will use the Pyrsia CLI to trigger a build from source. We can send the build
request to node B, which will relay the request to node A, which is an authorized
build node. Node B, which is running on port 7888, we will have to edit this config
again:

In a new terminal, while the Pyrsia nodes and the build pipeline prototype are
running, run:

```sh
 ./pyrsia config --edit
 ```

And enter the correct values:

```text
Enter host:
localhost
Enter port:
7888
Enter disk space to be allocated to pyrsia(Please enter with units ex: 10 GB):
10 GB

Node configuration Saved !!
```

Then trigger the build from source, like this:

```sh
./pyrsia build docker --image alpine:3.16.0
```

The build trigger should return immediately providing a build ID:

```text
Build request successfully handled. Build with ID c9ca3e57-aa84-4fab-a8be-381ab31e4916 has been started.
```

## Wait for the build to finish in the build pipeline

In the Pyrsia node logs of node A, you will see that a build has been started and
the Pyrsia node is now waiting for its result:

```text
INFO  pyrsia_registry > 127.0.0.1:50187 "POST /build/docker HTTP/1.1" 200 "-" "-" 42.826041ms
DEBUG pyrsia::build_service::service > Updated build info: BuildInfo { id: "c9ca3e57-aa84-4fab-a8be-381ab31e4916", status: Running }
```

In the build pipeline prototype you should see that build starting:

```text
#######################################################
#
# Starting Docker build for:
#   alpine:3.16
#
#######################################################
...
```

Do note that the build pipeline prototype will not actually build the docker image,
but instead download it from Docker Hub.

Once the build has finished, the status request from the Pyrsia node will contain:

```text
DEBUG pyrsia::build_service::event   > Handle BuildEvent: Result
{
   "build_id":"c9ca3e57-aa84-4fab-a8be-381ab31e4916",
   "build_trigger":"FromSource",
   "build_result":"BuildResult"{
      "package_type":"Docker",
      "package_specific_id":"alpine:3.16",
      "artifacts":[
         "BuildResultArtifact"{
            "artifact_specific_id":"alpine:3.16",
            "artifact_location":"/private/tmp/pyrsia/builds/c9ca3e57-aa84-4fab-a8be-381ab31e4916/1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870",
            "artifact_hash":"1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870"
         },
         "BuildResultArtifact"{
            "artifact_specific_id":"alpine@sha256:1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870",
            "artifact_location":"/private/tmp/pyrsia/builds/c9ca3e57-aa84-4fab-a8be-381ab31e4916/1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870",
            "artifact_hash":"1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870"
         },
         "BuildResultArtifact"{
            "artifact_specific_id":"alpine@sha256:213ec9aee27d8be045c6a92b7eac22c9a64b44558193775a1a7f626352392b49",
            "artifact_location":"/private/tmp/pyrsia/builds/c9ca3e57-aa84-4fab-a8be-381ab31e4916/213ec9aee27d8be045c6a92b7eac22c9a64b44558193775a1a7f626352392b49",
            "artifact_hash":"213ec9aee27d8be045c6a92b7eac22c9a64b44558193775a1a7f626352392b49"
         },
         "BuildResultArtifact"{
            "artifact_specific_id":"alpine@sha256:9c6f0724472873bb50a2ae67a9e7adcb57673a183cea8b06eb778dca859181b5",
            "artifact_location":"/private/tmp/pyrsia/builds/c9ca3e57-aa84-4fab-a8be-381ab31e4916/9c6f0724472873bb50a2ae67a9e7adcb57673a183cea8b06eb778dca859181b5",
            "artifact_hash":"9c6f0724472873bb50a2ae67a9e7adcb57673a183cea8b06eb778dca859181b5"
         }
      ]
   }
}
INFO  pyrsia::artifact_service::service > Build with ID c9ca3e57-aa84-4fab-a8be-381ab31e4916 completed successfully for package type Docker and package specific ID alpine:3.16
```

## Try to reach consensus with the other authorized nodes

Pyrsia node A will now try to reach consensus with the
other authorized nodes, but since we are only running one authorized node, this
step is implicit and node A will continue with the next steps: creating and
distributing the new transparency log.

## Create a transparency log about the artifact publication

When consensus has been reached, a transparency log is created for each built artifact.

```text
INFO  pyrsia::artifact_service::service > Adding artifact to transparency log: AddArtifactRequest { package_type: Docker, package_specific_id: "alpine:3.16", num_artifacts: 4, package_specific_artifact_id: "alpine:3.16", artifact_hash: "1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870" }
pyrsia::transparency_log::log     > Transparency log inserted into database with id: cc3dec20-8604-4d0a-8c18-ccb746769696
INFO  pyrsia::artifact_service::service > Transparency Log for build with ID c9ca3e57-aa84-4fab-a8be-381ab31e4916 successfully added. Adding artifact locally: TransparencyLog { id: "cc3dec20-8604-4d0a-8c18-ccb746769696", package_type: Docker, package_specific_id: "alpine:3.16", num_artifacts: 4, package_specific_artifact_id: "alpine:3.16", artifact_hash: "1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870", source_hash: "", artifact_id: "75c7bd83-1dd4-4666-a35f-e8c59b695e21", source_id: "7ec06216-b2dc-4e5a-a90d-7875fb77b846", timestamp: 1660906467, operation: AddArtifact, node_id: "64765410-136b-4332-a837-226bd062ba37", node_public_key: "558b0373-a29d-40c9-8125-019fb74dda31" }
```

Example for `alpine:3.16`:

```text
{
   "id":"cc3dec20-8604-4d0a-8c18-ccb746769696",
   "package_type":"Docker",
   "package_specific_id":"alpine:3.16",
   "num_artifacts":4,
   "package_specific_artifact_id":"alpine:3.16",
   "artifact_hash":"1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870",
   "source_hash":"",
   "artifact_id":"75c7bd83-1dd4-4666-a35f-e8c59b695e21",
   "source_id":"7ec06216-b2dc-4e5a-a90d-7875fb77b846",
   "timestamp":1660906467,
   "operation":"AddArtifact",
   "node_id":"64765410-136b-4332-a837-226bd062ba37",
   "node_public_key":"558b0373-a29d-40c9-8125-019fb74dda31"
}
```

## Publish the artifact on the p2p network

As a final step in the build from source scenario, the artifacts are stored locally
on node A and provided on the p2p network.

```text
 INFO  pyrsia::artifact_service::service > put_artifact with id: da341557-9150-4208-9474-f5884f799338
 INFO  pyrsia::artifact_service::storage > An artifact is being pushed to the artifact manager da341557-9150-4208-9474-f5884f799338
 DEBUG pyrsia::network::client           > p2p::Client::provide "da341557-9150-4208-9474-f5884f799338"
 ```

Now we are ready to use the published artifacts in our build workflow on node B
as shown in the sample section below.

## Use Pyrsia with Docker pull

Now that we have a Pyrsia network including a published Docker image, we can start
using Pyrsia with Docker.

### Configure Docker desktop to use node B as registry mirror

On Windows or MacOS, open your Docker desktop installation -> Settings ->
Docker Engine where Docker allows you to set registry-mirrors. Configure node B
as a registry mirror by adding/editing the following in the configuration:

```jsonc
 "registry-mirrors": [
   "http://192.168.0.110:7888" // (IP address of host machine and port number of your Pyrsia node)
 ]
```

On Linux, you'll find this configuration in the file `/etc/docker/daemon.json`.

On MacOS or Windows, you can't specify `localhost` because the request will
originate from the Docker Desktop VM, so you will need to specify the IP
address of host machine. On Linux you can use localhost.

You will need to restart Docker Desktop. Once restarted you should be able to
pull Docker images through Pyrsia.

## Pull `alpine` docker image

First make sure Alpine is not in local Docker cache, then pull Alpine:

```sh
docker rmi alpine:3.16.0 # remove alpine from local docker cache
docker pull alpine:3.16.0
```

You'll see this in the Pyrsia logs of node B:

```text
INFO  pyrsia_registry                      > 192.168.0.227:60446 "GET /v2/ HTTP/1.1" 200 "-" "docker/20.10.17 go/go1.17.11 git-commit/a89b842 kernel/5.10.124-linuxkit os/linux arch/arm64 UpstreamClient(Docker-Client/20.10.17-rd \(darwin\))" 259.375µs
DEBUG pyrsia::docker::v2::handlers::manifests > Fetching manifest for alpine with tag: 3.16
INFO  pyrsia::artifact_service::storage       > An artifact is being pulled from the artifact manager f6e32438-b23d-47be-908b-b6b97901a724
DEBUG pyrsia::network::client                 > p2p::Client::list_providers "f6e32438-b23d-47be-908b-b6b97901a724"
DEBUG pyrsia::network::event_loop             > Local Peer 12D3KooWNVVUAbLQnBWHvRS4Ad4aWpZyUTZMN5126KTgyYcubtpB is dialing Peer 12D3KooWHZRXJTjvYfP5A34bRLmeg9xmDFm46LUQJHHnhmemoUf6...
DEBUG pyrsia::network::client                 > p2p::Client::get_idle_peer() entered with 1 peers
DEBUG pyrsia::network::idle_metric_protocol   > p2p::idle_metric_protocol::write_request writing a request to peer for and idle metric
DEBUG pyrsia::network::idle_metric_protocol   > p2p::idle_metric_protocol::read_response Reading response to idle metric request with value =[150, 62, 178, 249, 43, 85, 23, 66]
DEBUG pyrsia::network::client                 > p2p::Client::get_idle_peer() Pushing idle peer with value 25053298284.56112
DEBUG pyrsia::network::client                 > p2p::Client::request_artifact PeerId("12D3KooWHZRXJTjvYfP5A34bRLmeg9xmDFm46LUQJHHnhmemoUf6"): "f6e32438-b23d-47be-908b-b6b97901a724"
DEBUG pyrsia::network::artifact_protocol      > Write ArtifactRequest: "f6e32438-b23d-47be-908b-b6b97901a724"
INFO  pyrsia::artifact_service::service       > put_artifact with id: f6e32438-b23d-47be-908b-b6b97901a724
INFO  pyrsia::artifact_service::storage       > An artifact is being pushed to the artifact manager f6e32438-b23d-47be-908b-b6b97901a724
```

Indicating that the Alpine image was first pulled from the Pyrsia network and then
stored locally, so node B can now also participate in the p2p content distribution.

## Inspect the transparency logs

The transparency logs that were created as part of the build from source process,
can be inspected using the Pyrsia CLI on any node. You can change the CLI config
to use the default port 7888 again to inspect the logs on node B, or you can run
inspect-log without any changes to inspect the logs on node A:

```sh
./pyrsia inspect-log docker --image alpine:3.16.0
```

This CLI command returns the transparency logs for all the Pyrsia artifacts that
make up the Docker image `alpine:3.16`:

```text
[
  {
    "id": "cc3dec20-8604-4d0a-8c18-ccb746769696",
    "package_type": "Docker",
    "package_specific_id": "alpine:3.16",
    "num_artifacts": 4,
    "package_specific_artifact_id": "alpine:3.16",
    "artifact_hash": "1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870",
    "source_hash": "",
    "artifact_id": "75c7bd83-1dd4-4666-a35f-e8c59b695e21",
    "source_id": "7ec06216-b2dc-4e5a-a90d-7875fb77b846",
    "timestamp": 1660906467,
    "operation": "AddArtifact",
    "node_id": "64765410-136b-4332-a837-226bd062ba37",
    "node_public_key": "558b0373-a29d-40c9-8125-019fb74dda31"
  },
  {
    "id": "d88982b1-261b-4e3d-9eb2-dd549c40ac05",
    "package_type": "Docker",
    "package_specific_id": "alpine:3.16",
    "num_artifacts": 4,
    "package_specific_artifact_id": "alpine@sha256:1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870",
    "artifact_hash": "1304f174557314a7ed9eddb4eab12fed12cb0cd9809e4c28f29af86979a3c870",
    "source_hash": "",
    "artifact_id": "f2648155-b665-4567-9e3c-27af7cc3b9bb",
    "source_id": "0ca693f9-7c50-4448-9cd6-0d7a145fba14",
    "timestamp": 1660906529,
    "operation": "AddArtifact",
    "node_id": "60b7d9ae-d5ba-4440-ab83-6c5638a18a45",
    "node_public_key": "4a873a2a-0e04-4540-b1bd-bccc0d721ed2"
  },
  {
    "id": "f53f9cc6-6998-470a-8094-cae3fbc82412",
    "package_type": "Docker",
    "package_specific_id": "alpine:3.16",
    "num_artifacts": 4,
    "package_specific_artifact_id": "alpine@sha256:213ec9aee27d8be045c6a92b7eac22c9a64b44558193775a1a7f626352392b49",
    "artifact_hash": "213ec9aee27d8be045c6a92b7eac22c9a64b44558193775a1a7f626352392b49",
    "source_hash": "",
    "artifact_id": "dac2e42c-fd48-4487-b48c-34f5eac1f674",
    "source_id": "eed938e9-9cf8-4e1b-995f-6a6d1da6ef26",
    "timestamp": 1660906589,
    "operation": "AddArtifact",
    "node_id": "1e3244e3-1fc5-429b-8cc6-43dbbebaccb2",
    "node_public_key": "7d7d96c0-1b8b-4028-bb20-df9a45eeaa7f"
  },
  {
    "id": "cae2f5a7-22ec-4d22-86af-59e1f0239056",
    "package_type": "Docker",
    "package_specific_id": "alpine:3.16",
    "num_artifacts": 4,
    "package_specific_artifact_id": "alpine@sha256:9c6f0724472873bb50a2ae67a9e7adcb57673a183cea8b06eb778dca859181b5",
    "artifact_hash": "9c6f0724472873bb50a2ae67a9e7adcb57673a183cea8b06eb778dca859181b5",
    "source_hash": "",
    "artifact_id": "3fc0ac72-8f5e-41fe-8ab6-94c565ebc52c",
    "source_id": "4cb49c33-af4c-4c3a-8053-b771007a6720",
    "timestamp": 1660906649,
    "operation": "AddArtifact",
    "node_id": "64d30c8e-d356-420c-ab87-e27687ca6f1d",
    "node_public_key": "57130e5d-d0dc-450b-b80d-966cb71210ef"
  }
]
```
