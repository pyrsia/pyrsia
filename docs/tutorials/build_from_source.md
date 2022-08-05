# How to setup a Pyrsia node to build from source

> **Warning:** The build-from-source demo is still work-in-progress.

This tutorial describes how to setup a Pyrsia node that can build artifacts from
source with the goal to publish them in the Pyrsia network.

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

See the [architecture and use-cases](../developers/pyrsia-architecture-and-use-cases.md)
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
cargo run --package pyrsia_node -- --pipeline-service-endpoint http://localhost:8080
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

In case you want to build a different artifact from source than already available
in the mapping, feel free to create a pull request to add it to the mapping repository.

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
export JAVA_HOME=/Library/Java/JavaVirtualMachines/jdk1.8.0_45.jdk/Contents/Home
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

## Trigger a build from source for a given artifact

In this demo we trigger a build for `commons-codec:commons-codec:1.15`.
The mapping repository already contains the [source repository mapping](https://github.com/pyrsia/pyrsia-mappings/blob/main/Maven2/commons-codec/commons-codec/1.15/commons-codec-1.15.mapping).

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

When consensus has been reached, a transparency log is created for each build artifact.

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
