# mvn User scenarios

__This assumes that Pyrsia network has a set of well known java binaries(jars) that have been built from source and are available on the cloud node. Adding new java binaries to the Pyrsia network is not in scope for this use case.__

Please refer to [mvn update scenarios](<https://lucid.app/lucidchart/d5254e8f-69c5-49d9-acae-91aff72382e2/edit?viewport_loc=42%2C69%2C1912%2C1107%2C0_0&invitationId=inv_a3c2b95f-f098-4758-8a6d-016f569b572c>#)

- As a user I can use a Java build tool like Maven to download Maven artifacts from the Pyrsia network.

- As a user of maven, I should be able to run `mvn update` and receive the dependencies for my java project from the Pyrsia network instead of the maven central repository.

- In case the dependencies are not yet available on the Pyrsia network, maven should fallback to retrieving those dependencies from mavencentral.

- If the dependencies are available on the Pyrsia network, they should then be downloaded to the peer node and any future requests for the dependency should fetch them from the peer node. Future requests for the same version of the dependency should not cause network traffic if they are available on the peer node.

- As a user of pyrsia - I should be able to use the `inspect-log` pyrsia cli command to look at the transparency log for the jar I am interested in.
