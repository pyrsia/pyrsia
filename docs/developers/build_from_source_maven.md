---
sidebar_position: 2
---

# Pyrsia demo: build Docker images from source

> **Warning:** The build-from-source demo is still work-in-progress.

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

## Authorize node A as a build node

We will use the Pyrsia CLI to authorize node A as a build node. In a new terminal, while
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

Next you'll need to find out the peer id of node A. You can see that in its logs
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
Build request successfully handled. Build with ID 23c994a6-65b7-4041-beca-397d8f491f64 has been started.
```

## Wait for the build to finish in the build pipeline

In the Pyrsia node logs, you will see that a build has been started and the Pyrsia
node is now waiting for its result:

```text
Executing build info request...!
Current Build Info: BuildInfo { id: "23c994a6-65b7-4041-beca-397d8f491f64", status: Running }
```

In the build pipeline prototype you should see that build starting:

```text
Requesting build of Maven2 for commons-codec:commons-codec:1.15
...
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
Current Build Info: BuildInfo { id: "23c994a6-65b7-4041-beca-397d8f491f64",
  status: Success { artifact_urls:
     ["/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15.pom.sha1",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-test-sources.jar",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-tests.jar",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-sources.jar.sha1",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-tests.jar.md5",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-tests.jar.sha1",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-test-sources.jar.sha1",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-sources.jar.md5",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15.jar.sha1",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15.pom.md5",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-test-sources.jar.md5",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15.jar",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15-sources.jar",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15.jar.md5",
      "/build/23c994a6-65b7-4041-beca-397d8f491f64/artifacts/commons-codec-1.15.pom"]
  } }
```

And Pyrsia will download all build result files from the pipeline service.

## Try to reach consensus with the other authorized nodes

In a regular scenario, the Pyrsia node will now try to reach consensus with the
other authorized nodes, but this step is skipped in this demo.

## Create a transparency log about the artifact publication

When consensus has been reached, a transparency log is created for each built artifact.

```text
INFO  pyrsia::artifact_service::service > Adding artifact to transparency log: AddArtifactRequest { package_type: Maven2, package_specific_id: "commons-codec:commons-codec:1.15", package_specific_artifact_id: "commons-codec/commons-codec/1.15/commons-codec-1.15.jar", artifact_hash: "7da8e6b90125463c26c950a97fd14143c2f39cd5d488748b265d83e8b124fa7c" }
DEBUG pyrsia::transparency_log::log     > Transparency log inserted into database with id: 2f30167e-e40f-4831-9197-11fc0b5450e3
INFO  pyrsia::artifact_service::service > Transparency Log for build with ID 0a6f2128-7410-4098-bd39-59dc05230464 successfully added. Adding artifact locally: TransparencyLog { id: "2f30167e-e40f-4831-9197-11fc0b5450e3", package_type: Maven2, package_specific_id: "commons-codec:commons-codec:1.15", package_specific_artifact_id: "commons-codec/commons-codec/1.15/commons-codec-1.15.jar", artifact_hash: "7da8e6b90125463c26c950a97fd14143c2f39cd5d488748b265d83e8b124fa7c", artifact_id: "6eb90399-24cd-4aef-a78f-ef95d64b53fa", source_id: "77ea0ea3-2eb7-4aac-9fdb-f43664ce62a4", timestamp: 1658132836, operation: AddArtifact, node_id: "5a04ba4d-9c8f-445a-bcb7-5c91a610d03c", node_public_key: "9c6ab508-1b86-47bb-87e9-6b99c18e4a73" }
```

Example for `commons-codec-1.15.jar`:

```json
{
  "id":"c52d7954-d9d9-40e2-a795-31aed2fc8a61",
  "package_type":"Maven2",
  "package_specific_id":"commons-codec:commons-codec:1.15",
  "package_specific_artifact_id":"commons-codec/commons-codec/1.15/commons-codec-1.15.jar",
  "artifact_hash":"3a1cabaab612b463e30fe44ae8794595311bbb8981bdcbb887736d35fcfd4d6f",
  "artifact_id":"e5b3ee84-4a83-491c-8cf6-3b9c60a0f87e",
  "source_id":"65e204f6-ff8b-42e2-898d-56c3723d6dc0",
  "timestamp":1657893583,
  "operation":"AddArtifact"
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
 DEBUG pyrsia::java::maven2::handlers::maven_artifacts > Requesting artifact for id commons-codec/commons-codec/1.15/commons-codec-1.15.jar
 INFO  pyrsia::artifact_service::storage               > An artifact is being pulled from the artifact manager a90e6792-4f2e-4ccc-8479-d935431e28ec
 DEBUG pyrsia::artifact_service::storage               > Pulling artifact from /private/tmp/pyrsia/a90e6792-4f2e-4ccc-8479-d935431e28ec.file
 INFO  pyrsia_registry                                 > 127.0.0.1:55273 "GET /maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.jar HTTP/1.1" 200 "-" "Apache-Maven/3.8.5 (Java 1.8.0_332; Mac OS X 12.4)" 23.00275ms
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
    "id": "d1e2ee25-5b8e-41a2-b36b-caa735969b94",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.pom.sha1",
    "artifact_hash": "5dc11ea7cfa14ac8e84250b21166b098f1ce57ba3316ba5cae589269b37ce54a",
    "source_hash": "",
    "artifact_id": "7ce014b5-9e0e-4d57-8783-a53e0f6ca5b7",
    "source_id": "07d57167-6d4e-4dd6-9d94-9604ea4d3981",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "daee9753-19eb-4494-bb23-3bf3122b24bf",
    "node_public_key": "ac611d95-01dd-43ff-96a1-257150531559"
  },
  {
    "id": "cda4e2fb-84ca-4598-a41e-b95b5d0dc78b",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-test-sources.jar",
    "artifact_hash": "997a5e8ed8c37e0c3dcdf20e76db11279c055a20c3875731390c7b8008912c4f",
    "source_hash": "",
    "artifact_id": "980bce73-6abd-4a27-905d-7158e078fbc6",
    "source_id": "599d833b-c9e4-4020-8cd6-2ad5c63b5e8c",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "ed2c2530-d57e-48e4-9dbe-544fe8afe54c",
    "node_public_key": "600ef060-8625-4591-a6c7-424dfe523b6c"
  },
  {
    "id": "41d861a0-b239-414f-94af-09afd4e91914",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-tests.jar",
    "artifact_hash": "dbf6348c24ff9031fed4d03f69562d6f72f22fd8df60c446addcd6be292479c2",
    "source_hash": "",
    "artifact_id": "5be26d34-6700-4c70-862e-b6c2152070a6",
    "source_id": "6f15dec9-6e59-48c6-837d-381f5245deea",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "f157a26a-850c-440e-919e-f6f86964a79c",
    "node_public_key": "1547d1f7-8bc0-49e7-a0fd-87a23ce20b77"
  },
  {
    "id": "1d43164c-eff9-41fd-9b9b-0e9c31bc7df3",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-sources.jar.sha1",
    "artifact_hash": "944a6c7643cc0ef821d38afdb576c3a3ea2d2308c053eb8e0bcbbc624766e345",
    "source_hash": "",
    "artifact_id": "937e2ef2-c124-4bfc-8229-0d573cb75cd5",
    "source_id": "b01def26-5fe1-4510-825f-a8fa6993e144",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "2520612b-d0b2-45f3-8dd3-c259283527b0",
    "node_public_key": "c60c2352-9ab5-4d1c-ac34-0e2d755e6caf"
  },
  {
    "id": "3ce2490f-0264-44a7-824b-9bdf9379be59",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-tests.jar.md5",
    "artifact_hash": "6875ce35f7aa1dffbbd390393c30056b06139a5d822a66ee2fe431366c542a7d",
    "source_hash": "",
    "artifact_id": "22de3073-5afb-49ed-a7cc-ec0c4fa1e86f",
    "source_id": "da789ef4-c9c3-4799-9b8b-7a05d3412fcd",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "1aed4d9e-4cae-4b5b-a4c6-01c6c4796ab7",
    "node_public_key": "1338c2c8-6d25-46ed-b7b1-a8a61244d9ad"
  },
  {
    "id": "a7718180-1a5c-4894-8e90-4f16f4f6feb7",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-tests.jar.sha1",
    "artifact_hash": "00540debf6f0091544b5daacea87fe4d6475f4abea566f19d1e7c0d062ca1016",
    "source_hash": "",
    "artifact_id": "9df96d6d-c5dd-40f8-bd5d-56531a47a737",
    "source_id": "1cde52f6-48ba-4b25-bb81-0620bf59630c",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "18409184-cb89-46d3-8119-5642fd4d0aea",
    "node_public_key": "f06359c3-42de-4bb3-a7fa-59ed50099ee9"
  },
  {
    "id": "c20167f6-2d96-4214-b6a8-7942c034167b",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-test-sources.jar.sha1",
    "artifact_hash": "a9be7ceaf4962f4c897239972a1dac711ca7570c2cf47a8b09b7b343950caf41",
    "source_hash": "",
    "artifact_id": "b632f752-707c-4a10-b113-02de9ea495ac",
    "source_id": "ce9b2f47-a579-44a6-8bd8-d80850085e84",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "a3690d7d-f450-41ab-b959-2d8de3bd74d8",
    "node_public_key": "05f364f4-9081-4fda-b37b-1043d435afbc"
  },
  {
    "id": "e447df77-f809-43dc-b40a-4fce58c3f6a4",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-sources.jar.md5",
    "artifact_hash": "53290bdbc449ae0e0230ab9f4d1d49043a3b67d0c40f49f1b6323188da2f0200",
    "source_hash": "",
    "artifact_id": "0af768c4-ea2d-4556-b9b4-b2ac81ab732c",
    "source_id": "b1104a19-357a-4ca8-aba5-c6d3a3944886",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "424a20ae-d1a2-45d1-977e-87463108d616",
    "node_public_key": "5312da51-99d5-496e-9bd5-b955bece15f8"
  },
  {
    "id": "0ff4df3e-c908-479f-850b-9bdcea2cc367",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.jar.sha1",
    "artifact_hash": "49258e0f1920c7303d0e31b31cf9d6157ca2beb1166c0c0576c0f0a5ab0c03d1",
    "source_hash": "",
    "artifact_id": "0ffa5544-2fe3-415b-9bd6-73bc98bc39b5",
    "source_id": "241c8aa1-acee-40be-be70-ba38d17d3d00",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "3e4b17f3-5b4a-4def-8598-8bd67c827092",
    "node_public_key": "f03deb88-c45f-4382-8662-96603a6caec6"
  },
  {
    "id": "2fee254e-db03-4e74-99ca-2d83d3302ec9",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.pom.md5",
    "artifact_hash": "ff89aba3ea6e2655feba41fdff2d8388b09f421ca6ca0ff5c49dbc24e53ae86a",
    "source_hash": "",
    "artifact_id": "1b674cc8-4bb1-47d7-adfb-9fe167afe270",
    "source_id": "2bddcb16-8c2e-46a1-b3c9-afd04bb2bb13",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "5b9555ea-1ba8-4813-ab37-4f525c7a3640",
    "node_public_key": "800d458b-f12b-4936-ad1a-1bfec7ca16e6"
  },
  {
    "id": "e7f10718-040c-4850-badf-e0a07afda381",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-test-sources.jar.md5",
    "artifact_hash": "e74339769bffce344ee5a4645a64466ad4ffe2002cf774285e3f20fdb110b503",
    "source_hash": "",
    "artifact_id": "dfe7e07d-bd42-4b26-b8df-2449d76a25a5",
    "source_id": "de40f6ef-32ed-4016-adaf-7cba2036ade9",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "21b15080-a6cf-4e3b-bcbd-d80a7509d8d2",
    "node_public_key": "15d97cc8-50ca-4651-bda0-f0133065d816"
  },
  {
    "id": "26756f3f-c719-422d-b3f6-b1d88f8b6f97",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.jar",
    "artifact_hash": "45307d7466bcb1f0cd52dd5df3c313f2c189586695b5d199d1a0a549f92bc50d",
    "source_hash": "",
    "artifact_id": "0c41a4bc-09b3-4e32-8c6b-dec54ba13d89",
    "source_id": "ff2f3717-d1c0-4a68-96f9-9dba20245aa3",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "46f0e541-1524-4973-b6d5-7499fb8fb3a8",
    "node_public_key": "17b7ee56-931c-4cd1-abc4-5b24d85c972a"
  },
  {
    "id": "1b548a2c-d6a4-45ab-95ff-7855bee9049b",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15-sources.jar",
    "artifact_hash": "930b528fc1cc6ad19b719c8f79fbf494814b3572f4df9f2555186ce2527a3116",
    "source_hash": "",
    "artifact_id": "04978768-8883-491e-9110-59a2b9710f34",
    "source_id": "6fc1e9c8-af1d-49ab-b870-247af461fc2a",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "ee39ca4d-b757-4329-8cc1-6e10319cb6a3",
    "node_public_key": "4d9d5ea0-9c82-4905-8423-2d7109993e19"
  },
  {
    "id": "e916ae11-cecf-422a-8123-a8d852ce2a90",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.jar.md5",
    "artifact_hash": "9e433debcc9932e38e6187e17a112f78f2fddc47419d27c4926776505528dfee",
    "source_hash": "",
    "artifact_id": "af797f13-9668-48c8-ab92-366c3b915167",
    "source_id": "d24a1dc1-4c39-4453-85fd-fd4872e8a857",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "9492e952-be4e-4051-be3d-2e3041075f97",
    "node_public_key": "bb225d61-9fc9-4433-a9ad-adab50404e56"
  },
  {
    "id": "70ff4e9e-a1fe-4d64-8c24-af0c8dc0a305",
    "package_type": "Maven2",
    "package_specific_id": "commons-codec:commons-codec:1.15",
    "num_artifacts": 15,
    "package_specific_artifact_id": "commons-codec/commons-codec/1.15/commons-codec-1.15.pom",
    "artifact_hash": "c86ee198a35a3715487860f419cbf642e7e4d9e8714777947dbe6a4e3a20ab58",
    "source_hash": "",
    "artifact_id": "c96317aa-b20c-4f67-aedf-c4be4fa912ff",
    "source_id": "0e316ffb-68c7-4e84-9352-95604582ba11",
    "timestamp": 1660908540,
    "operation": "AddArtifact",
    "node_id": "c08e0a25-1f79-4c0e-9f4d-f9dbc4d2e736",
    "node_public_key": "9563043e-7e2b-4e3c-b64b-cbed3e9da32d"
  }
]
```
