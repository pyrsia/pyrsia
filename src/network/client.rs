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
use crate::network::idle_metric_protocol::{IdleMetricResponse, PeerMetrics};
use crate::node_api::model::cli::Status;
use libp2p::core::{Multiaddr, PeerId};
use libp2p::request_response::ResponseChannel;
use log::debug;
use std::collections::HashSet;
use tokio::sync::{mpsc, oneshot};

/* peer metrics support */
const PEER_METRIC_THRESHOLD: f64 = 0.5_f64;
#[derive(Clone, Debug, PartialEq, PartialOrd)]
struct IdleMetric {
    pub peer: PeerId,
    pub metric: f64,
}
/* peer metric support */

/// A utility struct for easily defining a hash from different
/// types that can be used as a provisioning key within the
/// libp2p swarm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArtifactHash {
    pub hash: String,
}

/// Construct an ArtifactHash from `String`
impl From<String> for ArtifactHash {
    fn from(hash: String) -> Self {
        ArtifactHash { hash }
    }
}

/// Construct an ArtifactHash from `&String`
impl From<&String> for ArtifactHash {
    fn from(hash: &String) -> Self {
        ArtifactHash { hash: hash.clone() }
    }
}

/// Construct an ArtifactHash from `&str`
impl From<&str> for ArtifactHash {
    fn from(hash: &str) -> Self {
        ArtifactHash {
            hash: String::from(hash),
        }
    }
}

/// The `Client` provides entry points to interact with the libp2p swarm.
#[derive(Clone)]
pub struct Client {
    pub sender: mpsc::Sender<Command>,
    pub local_peer_id: PeerId,
}

impl Client {
    // Add a probe address for AutoNAT discovery
    pub async fn add_probe_address(
        &mut self,
        peer_id: &PeerId,
        probe_addr: &Multiaddr,
    ) -> anyhow::Result<()> {
        debug!(
            "p2p::Client::add_probe_address {:?} {:?}",
            peer_id, probe_addr
        );

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::AddProbe {
                peer_id: *peer_id,
                probe_addr: probe_addr.clone(),
                sender,
            })
            .await?;
        Ok(receiver.await?)
    }

    /// Instruct the swarm to start listening on the specified address.
    pub async fn listen(&mut self, addr: &Multiaddr) -> anyhow::Result<()> {
        debug!("p2p::Client::listen {:?}", addr);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Listen {
                addr: addr.clone(),
                sender,
            })
            .await?;
        receiver.await?
    }

    /// Dial a peer with the specified address.
    pub async fn dial(&mut self, peer_id: &PeerId, peer_addr: &Multiaddr) -> anyhow::Result<()> {
        debug!("p2p::Client::dial {:?}", peer_addr);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Dial {
                peer_id: *peer_id,
                peer_addr: peer_addr.clone(),
                sender,
            })
            .await?;
        receiver.await?
    }

    /// List the peers that this node is connected to.
    pub async fn list_peers(&mut self) -> anyhow::Result<HashSet<PeerId>> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::ListPeers {
                peer_id: self.local_peer_id,
                sender,
            })
            .await?;
        Ok(receiver.await?)
    }

    /// Get the status of the node including nearby peers cnt and my peer addrs
    pub async fn status(&mut self) -> anyhow::Result<Status> {
        let (sender, receiver) = oneshot::channel();
        self.sender.send(Command::Status { sender }).await?;
        Ok(receiver.await?)
    }

    /// Inform the swarm that this node is currently a provider
    /// of the artifact with the specified `artifact_id`.
    pub async fn provide(&mut self, artifact_id: &str) -> anyhow::Result<()> {
        debug!("p2p::Client::provide {:?}", artifact_id);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Provide {
                artifact_id: artifact_id.to_owned(),
                sender,
            })
            .await?;
        Ok(receiver.await?)
    }

    /// List all peers in the swarm that are providing
    /// the artifact with the specified `artifact_id`.
    pub async fn list_providers(&mut self, artifact_id: &str) -> anyhow::Result<HashSet<PeerId>> {
        debug!("p2p::Client::list_providers {:?}", artifact_id);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::ListProviders {
                artifact_id: artifact_id.to_owned(),
                sender,
            })
            .await?;
        Ok(receiver.await?)
    }

    /// Request an artifact with the specified `artifact_id`
    /// from the swarm.
    pub async fn request_artifact(
        &mut self,
        peer: &PeerId,
        artifact_id: &str,
    ) -> anyhow::Result<Vec<u8>> {
        debug!(
            "p2p::Client::request_artifact {:?}: {:?}",
            peer, artifact_id
        );

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestArtifact {
                artifact_id: artifact_id.to_owned(),
                peer: *peer,
                sender,
            })
            .await?;
        receiver.await?
    }

    /// Put the artifact as a response to an incoming artifact
    /// request.
    pub async fn respond_artifact(
        &mut self,
        artifact: Vec<u8>,
        channel: ResponseChannel<ArtifactResponse>,
    ) -> anyhow::Result<()> {
        debug!("p2p::Client::respond_artifact size={:?}", artifact.len());

        self.sender
            .send(Command::RespondArtifact { artifact, channel })
            .await?;

        Ok(())
    }

    //get a peer with a low enough work load to download artifact otherwise the lowest work load of the set
    //TODO: chunk the peers to some limit to keep from shotgunning the network
    pub async fn get_idle_peer(
        &mut self,
        providers: HashSet<PeerId>,
    ) -> anyhow::Result<Option<PeerId>> {
        debug!(
            "p2p::Client::get_idle_peer() entered with {} peers",
            providers.len()
        );
        let mut idle_metrics: Vec<IdleMetric> = Vec::new();
        for peer in providers.iter() {
            let (sender, receiver) = oneshot::channel();
            self.sender
                .send(Command::RequestIdleMetric {
                    peer: *peer,
                    sender,
                })
                .await?;

            match receiver.await.expect("Sender not to be dropped.") {
                Ok(peer_metric) => {
                    let metric: f64 = f64::from_le_bytes(peer_metric.idle_metric);
                    let idle_metric = IdleMetric {
                        peer: *peer,
                        metric,
                    };
                    if idle_metric.metric < PEER_METRIC_THRESHOLD {
                        debug!(
                                "p2p::Client::get_idle_peer() Found peer with a below threshold idle value {}",
                                metric
                            );
                        return Ok(Some(idle_metric.peer));
                    } else {
                        debug!(
                            "p2p::Client::get_idle_peer() Pushing idle peer with value {}",
                            metric
                        );
                        idle_metrics.push(idle_metric);
                    }
                }
                Err(e) => {
                    debug!(
                            "p2p::Client::get_idle_peer() Unable to get peer metric for peer {} error {}",
                            peer, e
                        );
                }
            };
        }

        //sort the peers in ascending order according to their idle metric and return top of list
        idle_metrics.sort_by(|a, b| a.metric.partial_cmp(&b.metric).unwrap());
        Ok(idle_metrics.first().map(|idle_metric| idle_metric.peer))
    }

    pub async fn respond_idle_metric(
        &mut self,
        metric: PeerMetrics,
        channel: ResponseChannel<IdleMetricResponse>,
    ) -> anyhow::Result<()> {
        debug!(
            "p2p::Client::respond_idle_metric PeerMetrics metric ={:?}",
            metric
        );

        self.sender
            .send(Command::RespondIdleMetric { metric, channel })
            .await?;

        Ok(())
    }

    pub async fn request_block_update(
        &mut self,
        peer: &PeerId,
        block_ordinal: u64,
        block: Vec<u8>,
    ) -> anyhow::Result<Option<u64>> {
        debug!(
            "p2p::Client::request_blockchain {:?}: {:?}={:?}",
            peer,
            block_ordinal,
            block.clone(),
        );

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestBlockUpdate {
                block_ordinal,
                block,
                peer: *peer,
                sender,
            })
            .await?;
        receiver.await?
    }

    pub async fn respond_block_update(&mut self) -> anyhow::Result<()> {
        Ok(())
    }
}

#[cfg(all(test, not(tarpaulin_include)))]
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

        tokio::select! {
            command = receiver.recv() => match command {
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

        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let mut client = Client {
            sender,
            local_peer_id,
        };

        let address: Multiaddr = "/ip4/127.0.0.1".parse().unwrap();
        let cloned_address = address.clone();
        tokio::spawn(async move { client.dial(&local_peer_id, &address).await });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::Dial { peer_id, peer_addr, sender }) => {
                    assert_eq!(peer_id, local_peer_id);
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

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::ListPeers { peer_id, sender }) => {
                    assert_eq!(peer_id, local_peer_id);
                    let _ = sender.send(Default::default());
                },
                _ => panic!("Command must match Command::ListPeers")
            }
        }
    }

    #[tokio::test]
    async fn test_status() {
        let (sender, mut receiver) = mpsc::channel(1);

        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let mut client = Client {
            sender,
            local_peer_id,
        };

        tokio::spawn(async move { client.status().await });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::Status { sender }) => {
                    let _ = sender.send(Default::default());
                },
                _ => panic!("Command must match Command::Status")
            }
        }
    }

    #[tokio::test]
    async fn test_get_idle_metric() {
        let (sender, mut receiver) = mpsc::channel(1);

        let local_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let mut client = Client {
            sender,
            local_peer_id,
        };

        let mut peers: HashSet<PeerId> = HashSet::new();
        peers.insert(client.local_peer_id);
        tokio::spawn(async move { client.get_idle_peer(peers).await });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::RequestIdleMetric { peer, sender }) => {
                    assert_eq!(peer, local_peer_id);
                    let peer_metric = PeerMetrics {
                        idle_metric: 8675309f64.to_le_bytes(),
                    };
                    let _ = sender.send(Ok(peer_metric));
                },
                None => {},
                _ => panic!("Command must match Command::RequestIdleMetric")
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

        let random_artifact_id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let cloned_random_artifact_id = random_artifact_id.clone();
        tokio::spawn(async move { client.provide(&random_artifact_id).await });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::Provide { artifact_id, sender }) => {
                    assert_eq!(artifact_id, cloned_random_artifact_id);
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

        let random_artifact_id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let cloned_random_artifact_id = random_artifact_id.clone();
        tokio::spawn(async move { client.list_providers(&random_artifact_id).await });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::ListProviders { artifact_id, sender }) => {
                    assert_eq!(artifact_id, cloned_random_artifact_id);
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
        let random_artifact_id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let cloned_random_artifact_id = random_artifact_id.clone();
        tokio::spawn(async move {
            client
                .request_artifact(&other_peer_id, &random_artifact_id)
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::RequestArtifact { peer, artifact_id, sender }) => {
                    assert_eq!(peer, other_peer_id);
                    assert_eq!(artifact_id, cloned_random_artifact_id);
                    let _ = sender.send(Ok(vec![]));
                },
                _ => panic!("Command must match Command::RequestArtifact")
            }
        }
    }

    #[tokio::test]
    async fn test_request_block_update() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let other_peer_id = Keypair::generate_ed25519().public().to_peer_id();

        let block = vec![0, 1, 2, 3, 4];
        tokio::spawn(async move {
            client
                .request_block_update(&other_peer_id, 1u64, block)
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::RequestBlockUpdate { peer, block_ordinal, block, sender:_ }) => {
                    assert_eq!(peer, other_peer_id);
                    assert_eq!(block_ordinal, 1u64 );
                    assert_eq!(block, vec![0, 1, 2, 3, 4]);
                },
                _ => panic!("Command must match Command::RequestBlockUpdate")
            }
        }
    }

    #[test]
    fn test_artifact_from_str_ref() {
        let str = "abcd";

        let artifact = ArtifactHash::from(str);

        assert_eq!(artifact.hash, str);
    }

    #[test]
    fn test_artifact_from_string() {
        let str = "abcd";

        let artifact = ArtifactHash::from(str.to_string());

        assert_eq!(artifact.hash, str);
    }

    #[test]
    fn test_artifact_from_string_ref() {
        let str = String::from("abcd");

        let artifact = ArtifactHash::from(&str);

        assert_eq!(artifact.hash, str);
    }
}
