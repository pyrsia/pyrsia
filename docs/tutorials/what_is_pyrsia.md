---
sidebar_position: 1
---

# What is Pyrsia

Pyrsia is a network of two types of **interconnected nodes**:

- Build nodes that build open source libraries from source
- Regular nodes that form the entry point to the network for build tools

On top of this, both type of nodes participate in the peer-to-peer distribution
of artifacts.

```mermaid
graph TD;
nA[Build node A]
nB[Build node B]
nC[Build node C]
style nA fill:#e3a04d
style nB fill:#e3a04d
style nC fill:#e3a04d
n1[Node 1]
n2[Node 2]
n3[Node 3]
n4[Node 4]
n5[Node 5]
n6[Node 6]
n7[Node 7]
n8[Node 8]

nA --- nB
nA --- nC
nB --- nC

n1 --- nA
n1 --- nC

n2 --- nA
n2 --- nB

n3 --- nA
n3 --- nC

n4 --- nA
n4 --- nC

nA --- n5
nB --- n5

nB --- n6
nC --- n6

nA --- n7
nC --- n7

nB --- n8
nC --- n8

n1 --- n8
n2 --- n7
n3 --- n6
n4 --- n5
```

The result is a high-available peer-to-peer network of trusted open source build
artifacts that no single entity can control.

## Why do I need Pyrsia?

As a developer relying on open source libraries, you greatly depend on:

- the entity performing a build of the open source library
- a central repository of build artifacts

This poses several risks:

- There is no way for you to verify the binary artifact is in fact the result of
  the given source so one malicious entity with publication access to a central
  repository can publish anything it wants.
- A central repository is controlled by a single entity, which could take
  decisions you do not agree with.

There is a solution to this problem: build all the open source libraries you
depend on yourself and publish them in a private local repository.

But why not use Pyrsia and work together instead?

## How does Pyrsia work?

Pyrsia works by designating a number of **independent build nodes**.

```mermaid
graph LR
nA[Build node A]
nB[Build node B]
nC[Build node C]
style nA fill:#e3a04d
style nB fill:#e3a04d
style nC fill:#e3a04d
nA --- nB
nA --- nC
nB --- nC
```

Those build nodes perform builds for all kinds of open source libraries (at this
stage, Pyrsia is building support for Docker images and Maven artifacts, but more package types will
be added soon). The trust in the built artifacts is reached because no single build
node can publish an artifact on its own. It needs to ask all the other build nodes
to verify the build (i.e. perform the same build and compare the result) and only
when an absolute majority verified the build (also known as 'consensus' is reached),
the artifact is published.

Pyrsia keeps a transparency log of those publications and distributes those in a
blockchain.

Any other node in the network can access these logs and use them to verify binary
artifacts that are downloaded from other nodes in the network. Whenever a node
downloads an artifact, it can choose to participate in the content distribution
and provide this artifact to other nodes itself.

```mermaid
graph LR
n1[Node 1] --- nA
n1 --- n2[Node 2]
nA[Build node A]
nB[Build node B]
nC[Build node C]
style nA fill:#e3a04d
style nB fill:#e3a04d
style nC fill:#e3a04d
nA --- nB
nA --- nC
nB --- nC
```

## FAQ

### Can I run a Pyrsia node?

Yes, have a look at [Quick Installation](/docs/tutorials/quick-installation.mdx)
and one of the package specific tutorials on [Docker](docker) or [Maven](maven).

### Do I have to participate in artifact distribution?

The more nodes participate in artifact distribution, the better of course. But if
you only want to run a Pyrsia node to consume artifacts, that works as well.
