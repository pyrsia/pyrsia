# Frequently Asked Questions

## Is Pyrsia a new package manager?

Pyrsia connects to many package managers and delivers the packages where they are used. Package repositories do a good job of storing packages and performing best effort delivery of those packages. Centralize package repositories are susceptible to single points of failures or single point attacks which could render them unusable for a period of time until they are revived  (https://status.npmjs.org/uptime?page=7 shows that NPM had major outages that lasted for hours together thus stalling any dependent software delivery efforts). Pyrsia is intended to fix these central failure issues and provide more resilience by relying on the P2P mechanism to store and deliver packages.

Along with all this Pyrsia will also connect the git-sha with the binary output and provide an independent infrastructure to build binaries. Building the binaries on independent infrastructure is expected to build trust that the package was in fact not affected by attacks (B-H from the SLSA diagram)

In the broad sense Pyrsia may be referred to by users as the package manager, because that is what they interact with, but Pyrsia aims to solve the problem of delivery packages reliably instead of just being a repository.

## What is missing in existing Package Managers - why do we need Pyrsia?

Existing Community Package managers have the issues we talked about in the presentation. Single points of failure, owned by central authorities with no control to the general community.
Pyrsia eliminates single points of failure, distributes control to the community and aims to empower under the guidance of OpenSSF.

## For fetching images, how will that work for CLIs such as docker, podman etc?

We know from experience that it is hard to change all the CI pipelines that already use `docker pull`. So we have integrated docker in a way where Pyrsia can be used in the background to provide features like - reliability, distribution network based on p2p to speed up downloads, and provides a provenance chain(transparency log) that can be used to produce SBOMs.

For docker you still continue to use docker commands and pyrsia nodes act as a conduit. So nothing changes on your CI system (since there is a large installed base) but get the benefits of P2P and also network fault tolerance (so if npm is down your CD system can still work).

## Why not use sigstore/rekor for an immutable ledger?

Signing will be part of how the ledger is recorded (and will include support for sigstore/rekor and Notary V2) But more importantly other than signing the ledger will provide a transparency log - for provenance.

## I heard that Pyrsia uses a blockchain? Aren't blockchains used for CryptoCurrencies and take a lot of time and energy for consensus?

This is currently going through the design phase. But remote verification is the key requirement. The scale of the network as well as the security promise are currently being balanced and we are working on a Proof of Concept to prove out a simple Proof of Authority mechanism and evolve it as we scale.
I think it's BFT, so relies on trusted nodes rather than PoW or PoS or sec enclaves

## This seems to be trying to tackle 2 different aspects: distribution and attestation & validation - I don't think blockchain is a good fit for the former - blockchain doesn't deal with big amount of data very well -  I would think the sweet spot is more on the latter

Response: Our goal is to keep the blockchain lean so that it contains provenance information instead of packages.

Response: The blockchain is being used to provide a distributed ledger that can be shared over the network. This ledger is basically the transparency log that is can repair after network partitions. The distribution of packages uses the basic file distribution mechanisms from libp2p.

From Sudhindra Rao to Everyone 08:58 AM
Pyrsia is not a package manager - but a very efficient distribution mechanism that is resilient to network partitions, and also provides an independent build mechanism to ensure that the developer machine is not the one we rely on for quality.

From Luke lhinds@protonmail.com to Everyone 08:58 AM
Comment: it is a package manager if you have a CLI that fetches artifacts 🙂

"public" blockchains can be public-read not not public-write. Public-write does require a sybil-resistent mechanism like PoW/PoS or POET-like things. But public-read, trusted-node-write can use a BFT-variant consensus mechanism (or others)
s/not not/but not/

Response: Agree on this point. So we are carefully consider how we performed trusted/certified writes to the blockchain but allow public-read/consumption of the blockchain
