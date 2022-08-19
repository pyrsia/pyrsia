# How to setup a Pyrsia node to build Docker images from source

> **Warning:** The build-from-source demo is still work-in-progress.

This tutorial describes how to setup a Pyrsia node that can build Docker images
from source with the goal to publish them in the Pyrsia network. (Note: in the
current prototype, the build pipeline does not actually build the image, but
downloads them from Docker Hub instead).

Ultimately, the following scenario will be used, but for now some steps
(indicated below) are skipped for the purpose of this build-from-source demo:

- Setup at least 3 authorized nodes (Skipped in this demo, only one Pyrsia node
  is used)
- Set up a build pipeline per Pyrsia node and configure the nodes to use them
- Trigger a build from source for a given artifact
- Wait for the build to finish in the build pipeline
- Try to reach consensus with the other authorized nodes, which have to run the
  same build and verify they produce the same build result. (Skipped in this demo)
- Create a transparency log about the artifact publication
- Publish the artifact on the p2p network

See the [architecture and use-cases](../developers/pyrsia-architecture-and-use-cases.md)
document for more information.

Because this demo scenario results in a published Docker image in the Pyrsia
network, we can run a final step to show the build from source worked:

- Use docker pull to pull the docker image from the Pyrsia network

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

## Run the Pyrsia node

Now we will set the following env vars and start a pyrsia node:

- RUST_LOG: to make sure we can see all the debug logs
- DEV_MODE: to make sure all non-existing directories are created on-the-fly
- PYRSIA_ARTIFACT_PATH: pointing to a directory to store artifacts. optionally
  remove this directory prior to starting Pyrsia if you want to start from an
  empty state.

```sh
RUST_LOG=pyrsia=debug DEV_MODE=on PYRSIA_ARTIFACT_PATH=/tmp/pyrsia \
cargo run --package pyrsia_node -- --pipeline-service-endpoint http://localhost:8080 -H 0.0.0.0 --listen-only true
```

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
and run as follows:

```sh
cd pyrsia_build_pipeline_prototype
RUST_LOG=debug cargo run
```

By default, this prototype listens on http port 8080. If you run it on a different
host or port, make sure to specify its location when starting the Pyrsia node
with `--pipeline-service-endpoint` (see above).

You will see the following output indicating that the build pipeline is ready for use

```text
   Finished dev [unoptimized + debuginfo] target(s) in 1m 07s
     Running `target/debug/pyrsia_build`
 INFO  actix_server::builder > Starting 8 workers
 INFO  actix_server::server  > Tokio runtime found; starting in existing Tokio runtime
```

## Trigger a build from source for a given artifact

In this section we will trigger a build for `alpine:3.16`.

We will use the Pyrsia CLI to trigger a build from source. In a new terminal, while
the Pyrsia node and build pipeline prototype are running, check if your Pyrsia CLI
config is correct:

```sh
cd $PYRSIA_HOME/target/debug
./pyrsia config --show
host = 'localhost'
port = '7888'
disk_allocated = '5.84 GB'
```

If you're not using the default port for your Pyrsia node, make sure to configure
the CLI using `./pyrsia config --add`.

Then trigger the build from source, like this:

```sh
./pyrsia build docker --image alpine:3.16
```

The build trigger should return immediately providing a build ID:

```text
Build request successfully handled. Build with ID c9ca3e57-aa84-4fab-a8be-381ab31e4916 has been started.
```

## Wait for the build to finish in the build pipeline

In the Pyrsia node logs, you will see that a build has been started and the Pyrsia
node is now waiting for its result:

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

In a regular scenario, the Pyrsia node will now try to reach consensus with the
other authorized nodes, but this step is skipped in this demo.

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
and provided on the p2p network.

```text
 INFO  pyrsia::artifact_service::service > put_artifact with id: da341557-9150-4208-9474-f5884f799338
 INFO  pyrsia::artifact_service::storage > An artifact is being pushed to the artifact manager da341557-9150-4208-9474-f5884f799338
 DEBUG pyrsia::network::client           > p2p::Client::provide "da341557-9150-4208-9474-f5884f799338"
 ```

Now we are ready to use the published artifacts in our build workflow as shown in
the sample section below.

## Use Pyrsia with Docker pull

Now that we have a Pyrsia network including a published Docker image, we can start
using Pyrsia with Docker.

### Configure Docker desktop to use node A as registry mirror

In your Docker desktop installation -> Settings -> Docker Engine where Docker
allows you to set registry-mirrors. Setup node A as a registry mirror by
adding/editing the following in the configuration.

```jsonc
 "registry-mirrors": [
   "http://192.168.0.110:7888" // (IP address of host machine and port number of your Pyrsia node)
 ]
```

Note: if you're using Linux, you'll find this file in `/etc/docker/daemon.json`.

On Mac OS X using localhost does not work (because the request is made from the
Docker Desktop VM), so you will need to specify the IP address of host machine.
On Linux you can use localhost.

You will need to restart Docker Desktop. Once restarted you should be able to
pull Docker images through Pyrsia.

## Pull `alpine` docker image

First make sure Alpine is not in local Docker cache, then pull Alpine:

```sh
docker rmi alpine:3.16 # remove alpine from local docker cache
docker pull alpine:3.16
```

You'll see this in the Pyrsia logs:

```text
INFO  pyrsia_registry                         > 192.168.0.227:64436 "GET /v2/ HTTP/1.1" 200 "-" "docker/20.10.17 go/go1.17.11 git-commit/a89b842 kernel/5.10.104-linuxkit os/linux arch/arm64 UpstreamClient(Docker-Client/20.10.17 \(darwin\))" 76.666µs
DEBUG pyrsia::docker::v2::handlers::manifests > Fetching manifest for alpine with tag: 3.16
INFO  pyrsia::artifact_service::storage       > An artifact is being pulled from the artifact manager b0ed9f25-f322-47ef-8dac-03154209cfcf
INFO  pyrsia_registry                         > 192.168.0.227:64437 "HEAD /v2/library/alpine/manifests/3.16 HTTP/1.1" 200 "-" "docker/20.10.17 go/go1.17.11 git-commit/a89b842 kernel/5.10.104-linuxkit os/linux arch/arm64 UpstreamClient(Docker-Client/20.10.17 \(darwin\))" 1.04075ms
```

Indicating that the Alpine image was pulled from Pyrsia.

## Inspect the transparency logs

The transparency logs that were created as part of the build from source process,
can be inspected using the Pyrsia CLI.

```sh
./pyrsia inspect-log docker --image alpine:3.16
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
