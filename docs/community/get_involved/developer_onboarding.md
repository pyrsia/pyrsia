# Developer Onboarding

If you are interested to contribute in Pyrsia project, here are a few important resources which might be helpful on your onboarding journey.

## Join, Subscribe & Access

Request membership access to join [googlegroup for Pyrsia](https://groups.google.com/g/pyrsia). This will allow you to
receive emails and calendar invites send to Pyrsia Googlegroup.

We collaborate on the project using Google Drive's shared directory
[Pyrsia](https://drive.google.com/drive/folders/1vXO2YUjdyFXfjNGiKn82kUQ5ePrzp_jV?usp=share_link)

Join Slack Workspace [Continuous Delivery Foundation](https://cdeliveryfdn.slack.com/). Join a few Slack channels #pyrsia, #pyrsia-team, #pyrsia-alert, #pyrsia-notifications

Subscribe [PyrsiaOSS](https://www.youtube.com/channel/UClPQKloIElvJk7EdSST3W5g) Youtube channel for latest release, to know what is happening.

## Technical Resources and a Few Important Concepts

Make yourself familiar with some Rust knowledge and a few basic underlying concepts what we are using to build Pyrsia.
To know more about project's architecture and use cases follow the link to
[Pyrsia architecture and use cases](https://github.com/pyrsia/pyrsia/blob/main/docs/developers/pyrsia-architecture-and-use-cases.md).

### Rust

Here a few Rust resources to start with

- [Getting Started Rust Programming Language](https://www.rust-lang.org/learn/get-started)
- [rustlings 🦀❤️](https://github.com/rust-lang/rustlings) small exercises to get you used to Rust coding.
- [The Rust Programming Language](https://doc.rust-lang.org/stable/book/) book
- Chapter by Chapter [The Rust Lang Book Video Playlist](https://www.youtube.com/playlist?list=PLai5B987bZ9CoVR-QEIN9foz4QCJ0H2Y8)
- [The Cargo Book](https://doc.rust-lang.org/cargo/index.html) - Rust Package Manager
- A collection of runnable example at [Rust By Example](https://doc.rust-lang.org/stable/rust-by-example/)
- In Pyrsia we use Asynchronous Programing in quite a few places. You would find important concepts at [Asynchronous Programming in Rust](https://rust-lang.github.io/async-book/)

### Third Party Frameworks & Libraries

To build Pyrsia's decentralized network we made some architectural design decisions for **_peer to peer_**
communications. This requires us to use a few protocols and their third party implementations. Here is a list of few
of them along with useful links to relevant documents.

- Pyrsia uses libp2p library for building the decentralized network. [libp2p concepts](https://docs.libp2p.io/concepts/) link to have better understanding on Pyrsia.
  - [libp2p in Rust](https://github.com/libp2p/rust-libp2p)
  - Few [libp2p examples](https://github.com/libp2p/rust-libp2p/tree/master/examples)
- Using [Tokio](https://tokio.rs/tokio/tutorial)'s asynchronous runtime for the Rust programming language.
- Ed25519 high-speed high-security public-key signature system
  - [Ed25519](https://ed25519.cr.yp.to/index.html)
  - [Why Ed25519](https://sectigostore.com/blog/ecdsa-vs-rsa-everything-you-need-to-know/)
- Aleph Consensus Protocol
  - [Aleph BFT](https://cardinal-cryptography.github.io/AlephBFT/what_is_aleph_bft.html) concepts.
  - [AlephBFT Consensus](https://docs.alephzero.org/aleph-zero/explore/alephbft-consensus)
  - Rust implementation at [Cardinal-Cryptography/AlephBFT](https://github.com/Cardinal-Cryptography/AlephBFT)
- [Distributed Hash Table (DHT) and P2P technologies](https://www.sobyte.net/post/2022-01/dht-and-p2p/)
