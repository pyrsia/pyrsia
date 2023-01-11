---
sidebar_position: 11
---

# Pyrsia demo: build Maven images from source

> **Warning:** This tutorial is aimed at developers actively contributing to the
> Pyrsia codebase. If you simply want to use Pyrsia, please have a look at [Configure Maven to use Pyrsia](/docs/tutorials/maven.md)

This tutorial describes how to setup a Pyrsia node that can build Maven artifacts
from source with the goal to publish them in the Pyrsia network.

Ultimately, the following scenario will be used, but for now some steps
(indicated below) are skipped for the purpose of this build-from-source demo:

- Setup at least 3 authorized nodes (Skipped in this demo, only one Pyrsia node
  is used)
- Make sure a mapping between an artifact and its source exists in the
  [Pyrsia Mappings Repo](https://github.com/pyrsia/pyrsia-mappings) (for Maven artifacts
  only)
- Set up a build pipeline per Pyrsia node and configure the nodes to use them
- Trigger a build from source for a given artifact
- Wait for the build to finish in the build pipeline
- Try to reach consensus with the other authorized nodes, which have to run the
  same build and verify they produce the same build result. (Skipped in this demo)
- Create a transparency log about the artifact publication
- Publish the artifact on the p2p network

See the [architecture and use-cases](pyrsia-architecture-and-use-cases.md)
document for more information.

Because this demo scenario results in a published Maven artifact in the Pyrsia
network, we can run a final step to show the build from source worked:

- Use Pyrsia in a Maven project

## Prerequisites

The following steps rely on JDK11 and maven being correctly installed.
Please find and install the appropriate [JDK11](https://www.openlogic.com/openjdk-downloads) and [mvn](https://maven.apache.org/install.html) before proceeding.

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
cargo run --package pyrsia_node -- --pipeline-service-endpoint http://localhost:8080 --listen-only --init-blockchain
```

As you can see, we specified the `--pipeline-service-endpoint` argument to point
to `http://localhost:8080`, which is where we will run our build pipeline prototype
(see below). In a production setup, the build pipeline needs to run on its
own isolated infrastructure.

## Create a mapping between the artifact and its source repository

In this demo, we will build a Maven artifact from source. Since there's no direct
connection between the Maven artifact defined as `groupId:artifactId:version` and
its source repository, Pyrsia keeps a [public mapping repository](https://github.com/pyrsia/pyrsia-mappings). For every known
artifact, this repository has a mapping file like this:

```json
{
  "package_type": "Maven2",
  "package_specific_id": "groupId:artifactId:version",
  "source_repository": {
    "Git": {
      "url": "https://github.com/organization/artifact",
      "tag": "rel/artifact-version-tag"
    }
  },
  "build_spec_url": ""
}
```

In case you want to build an artifact from source for which a mapping is not yet
available, feel free to create a pull request to add it to the mapping repository.

In this demo we will build `commons-codec:commons-codec:1.15` for which the [mapping](https://github.com/pyrsia/pyrsia-mappings/blob/main/Maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.mapping)
is already available.

## Run build pipeline prototype

In this demo, we use a build pipeline prototype. Eventually, Pyrsia will use
industry-standard build pipeline solutions. Integration with such solutions is
currently being investigated.

The most important features of the build pipeline are:

- it runs completely separate from the Pyrsia node
- it exposes an interface so Pyrsia can start a build
- it exposes an interface so Pyrsia can download the build output

The current build pipeline prototype only supports Maven builds. It takes
the artifact mapping as input (see above), starts a Maven build and provides the
build output as a download.

Ensure that JAVA_HOME is setup correctly

```sh
export JAVA_HOME=/Library/Java/JavaVirtualMachines/jdk1.8.0_jdk/Contents/Home
```

and maven is available on the PATH

```sh
export PATH=path to your maven download location/apache-maven-3.8.6/bin:$PATH
```

Download or clone the [prototype repo](https://github.com/tiainen/pyrsia_build_pipeline_prototype)
and run as follows:

```sh
cd pyrsia_build_pipeline_prototype
RUST_LOG=debug cargo run
```

By default, this prototype listens on http port 8080. If you run it on a different
host or port, make sure to specify its location when starting the Pyrsia node
with `--pipeline-service-endpoint` (see above).

Because we will be using this prototype for building Maven artifacts, make sure
you have installed a JDK11 and configured JAVA_HOME before running `cargo run`.

You will see the following output indicating that the build pipeline is ready for use

```text
   Finished dev [unoptimized + debuginfo] target(s) in 1m 07s
     Running `target/debug/pyrsia_build`
 INFO  actix_server::builder > Starting 8 workers
 INFO  actix_server::server  > Tokio runtime found; starting in existing Tokio runtime
```

## Authorize the Pyrsia node as a build node

We will use the Pyrsia CLI to authorize node A as a build node. In a new terminal, while
the Pyrsia node and build pipeline prototype are running, check if your Pyrsia CLI
config is correct:

```sh
cd $PYRSIA_HOME/target/debug
./pyrsia config --show
Config file path: /some/path/default-config.toml
host = 'localhost'
port = '7888'
disk_allocated = '10 GB'
```

If you're not using the default port for your Pyrsia node, make sure to configure
the CLI using `./pyrsia config edit`.

Next you'll need to find out the peer id of the node. You can see that in its logs
or you can query the `/status` endpoint like this: (assuming you have `jq` installed)

```shell
curl -s http://localhost:7888/status | jq  .peer_id
```

Once you know the peer id, authorize it like this:

```shell
./pyrsia authorize --peer <PEER_ID>
```

## Trigger a build from source for a given artifact

In this demo we trigger a build for `commons-codec:commons-codec:1.15`.
The mapping repository already contains the [source repository mapping](https://github.com/pyrsia/pyrsia-mappings/blob/main/Maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.mapping).

Then trigger the build from source, like this:

```sh
./pyrsia build maven --gav commons-codec:commons-codec:1.15
```

The build trigger should return immediately providing a build ID:

```text
Build request successfully handled. Build with ID '13abff76-5aea-4d05-8f42-d625943ceb78' has been started.
```

## Wait for the build to finish in the build pipeline

In the Pyrsia node logs, you will see that a build has been started and the Pyrsia
node is now waiting for its result:

```text
DEBUG pyrsia::build_service::service        > Updated build info: BuildInfo { id: "13abff76-5aea-4d05-8f42-d625943ceb78", status: Running }
```

In the build pipeline prototype you should see that the build has started:

```text
Requesting build of Maven2 for commons-codec:commons-codec:1.15
Starting build with ID 13abff76-5aea-4d05-8f42-d625943ceb78
#######################################################
#
# Starting Maven2 build for:
#   commons-codec:commons-codec:1.15
#
#######################################################
Cloning into 'repo'...
...
[INFO] ------------------------------------------------------------------------
[INFO] BUILD SUCCESS
[INFO] ------------------------------------------------------------------------
[INFO] Total time:  19.787 s
[INFO] Finished at: 2022-07-15T15:59:42+02:00
[INFO] ------------------------------------------------------------------------
...
```

Once the build has finished, the status request from the Pyrsia node will contain:

```text
DEBUG pyrsia::build_service::service        > Updated build info: BuildInfo { id: "13abff76-5aea-4d05-8f42-d625943ceb78", status: Success { artifact_urls: ["/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15.pom.md5", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-sources.jar.sha1", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15.pom", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15.jar", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-tests.jar.sha1", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15.jar.sha1", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-tests.jar.md5", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-tests.jar", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-test-sources.jar.sha1", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-test-sources.jar", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15.jar.md5", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15.pom.sha1", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-sources.jar", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-sources.jar.md5", "/build/13abff76-5aea-4d05-8f42-d625943ceb78/artifacts/commons-codec-1.15-test-sources.jar.md5"] } }
```

And Pyrsia will download all build result files from the pipeline service.

## Try to reach consensus with the other authorized nodes

In a regular scenario, the Pyrsia node will now try to reach consensus with the
other authorized nodes, but this step is skipped in this demo.

## Create a transparency log about the artifact publication

When consensus has been reached, a transparency log is created for each built artifact.

```text
INFO  pyrsia::artifact_service::service     > Adding artifact to transparency log: AddArtifactRequest { package_type: Maven2, package_specific_id: "commons-codec:commons-codec:1.15", num_artifacts: 15, package_specific_artifact_id: "commons-codec/commons-codec/1.15/commons-codec-1.15.pom.md5", artifact_hash: "ff89aba3ea6e2655feba41fdff2d8388b09f421ca6ca0ff5c49dbc24e53ae86a" }
INFO  pyrsia::artifact_service::service     > Transparency Log for build with ID 13abff76-5aea-4d05-8f42-d625943ceb78 successfully created.
DEBUG pyrsia::transparency_log::log         > Transparency log inserted into database with id: 95bd06e4-8254-437f-bc28-432883379426
```

## Publish the artifact on the p2p network

As a final step in the build from source scenario, the artifacts are stored locally
and provided on the p2p network.

```text
INFO  pyrsia::artifact_service::service     > put_artifact with id: e7eb2455-51b8-4d93-b8b7-1cde8004f1e2
INFO  pyrsia::artifact_service::storage     > An artifact is being pushed to the artifact manager e7eb2455-51b8-4d93-b8b7-1cde8004f1e2
DEBUG pyrsia::network::client               > p2p::Client::provide "e7eb2455-51b8-4d93-b8b7-1cde8004f1e2"
```

Now we are ready to use the published artifacts in our build workflow as shown in
the sample section below.

## Use Pyrsia in a Maven project

Now that we have a published Maven artifact in the Pyrsia network, we can try to
use it in a Maven project.

Create a Java project:

```sh
mkdir pyrsia-maven-sample
cd pyrsia-maven-sample
mkdir -p src/main/java/org/pyrsia/sample
```

Create a file `src/main/java/org/pyrsia/sample/Main.java`:

```java
package org.pyrsia.sample;

import java.util.Arrays;
import org.apache.commons.codec.binary.Hex;

public class Main {

    public static void main(String[] args) {
        byte[] data = { 1, 2, 3, 4, 5, 6, 7, 8 };
        String hexEncodedData = Hex.encodeHexString(data);

        System.out.println("Byte array " + Arrays.toString(data) + " encoded as a hex string: " + hexEncodedData);
    }
}
```

The code in this sample uses `org.apache.commons.codec.binary.Hex` from the commons-codec
library, so let's add this as dependency in our Maven build:

Create a `pom.xml` file:

```xml
<project xmlns:xsi="http://www.w3.org/2001/XMLSchema-instance"
         xmlns="http://maven.apache.org/POM/4.0.0"
         xsi:schemaLocation="http://maven.apache.org/POM/4.0.0
              http://maven.apache.org/xsd/maven-4.0.0.xsd">
    <modelVersion>4.0.0</modelVersion>
    <groupId>org.pyrsia.sample</groupId>
    <artifactId>maven</artifactId>
    <packaging>jar</packaging>
    <version>1.0.0-SNAPSHOT</version>
    <name>Pyrsia Sample for Maven</name>

    <repositories>
        <repository>
            <id>pyrsia</id>
            <url>http://localhost:7888/maven2</url>
        </repository>
    </repositories>

    <dependencies>
        <dependency>
            <groupId>commons-codec</groupId>
            <artifactId>commons-codec</artifactId>
            <version>1.15</version>
        </dependency>
    </dependencies>

    <build>
        <plugins>
            <plugin>
                <groupId>org.apache.maven.plugins</groupId>
                <artifactId>maven-compiler-plugin</artifactId>
                <version>3.10.1</version>
                <configuration>
                    <release>11</release>
                </configuration>
            </plugin>
        </plugins>
    </build>
</project>
```

As you can see, we have set the repository to `http://localhost:7888/maven2`, which
will trigger the Maven build to request dependencies from this URL, which is our
Pyrsia node, acting as a Maven repository.

To make sure your local maven cache doesn't already contain this dependency, remove
it first:

```sh
rm -rf ~/.m2/repository/commons-codec/commons-codec/1.15
```

And then run the maven build:

```sh
mvn clean package
```

It should show output like this:

```text
[INFO] Scanning for projects...
[INFO]
[INFO] ----------------------< org.pyrsia.sample:maven >-----------------------
[INFO] Building Pyrsia Sample for Maven 1.0.0-SNAPSHOT
[INFO] --------------------------------[ jar ]---------------------------------
Downloading from pyrsia: http://localhost:7888/maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.pom
Downloaded from pyrsia: http://localhost:7888/maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.pom (15 kB at 343 kB/s)
Downloading from pyrsia: http://localhost:7888/maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.jar
Downloaded from pyrsia: http://localhost:7888/maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.jar (354 kB at 9.3 MB/s)
...
[INFO] ------------------------------------------------------------------------
[INFO] BUILD SUCCESS
[INFO] ------------------------------------------------------------------------
[INFO] Total time:  0.925 s
[INFO] Finished at: 2022-07-15T16:33:39+02:00
[INFO] ------------------------------------------------------------------------
```

The dependency was downloaded from Pyrsia, which you can verify in the Pyrsia node
logs:

```text
DEBUG pyrsia::java::maven2::routes                    > route full path: /maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.jar
DEBUG pyrsia::java::maven2::handlers::maven_artifacts > Requesting maven artifact: /maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.jar
DEBUG pyrsia::java::maven2::handlers::maven_artifacts > Requesting artifact with package specific id: commons-codec:commons-codec:1.15, and package specific artifact id: commons-codec/commons-codec/1.15/commons-codec-1.15.jar. If not found a build will be requested
INFO  pyrsia::artifact_service::storage               > An artifact is being pulled from the artifact manager 73cee038-59dc-46f1-a325-fef83e4ff51c
INFO  pyrsia_registry                                 > 127.0.0.1:55180 "GET /maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.jar HTTP/1.1" 200 "-" "Apache-Maven/3.6.3 (Java 11.0.8; Linux 5.15.0-56-generic)" 24.345381ms
```

## Inspect the transparency logs

The transparency logs that were created as part of the build from source process,
can be inspected using the Pyrsia CLI.

```sh
/pyrsia inspect-log maven --gav commons-codec:commons-codec:1.15
```

This results in the transparency logs for all the Pyrsia artifacts that make up
the Maven library:

```text
[
  {
    "artifact_hash": "ff89aba3ea6e2655feba41fdff2d8388b09f421ca6ca0ff5c49dbc24e53ae86a",
    "artifact_id": "e7eb2455-51b8-4d93-b8b7-1cde8004f1e2",
    "id": "95bd06e4-8254-437f-bc28-432883379426",
    "node_id": "6d2faed8-88fe-4aac-8414-ea0055cb828d",
    "node_public_key": "076a5a92-9052-43ff-a665-2e9b6a88df73",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.pom.md5",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "5e58cae7-b06c-4d21-a475-fb8b188e1a3f",
    "timestamp": 1671004615
  },
  {
    "artifact_hash": "944a6c7643cc0ef821d38afdb576c3a3ea2d2308c053eb8e0bcbbc624766e345",
    "artifact_id": "0f02db39-a3ae-4039-84ce-68b1726156ce",
    "id": "7bf97f16-e8a6-4c56-8c4c-4d7e6adf7cfd",
    "node_id": "3bf52a5a-1230-4b1d-b32f-b660d2d7a5cd",
    "node_public_key": "ddd83497-e571-4282-8be2-5307df1a636d",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-sources.jar.sha1",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "95a0ca89-dcfa-4306-9314-92e2411c5fce",
    "timestamp": 1671004615
  },
  {
    "artifact_hash": "c86ee198a35a3715487860f419cbf642e7e4d9e8714777947dbe6a4e3a20ab58",
    "artifact_id": "311a547b-bce2-429a-8def-783f4e2fbc61",
    "id": "2fcbefd2-c40f-4a06-8b5d-2061e7e4b6b5",
    "node_id": "4899356d-f1d5-4041-a7dd-2e26dce42fd4",
    "node_public_key": "5faeb8d0-0e58-4fed-a0a8-49b62ec75095",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.pom",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "94617920-9637-4722-83e6-7850ef4e9153",
    "timestamp": 1671004615
  },
  {
    "artifact_hash": "9eb1496028b5de8910b14febc3f6a10722eb61aee79c0f2bfda3678d64381541",
    "artifact_id": "73cee038-59dc-46f1-a325-fef83e4ff51c",
    "id": "595a61d8-bcce-4e67-9d0e-47760838697e",
    "node_id": "a61060dc-998e-4e4b-b881-34b8268525b4",
    "node_public_key": "86e858cd-33ec-44b3-9465-0a330db1b901",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.jar",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "344c7854-f0bd-47f4-9972-d490d42054f9",
    "timestamp": 1671004615
  },
  {
    "artifact_hash": "db4e82248ee9c594e63e4ad5bef30eb7063ed3c0bd498ed3af71294b0ebf7a3e",
    "artifact_id": "93576736-e64f-4da2-8cee-b88034e54c4c",
    "id": "7e1953a3-c2c6-4efd-b48e-56ef387b9b33",
    "node_id": "6f2f1613-89b3-48c5-ba5d-39cd1cb55004",
    "node_public_key": "9553c499-db8e-48c1-8370-171e106a3329",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-tests.jar.sha1",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "e982f1a7-5d7b-4860-aab3-adaf33ce1bfb",
    "timestamp": 1671004616
  },
  {
    "artifact_hash": "9bcaba0174b9803b04312a577917b20a9152582f1c8bf3b4ca9876f5a1569164",
    "artifact_id": "df34d9de-9757-4c6d-b7a6-730c1cb0e18e",
    "id": "b6b0ebfb-8fed-4229-906f-7e9528601880",
    "node_id": "faf9ae75-a6e4-49bc-bb4e-a3e82e1f00b4",
    "node_public_key": "bae676c7-95b0-4c16-8520-b53fcfda1cdc",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.jar.sha1",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "cb5d0c3b-4475-4b76-b039-64bceb3af222",
    "timestamp": 1671004616
  },
  {
    "artifact_hash": "2758a7c8666bc7a91cb74a042b24f8e7f0c912e4a5461dbaf5fa35b227332c65",
    "artifact_id": "236c5cb0-ddf4-4f16-89bf-bc5ec1052abd",
    "id": "a36faabf-d51a-40d0-b4e8-addf56cfe2f7",
    "node_id": "2276b670-fed6-46b1-b8f0-fb03ed775ab6",
    "node_public_key": "638f0276-ea41-4257-8dda-089ae867a5de",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-tests.jar.md5",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "5feaa8d2-f868-4ae4-8f53-44da49a1f85e",
    "timestamp": 1671004616
  },
  {
    "artifact_hash": "4d1cf8fc05b8952412bcb436579bc335987364900e1f44194dbb98930c23eee8",
    "artifact_id": "cc5a5cce-f292-4865-9dba-c96986b042d2",
    "id": "5dcb88dc-0162-4a86-b3a3-80d6288290c9",
    "node_id": "dfcaf577-dab4-4ada-a3b8-e4948898308a",
    "node_public_key": "d2a68745-908a-4210-aea9-73138084e228",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-tests.jar",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "b02667a1-9938-441b-9c48-1bf70219f999",
    "timestamp": 1671004616
  },
  {
    "artifact_hash": "a9be7ceaf4962f4c897239972a1dac711ca7570c2cf47a8b09b7b343950caf41",
    "artifact_id": "521faabe-7205-4ef6-8cdc-f4fafb0c58d0",
    "id": "1a200591-cf3c-497f-9f87-5fba40ab61a6",
    "node_id": "8b6a0dfb-94ba-4122-b06b-a9daefc08c00",
    "node_public_key": "aa0a16d2-6957-4107-8bd4-1936d2679e12",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-test-sources.jar.sha1",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "3838c987-75a4-49d8-8883-4936ca6c6be0",
    "timestamp": 1671004616
  },
  {
    "artifact_hash": "997a5e8ed8c37e0c3dcdf20e76db11279c055a20c3875731390c7b8008912c4f",
    "artifact_id": "7c3d7314-e455-4715-b760-a30f8496c1dc",
    "id": "d371a08f-de02-4fe0-8aea-a3bd78d30504",
    "node_id": "29d9a1aa-60e6-4013-aacb-51f8f9baab1e",
    "node_public_key": "b4c5b976-3894-4fcc-bdf2-d98943bab7c3",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-test-sources.jar",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "802473db-3d7b-47af-b716-dba0fd83b6cd",
    "timestamp": 1671004617
  },
  {
    "artifact_hash": "30b6bfae61e5fdf2832f1378c64bc90cc24e970b7df48e97610afcdf5dc048c7",
    "artifact_id": "4057f20b-5d0b-4912-ada3-8a8fe596b175",
    "id": "6ad5899b-ea66-4d1d-85fc-b7d7dd373369",
    "node_id": "48e97e91-227e-42dd-9fe9-f49aeb7aebce",
    "node_public_key": "ac8bd86b-6325-4305-b08d-a413574d50f4",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.jar.md5",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "51eb79b0-29bc-4fd6-9e83-49038e355e80",
    "timestamp": 1671004617
  },
  {
    "artifact_hash": "5dc11ea7cfa14ac8e84250b21166b098f1ce57ba3316ba5cae589269b37ce54a",
    "artifact_id": "2dfdfe5f-3f2b-47a2-bcff-655a27dfca57",
    "id": "aa06913a-b1eb-4ab7-8e0a-91659e1ff990",
    "node_id": "98f3dc15-b999-4cda-be96-e861daa6ceb7",
    "node_public_key": "0bf9a398-662e-4cfd-9e34-d88256f275ed",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.pom.sha1",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "be90b0ed-885f-40f0-b409-0de0fb4863ec",
    "timestamp": 1671004617
  },
  {
    "artifact_hash": "930b528fc1cc6ad19b719c8f79fbf494814b3572f4df9f2555186ce2527a3116",
    "artifact_id": "ec0cd5d0-697e-48f3-bc8a-ef9ef9731ed9",
    "id": "7c12bd6f-5784-45eb-ac10-ed9f80c1f130",
    "node_id": "e0571aa3-d5d0-4d77-985e-00cf835ca496",
    "node_public_key": "51c179cc-9686-4cb4-aeb2-f2cdf93c20ea",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-sources.jar",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "a99fc485-a576-4f77-b513-aa1c5ba38647",
    "timestamp": 1671004617
  },
  {
    "artifact_hash": "53290bdbc449ae0e0230ab9f4d1d49043a3b67d0c40f49f1b6323188da2f0200",
    "artifact_id": "a76ea024-dfae-443e-947c-425546944cc1",
    "id": "a81459be-67d6-4eed-99ba-89800b66742c",
    "node_id": "92583486-2d4f-47eb-ad7a-1e4cc874f417",
    "node_public_key": "b73b0bfa-7196-4b23-9189-b320e73d7424",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-sources.jar.md5",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "03b057cb-a078-4cc7-af86-ccf8a9acd700",
    "timestamp": 1671004618
  },
  {
    "artifact_hash": "e74339769bffce344ee5a4645a64466ad4ffe2002cf774285e3f20fdb110b503",
    "artifact_id": "a1157f84-27a5-44ba-91ed-7d0b715d5cdc",
    "id": "11a18760-353b-4b01-8f08-1ebd7beccc0b",
    "node_id": "4ea91665-c74d-434c-b9bd-cab07f70ccd9",
    "node_public_key": "e9d812d8-bac1-4920-b846-d0d3d4c305ca",
    "num_artifacts": 15,
    "operation": "AddArtifact",
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-test-sources.jar.md5",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "package_type": "Maven2",
    "source_hash": "",
    "source_id": "c187d8ab-de26-4811-a2d5-8a0920eb0b9f",
    "timestamp": 1671004618
  }
]
```
