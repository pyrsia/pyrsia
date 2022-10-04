---
sidebar_position: 6
---

# Maven user scenarios

__This assumes that Pyrsia network has a set of well known java binaries (jars) that have been built from source and are available on the cloud node. Adding new java binaries to the Pyrsia network is not in scope for this use case.__

Please refer to [mvn scenarios](<https://lucid.app/lucidchart/d5254e8f-69c5-49d9-acae-91aff72382e2/edit?viewport_loc=42%2C69%2C1912%2C1107%2C0_0&invitationId=inv_a3c2b95f-f098-4758-8a6d-016f569b572c>#)

- As a user I can use a Java build tool like Maven to download Maven artifacts from the Pyrsia network.

- As a user of maven, I should be able to run maven commands and receive the dependencies for my java project from the Pyrsia network instead of the Maven Central Repository.
  - ( For instance you could use `mvn -U dependency:resolve` to test this feature. This command will look for the dependencies on the network and should go to Pyrsia before looking for them in Maven Central)
  The user needs to configure their repository-order as follows:
  - A Maven project can contain a number of remote repositories (via pom.xml, super pom, or local/global settings.xml). By default the local repository at ~/.m2 and Maven Central are always included. The precedence of repositories is defined here: <https://maven.apache.org/guides/mini/guide-multiple-repositories.html#repository-order>.

Adding the Pyrsia repository to the pom/settings will allow downloading from Pyrsia first (if the artifact is not already downloaded into ~/.m2, that is), and if not present, then from Maven Central.)

- In case the dependencies are not yet available on the Pyrsia network, Maven should fallback to retrieving those dependencies from Maven Central.

- If the dependencies are available on the Pyrsia network, they should then be downloaded to the peer node and any future requests for the dependency should fetch them from the peer node. Future requests for the same version of the dependency should not cause network traffic if they are available on the peer node.

- As a user of pyrsia - I should be able to use the `inspect-log` pyrsia cli command to look at the transparency log for the jar I am interested in.
