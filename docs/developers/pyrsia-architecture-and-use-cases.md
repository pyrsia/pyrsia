# Pyrsia architecture and use cases

> **Warning:** This document is a work-in-progress.

## Concepts used in this document

- Pyrsia network: a peer-to-peer network of interconnected nodes exchanging software
  packages and transparency logs.
- Node: a process participating the Pyrsia network, either as an authorized node
  or a regular node.
- Authorized node: a Node authorized to participate in the consensus algorithm to
  verify transactions
- Regular node: a Node using the network to download and validate software packages
- Transaction: an operation in the transparency log. e.g. `add_artifact`,
  `add_node`, ...
- Consensus: consensus establishes the agreement between authorized nodes that a
  transaction is valid.
- Artifact: a single file that can be retrieved from the Pyrsia network. It does
  not necessarily coincide with package specific artifacts.
- Authorized node admin: the person who can administer an authorized node

## Introduction

The Pyrsia network's first aim is to distribute software packages without central
authority. Instead, it relies on a set of designated authorized nodes that reach
consensus about the publication of software packages.

Those software packages originate from a source repository with build instructions.
All authorized nodes perform the necessary build steps and reach consensus if the
produced build is identical (or at least identical for the parts that matter).

**Note**: The Docker build pipeline will leverage Docker Hub builds for the
official Docker images.

The Pyrsia network also distributes transparency logs so every node in the network
can verify a downloaded artifact.

## High level user stories

- As a user I can use Docker client to pull an official Docker image from the Pyrsia
  network.
- As a user I can use a Java build tool like Maven to download Maven artifacts from
  the Pyrsia network.
- As a user I can use the Pyrsia CLI to show the transparency log based on search
  parameters.
- New authorized nodes can be added to the Pyrsia network.
- As a user I can request a build from source of a specific artifact, so it is
  added to the Pyrsia network.

## Pyrsia network overview

The following diagram shows three authorized nodes, each with their own build pipelines.
In a real network, we expect tens or even hundreds of authorized nodes.

![Pyrsia component diagram](pyrsia-network1.png)

The next diagram show the same authorized nodes. But next to those, a larger number
of regular nodes have now joined the network as well. While regular nodes don't
participate in the consensus mechanism, they do participate in the distribution
of artifacts and transparency logs. They play a crucial role in the performance
of the Pyrsia network.

![Pyrsia component diagram](pyrsia-network2.png)

## Pyrsia node architecture

![Pyrsia component diagram](pyrsia-node-high-level-components.png)

### Package type ecosystems

A Pyrsia node contains several connectors to specific package type ecosystems
like Docker or Maven. The ecosystem connectors allow the existing tooling of a
specific package type to seamlessly integrate with Pyrsia. e.g. the Docker
repository service implements a subset of the Docker Registry API or the Java
repository service implements a subset of the Maven repository API.

The end goal of a such a service is always a frictionless integration of Pyrsia
in the developer's workflow.

### Pyrsia CLI API

The Pyrsia CLI API is the entry point into the Pyrsia node for the Pyrsia
command line tool. It supports all kinds of management operations (requesting
status information about the local artifact storage, information about the peers
in the p2p network) and inspecting transparency logs.

### Artifact Service

The artifact service is the component that can store, retrieve and verify Pyrsia
artifacts.

In the artifact consumption use cases, this component offers an abstract way of
dealing with Pyrsia artifacts for the specific ecosystem connectors. It will
handle get_artifact requests and perform all necessary steps to find, retrieve
and validate artifacts either locally or using the p2p network.

In the publication use cases (on authorized nodes) this component is responsible
to drive build triggers: it will request a build from source at the Build Service,
use the Transparency Log Service to add an `add_artifact` transaction, and when
consensus is reached, it will store the artifact and make it available in the p2p
network.

### p2p

The p2p component offers an interface to the peer-to-peer network. This component
heavily relies on libp2p and bundles everything that is required to set up and
maintain a p2p network between Pyrsia nodes, allowing them to exchange messages,
artifacts and transparency logs.

### Transparency Log Service

This component is used by the Artifact Service to store and retrieve transparency
log information about an artifact.

It uses the Blockchain component to retrieve transactions and to reach consensus
on the publication of new transactions. It uses a local database to store and index
transaction information for easy access.

### Blockchain

This component offers an interface to store and retrieve immutable transaction
logs, and distribute them across all peers.

Before transactions can be added to the blockchain, consensus needs to be reached
using a fault-tolerant consensus algorithm, because:

- A majority of (authorized) nodes must be able to agree to the same result
- A small number of faulty (authorized) nodes must not be able to influence the
  result
- A small number of faulty (authorized) nodes must not be able to slow down the
  system or make it stop working

### Build service

The build service is a component only used by authorized nodes. It is the entry
point to the authorized node's build pipeline infrastructure and takes a Transaction
as input, including:

- the package type
- the source repo url

Based on the package type and the build spec of the artifact, the build service
will then invoke a build using a suitable pipeline.

## Technical stories and details

- As a user I can use Docker client to pull an official Docker image from the Pyrsia
  network.

  - a Pyrsia node handles incoming requests from a Docker client **[DOCKER_REGISTRY]**

    This is the implementation of the Docker Registry API so a Docker client can
    seamlessly integrate with the Pyrsia node.

- As a user I can use a Java build tool like Maven to download Maven artifacts
  from the Pyrsia network.

  - a Pyrsia node handles incoming requests from a maven client **[JAVA_REPOSITORY]**

    This is the implementation of the Maven Repository API so several Java build
    tools can seamlessly integrate with the Pyrsia node.

- When an artifact is requested, the node verifies the existence in the Pyrsia
  network, and downloads it if necessary. **[ARTIFACT_SERVICE]**

  When an artifact is requested, the Artifact Service  will query the transparency
  log component. If the artifact exists (so if a log exists) the transparency log
  will contain a reference to the required p2p file.
  The artifact service will then lookup this file in its local storage, or download
  it from the p2p network.

- Any Pyrsia node that downloaded an artifact provides that artifact on the
  network for other nodes to download. **[ARTIFACT_SERVICE]**

- As a user I can configure my Pyrsia node to limit the network bandwidth usage
  or even disable downloads from other nodes. **[ARTIFACT_SERVICE]**

- As a Pyrsia node, I provide my locally stored artifacts in the Pyrsia network
  at boot. **[ARTIFACT_SERVICE]**

- As a user behind a NAT router, my node can participate in het Pyrsia network
  **[P2P]**

  In order to participate in a distributed peer-to-peer network, nodes need to be
  reachable by other nodes. This can be a challenge when a node is run behind a
  NAT router. There are multiple ways of NAT traversal like TCP hole punching
  that Pyrsia will try to accomplish. However, Pyrsia nodes will not relay
  traffic if none of the other traversal methods worked. In that case, the
  Pyrsia node can simply download artifacts from one or more of the authorized
  nodes, which will by definition always contain all the data.

- As a user I can use the Pyrsia CLI to show the transparency log. **[CLI]**

  including search on author/dependencies/...

- New authorized nodes can be added to the Pyrsia network. **[CLI]**
  **[TRANSPARENCY_LOG]** **[BLOCKCHAIN]**

  - As an authorized node admin I can add a candidate authorized node

    The authorized node marks the new node id as an authorized node 'candidate'
    and creates an `AddNode` transaction request and waits for consensus.
    Consensus might not be reached yet, but the authorized node keeps the
    candidate so a future transaction request from another authorized node might
    reach consensus.

- As a user I can request the addition of an official Docker Hub image to the
  Pyrsia network. **[CLI]** **[ARTIFACT_MANAGER]**

  - The Pyrsia node accepts "Docker image add requests" and as a result starts
    build pipeline and adds a Transaction request.

- As a user I can request a build from source of a specific artifact, so it is
  added to the Pyrsia network **[CLI]** **[ARTIFACT_MANAGER]**

  - The Pyrsia node accepts " Build from source requests" and as a result
    starts build pipeline and adds a Transaction request.

- When a Transaction request is received all authorized nodes participate in the
  consensus mechanism **[BLOCKCHAIN]**

  Other authorized nodes validate transactions based on the transaction's operation
  type. Examples of transaction operations:

  - `AddNode`: to add a new authorized node. see 'AddNode transaction requests
     are handled'
  - `RemoveNode`: to add a new authorized node. see 'RemoveNode transaction
     requests are handled'
  - `AddArtifact`: to add a new artifact. see 'AddArtifact transaction
     requests are handled'

- AddNode transaction requests are handled **[BLOCKCHAIN]**
  an `AddNode` transaction requests follows this procedure:

  - was the node previously marked as an authorized node candidate?
  - if yes, the authorized node answers positively in the consensus algorithm

- RemoveNode transaction requests are handled **[BLOCKCHAIN]**

  - was the node previously marked as an authorized node candidate for removal?
  - if yes, the authorized node answers positively in the consensus algorithm

- AddArtifact transaction requests are handled **[BLOCKCHAIN]**
  The AddArtifact transaction request triggers a build verification using the
  Build Service. The response of the Build Service defines the authorized
  node's answer in the consensus algorithm.

- When consensus is reached, the transaction is committed to the blockchain.
  **[BLOCKCHAIN]**

  As a result, all nodes must receive new transactions. The authorized nodes store
  the artifact locally and provide it in the p2p network.

- When a build is started, the Build Service finds a corresponding build pipeline
  suitable to run the build. **[BUILD_SERVICE]**

- When a build result is returned from a pipeline, the build service verifies
  the generated build (part 1: for reproducible builds) **[BUILD_SERVICE]**

- When a build result is returned from a pipeline, the build service verifies
  the generated build by doing a semantic analysis (part 2: for non-reproducible
  builds) **[BUILD_SERVICE]**

- On any Pyrsia node, when a new transaction is received, it is added to the
  transparency log so it can be used in verification scenarios
  **[TRANSPARENCY_LOG]**

- As a Pyrsia node, I make sure the transparency log is up-to-date when I
  boot. **[TRANSPARENCY_LOG]**

- As a Pyrsia node, I make sure the transparency log is kept up-to-date while
  running. **[TRANSPARENCY_LOG]**

- As a Pyrsia node, I can download an artifact from multiple other nodes
  simultaneously. **[P2P]**

- As a user I can measure the download via Pyrsia is faster than from a central
  repository. (benchmark) **[P2P]**

- As a user I can use Docker Desktop to install Pyrsia (Docker Desktop Pyrsia)
  **[INSTALLATION]**

- As a user I can use a package manager on Ubuntu to install Pyrsia **[INSTALLATION]**
