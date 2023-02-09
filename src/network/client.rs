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

use crate::artifact_service::model::PackageType;
use crate::network::artifact_protocol::ArtifactResponse;
use crate::network::blockchain_protocol::BlockchainResponse;
use crate::network::build_protocol::BuildResponse;
use crate::network::build_status_protocol::BuildStatusResponse;
use crate::network::client::command::Command;
use crate::network::idle_metric_protocol::{IdleMetricResponse, PeerMetrics};
use crate::node_api::model::request::Status;
use libp2p::core::{Multiaddr, PeerId};
use libp2p::gossipsub;
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
#[derive(Clone, Debug)]
pub struct Client {
    pub sender: mpsc::Sender<Command>,
    pub local_peer_id: PeerId,
    pyrsia_topic: gossipsub::IdentTopic,
}

impl Client {
    pub fn new(
        sender: mpsc::Sender<Command>,
        local_peer_id: PeerId,
        pyrsia_topic: gossipsub::IdentTopic,
    ) -> Self {
        Self {
            sender,
            local_peer_id,
            pyrsia_topic,
        }
    }

    /// Add a probe address for AutoNAT discovery. When adding the probe
    /// was handled successfully, the kademlia DHT will be bootstrapped.
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
        receiver.await??;

        let (bootstrap_sender, bootstrap_receiver) = oneshot::channel();
        self.sender
            .send(Command::BootstrapDht {
                sender: bootstrap_sender,
            })
            .await?;
        bootstrap_receiver.await?
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

    /// Dial a peer with the specified address. When dialing the probe
    /// was successful, the kademlia DHT will be bootstrapped.
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
        receiver.await??;

        let (bootstrap_sender, bootstrap_receiver) = oneshot::channel();
        self.sender
            .send(Command::BootstrapDht {
                sender: bootstrap_sender,
            })
            .await?;
        bootstrap_receiver.await?
    }

    /// List the peers that this node is connected to.
    pub async fn list_peers(&mut self) -> anyhow::Result<HashSet<PeerId>> {
        let (sender, receiver) = oneshot::channel();
        self.sender.send(Command::ListPeers { sender }).await?;
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

    /// Request a build to a peer with the specified address.
    pub async fn request_build(
        &mut self,
        peer_id: &PeerId,
        package_type: PackageType,
        package_specific_id: String,
    ) -> anyhow::Result<String> {
        debug!(
            "p2p::Client::request_build {:?}: {:?}: {:?}",
            peer_id, package_type, package_specific_id
        );

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestBuild {
                peer: *peer_id,
                package_type: package_type.to_owned(),
                package_specific_id: package_specific_id.to_owned(),
                sender,
            })
            .await?;

        receiver.await?
    }

    /// Put the build id as a response to an incoming build request.
    pub async fn respond_build(
        &mut self,
        build_id: &str,
        channel: ResponseChannel<BuildResponse>,
    ) -> anyhow::Result<()> {
        debug!("p2p::Client::respond_build build_id={}", build_id);

        self.sender
            .send(Command::RespondBuild {
                build_id: build_id.to_owned(),
                channel,
            })
            .await?;

        Ok(())
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

    pub async fn request_blockchain(
        &mut self,
        peer: &PeerId,
        data: Vec<u8>,
    ) -> anyhow::Result<Vec<u8>> {
        debug!("p2p::Client::request_blockchain from peer {:?}", peer);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestBlockchain {
                data,
                peer: *peer,
                sender,
            })
            .await?;
        receiver.await?
    }

    pub async fn respond_blockchain(
        &mut self,
        data: Vec<u8>,
        channel: ResponseChannel<BlockchainResponse>,
    ) -> anyhow::Result<()> {
        debug!("p2p::Client::respond_blockchain sent");

        self.sender
            .send(Command::RespondBlockchain { data, channel })
            .await?;

        Ok(())
    }

    pub async fn broadcast_block(&mut self, block: Vec<u8>) -> anyhow::Result<()> {
        debug!("p2p::Client::broadcast_block sent");

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::BroadcastBlock {
                topic: self.pyrsia_topic.clone(),
                block,
                sender,
            })
            .await?;
        receiver.await?
    }

    pub async fn request_build_status(
        &mut self,
        peer_id: &PeerId,
        build_id: String,
    ) -> anyhow::Result<String> {
        debug!(
            "p2p::Client::request_build_status peer_id {:?}, build_id: {:?}",
            peer_id, build_id
        );

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestBuildStatus {
                peer: *peer_id,
                build_id,
                sender,
            })
            .await?;

        receiver.await?
    }

    pub async fn respond_build_status(
        &mut self,
        status: &str,
        channel: ResponseChannel<BuildStatusResponse>,
    ) -> anyhow::Result<()> {
        debug!("p2p::Client::respond_build_status status={}", status);

        self.sender
            .send(Command::RespondBuildStatus {
                status: String::from(status),
                channel,
            })
            .await?;

        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use libp2p::gossipsub::IdentTopic;
    use libp2p::identity::{self, Keypair};
    use pyrsia_blockchain_network::crypto::hash_algorithm::HashDigest;
    use pyrsia_blockchain_network::structures::block::Block;
    use rand::distributions::Alphanumeric;
    use rand::{thread_rng, Rng};

    #[tokio::test]
    async fn test_listen() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
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
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
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
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
        };

        tokio::spawn(async move { client.list_peers().await });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::ListPeers { sender }) => {
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
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
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
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
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
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
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
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
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
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
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
    async fn test_request_docker_build() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
        };

        let other_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        let docker_package_type = PackageType::Docker;
        let random_package_specific_id: String = thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        let cloned_random_package_specific_id = random_package_specific_id.clone();

        tokio::spawn(async move {
            client
                .request_build(
                    &other_peer_id,
                    docker_package_type,
                    random_package_specific_id,
                )
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::RequestBuild { peer, package_type, package_specific_id, sender }) => {
                    assert_eq!(peer, other_peer_id);
                    assert_eq!(package_type, docker_package_type);
                    assert_eq!(package_specific_id, cloned_random_package_specific_id);
                    let _ = sender.send(Ok(String::from("ok")));
                },
                _ => panic!("Command must match Command::RequestBuild")
            }
        }
    }

    #[tokio::test]
    async fn test_request_blockchain() {
        let (sender, mut receiver) = mpsc::channel(1);
        let local_key = identity::ed25519::Keypair::generate();

        let mut client = Client {
            sender,
            local_peer_id: identity::PublicKey::Ed25519(local_key.public()).to_peer_id(),
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
        };

        let other_peer_id = Keypair::generate_ed25519().public().to_peer_id();

        let block = Block::new(HashDigest::new(b""), 0, vec![], &local_key);

        let mut buf: Vec<u8> = vec![1u8];
        buf.append(&mut bincode::serialize(&(1_u128)).unwrap());
        buf.append(&mut bincode::serialize(&block).unwrap());

        tokio::spawn(async move { client.request_blockchain(&other_peer_id, buf.clone()).await });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::RequestBlockchain { peer, data, sender:_ }) => {
                    assert_eq!(peer, other_peer_id);
                    assert_eq!(1u8, data[0]);
                },
                _ => panic!("Command must match Command::RequestBlockchain")
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

    #[tokio::test]
    async fn test_request_build_status() {
        let (sender, mut receiver) = mpsc::channel(1);

        let mut client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
            pyrsia_topic: IdentTopic::new("pyrsia-blockchain-topic"),
        };

        let other_peer_id = Keypair::generate_ed25519().public().to_peer_id();
        const BUILD_ID: &str = "b024a136-9021-42a1-b8de-c665c94470f4";

        tokio::spawn(async move {
            client
                .request_build_status(&other_peer_id, BUILD_ID.to_string())
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(Command::RequestBuildStatus{ peer, build_id, sender }) => {
                    assert_eq!(peer, other_peer_id);
                    assert_eq!(build_id, BUILD_ID);
                    let _ = sender.send(Ok(String::from("ok")));
                },
                _ => panic!("Command must match Command::RequestBuildStatus")
            }
        }
    }
}
