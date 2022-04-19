/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

pub mod command;

use crate::network::artifact_protocol::ArtifactResponse;
use crate::network::client::command::Command;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use libp2p::core::{Multiaddr, PeerId};
use libp2p::request_response::ResponseChannel;
use log::debug;
use std::collections::HashSet;
use std::error;

/// The `Client` provides entry points to interact with the libp2p swarm.
#[derive(Clone)]
pub struct Client {
    pub sender: mpsc::Sender<Command>,
    pub local_peer_id: PeerId,
}

impl Client {
    /// Instruct the swarm to start listening on the specified address.
    pub async fn listen(&mut self, addr: &Multiaddr) -> Result<(), Box<dyn error::Error + Send>> {
        debug!("p2p::Client::listen {:?}", addr);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Listen {
                addr: addr.clone(),
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Dial a peer with the specified address.
    pub async fn dial(
        &mut self,
        peer_addr: &Multiaddr,
    ) -> Result<(), Box<dyn error::Error + Send>> {
        debug!("p2p::Client::dial {:?}", peer_addr);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Dial {
                peer_addr: peer_addr.clone(),
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// List the peers that this node is connected to.
    pub async fn list_peers(&mut self) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::ListPeers {
                peer_id: self.local_peer_id,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Inform the swarm that this node is currently a
    /// provider of the artifact with the specified `hash`.
    pub async fn provide(&mut self, hash: &str) {
        debug!("p2p::Client::provide {:?}", hash);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Provide {
                hash: String::from(hash),
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// List all peers in the swarm that are providing
    /// the artifact with the specified `hash`.
    pub async fn list_providers(&mut self, hash: &str) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::ListProviders {
                hash: String::from(hash),
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Request an artifact with the specified `hash` from
    /// the swarm.
    pub async fn request_artifact(
        &mut self,
        peer: &PeerId,
        hash: &str,
    ) -> Result<Vec<u8>, Box<dyn error::Error + Send>> {
        debug!("p2p::Client::request_artifact {:?}: {:?}", peer, hash);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestArtifact {
                hash: String::from(hash),
                peer: *peer,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    /// Put the artifact as a response to an incoming artifact
    /// request.
    pub async fn respond_artifact(
        &mut self,
        artifact: Vec<u8>,
        channel: ResponseChannel<ArtifactResponse>,
    ) {
        debug!("p2p::Client::respond_artifact size={:?}", artifact.len());

        self.sender
            .send(Command::RespondArtifact { artifact, channel })
            .await
            .expect("Command receiver not to be dropped.");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    #[tokio::test]
    async fn test_listen() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let address: Multiaddr = "/ip4/127.0.0.1".parse().unwrap();
        let cloned_address = address.clone();
        tokio::spawn(async move { client.listen(&address).await });

        futures::select! {
            command = receiver.next() => match command {
                Some(Command::Listen { addr, sender }) => {
                    assert_eq!(addr, cloned_address);
                    let _ = sender.send(Ok(()));
                },
                _ => panic!("Command must match Command::Listen")
            }
        }
    }

    #[tokio::test]
    async fn test_dial() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let address: Multiaddr = "/ip4/127.0.0.1".parse().unwrap();
        let cloned_address = address.clone();
        tokio::spawn(async move { client.dial(&address).await });

        futures::select! {
            command = receiver.next() => match command {
                Some(Command::Dial { peer_addr, sender }) => {
                    assert_eq!(peer_addr, cloned_address);
                    let _ = sender.send(Ok(()));
                },
                _ => panic!("Command must match Command::Dial")
            }
        }
    }

    #[tokio::test]
    async fn test_list_peers() {
        let (sender, mut receiver) = mpsc::channel(1);

        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let mut client = Client {
            sender,
            local_peer_id,
        };

        tokio::spawn(async move { client.list_peers().await });

        futures::select! {
            command = receiver.next() => match command {
                Some(Command::ListPeers { peer_id, sender }) => {
                    assert_eq!(peer_id, local_peer_id);
                    let _ = sender.send(Default::default());
                },
                _ => panic!("Command must match Command::ListPeers")
            }
        }
    }

    #[tokio::test]
    async fn test_provide() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let random_hash: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let cloned_random_hash = random_hash.clone();
        tokio::spawn(async move { client.provide(&random_hash).await });

        futures::select! {
            command = receiver.next() => match command {
                Some(Command::Provide { hash, sender }) => {
                    assert_eq!(hash, cloned_random_hash);
                    let _ = sender.send(());
                },
                _ => panic!("Command must match Command::Provide")
            }
        }
    }

    #[tokio::test]
    async fn test_list_providers() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let random_hash: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let cloned_random_hash = random_hash.clone();
        tokio::spawn(async move { client.list_providers(&random_hash).await });

        futures::select! {
            command = receiver.next() => match command {
                Some(Command::ListProviders { hash, sender }) => {
                    assert_eq!(hash, cloned_random_hash);
                    let _ = sender.send(Default::default());
                },
                _ => panic!("Command must match Command::ListProviders")
            }
        }
    }

    #[tokio::test]
    async fn test_request_artifact() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let other_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let random_hash: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let cloned_random_hash = random_hash.clone();
        tokio::spawn(async move { client.request_artifact(&other_peer_id, &random_hash).await });

        futures::select! {
            command = receiver.next() => match command {
                Some(Command::RequestArtifact { peer, hash, sender }) => {
                    assert_eq!(peer, other_peer_id);
                    assert_eq!(hash, cloned_random_hash);
                    let _ = sender.send(Ok(vec![]));
                },
                _ => panic!("Command must match Command::RequestArtifact")
            }
        }
    }
}
