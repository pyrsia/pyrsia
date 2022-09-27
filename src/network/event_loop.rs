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

use crate::artifact_service::model::PackageType;
use crate::network::artifact_protocol::{ArtifactRequest, ArtifactResponse};
use crate::network::behaviour::{PyrsiaNetworkBehaviour, PyrsiaNetworkEvent};
use crate::network::blockchain_protocol::{BlockchainRequest, BlockchainResponse};
use crate::network::build_protocol::{BuildRequest, BuildResponse};
use crate::network::client::command::Command;
use crate::network::idle_metric_protocol::{IdleMetricRequest, IdleMetricResponse, PeerMetrics};
use crate::node_api::model::cli::Status;
use crate::util::env_util::read_var;
use libp2p::autonat::{Event as AutonatEvent, NatStatus};
use libp2p::core::PeerId;
use libp2p::futures::StreamExt;
use libp2p::identify::IdentifyEvent;
use libp2p::kad::{GetClosestPeersOk, GetProvidersOk, KademliaEvent, QueryId, QueryResult};
use libp2p::multiaddr::Protocol;
use libp2p::request_response::{
    RequestId, RequestResponseEvent, RequestResponseMessage, ResponseChannel,
};
use libp2p::swarm::SwarmEvent;
use libp2p::Swarm;
use log::{debug, error, info, trace, warn};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::error::Error;
use tokio::sync::{mpsc, oneshot};

type PendingDialMap = HashMap<PeerId, oneshot::Sender<anyhow::Result<()>>>;
type PendingListPeersMap = HashMap<QueryId, oneshot::Sender<HashSet<PeerId>>>;
type PendingStartProvidingMap = HashMap<QueryId, oneshot::Sender<()>>;
type PendingRequestArtifactMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<Vec<u8>>>>;
type PendingRequestBuildMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<String>>>;
type PendingRequestIdleMetricMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<PeerMetrics>>>;
type PendingRequestBlockchainMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<Vec<u8>>>>;

/// The `PyrsiaEventLoop` is responsible for taking care of incoming
/// events from the libp2p [`Swarm`] itself, the different network
/// behaviours that exist inside the `Swarm` and incoming commands
/// from the [`Client`].
pub struct PyrsiaEventLoop {
    swarm: Swarm<PyrsiaNetworkBehaviour>,
    command_receiver: mpsc::Receiver<Command>,
    event_sender: mpsc::Sender<PyrsiaEvent>,
    pending_dial: PendingDialMap,
    pending_list_peers: PendingListPeersMap,
    pending_start_providing: PendingStartProvidingMap,
    pending_list_providers: PendingListPeersMap,
    pending_request_artifact: PendingRequestArtifactMap,
    pending_request_build: PendingRequestBuildMap,
    pending_idle_metric_requests: PendingRequestIdleMetricMap,
    pending_blockchain_requests: PendingRequestBlockchainMap,
}

impl PyrsiaEventLoop {
    pub fn new(
        swarm: Swarm<PyrsiaNetworkBehaviour>,
        command_receiver: mpsc::Receiver<Command>,
        event_sender: mpsc::Sender<PyrsiaEvent>,
    ) -> Self {
        Self {
            swarm,
            command_receiver,
            event_sender,
            pending_dial: Default::default(),
            pending_list_peers: Default::default(),
            pending_start_providing: Default::default(),
            pending_list_providers: Default::default(),
            pending_request_artifact: Default::default(),
            pending_request_build: Default::default(),
            pending_idle_metric_requests: Default::default(),
            pending_blockchain_requests: Default::default(),
        }
    }

    /// Creates the actual event loop to begin listening for
    /// incoming events on the swarm and command channels.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::AutoNat(autonat_event)) => self.handle_autonat_event(autonat_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::Identify(identify_event)) => self.handle_identify_event(identify_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::Kademlia(kademlia_event)) => self.handle_kademlia_event(kademlia_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::RequestResponse(request_response_event)) => self.handle_request_response_event(request_response_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::BuildRequestResponse(build_request_response_event)) => self.handle_build_request_response_event(build_request_response_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::IdleMetricRequestResponse(request_response_event)) => self.handle_idle_metric_request_response_event(request_response_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::BlockchainRequestResponse(request_response_event)) => self.handle_blockchain_request_response_event(request_response_event).await,
                    swarm_event => self.handle_swarm_event(swarm_event).await,
                },
                command = self.command_receiver.recv() => match command {
                    Some(c) => {
                        self.handle_command(c).await;
                    },
                    // Command channel closed, thus shutting down the network event loop.
                    None => { warn!("Got empty command"); return },
                },
            }
        }
    }

    // Handles events from the `AutoNat` network behaviour.
    async fn handle_autonat_event(&mut self, event: AutonatEvent) {
        trace!("Handle AutonatEvent: {:?}", event);
        match event {
            AutonatEvent::InboundProbe(..) => {}
            AutonatEvent::OutboundProbe(..) => {}
            AutonatEvent::StatusChanged { old, new } => {
                info!("Autonat status changed from {:?} to {:?}", old, new);
                match new {
                    NatStatus::Public(address) => {
                        let local_peer_id = *self.swarm.local_peer_id();
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&local_peer_id, address);
                    }
                    NatStatus::Private => {
                        // todo: setup relay listen address
                    }
                    NatStatus::Unknown => {}
                }
            }
        }
    }

    // Handles events from the `Identify` network behaviour.
    async fn handle_identify_event(&mut self, event: IdentifyEvent) {
        trace!("Handle IdentifyEvent: {:?}", event);
        match event {
            IdentifyEvent::Pushed { .. } => {}
            IdentifyEvent::Received { .. } => {}
            IdentifyEvent::Sent { .. } => {}
            IdentifyEvent::Error { .. } => {}
        }
    }

    // Handles events from the `Kademlia` network behaviour.
    async fn handle_kademlia_event(&mut self, event: KademliaEvent) {
        trace!("Handle KademliaEvent: {:?}", event);
        let event_str = format!("{:#?}", event);
        match event {
            KademliaEvent::OutboundQueryCompleted {
                id,
                result: QueryResult::GetClosestPeers(Ok(GetClosestPeersOk { key: _key, peers })),
                ..
            } => {
                self.pending_list_peers
                    .remove(&id)
                    .expect("Completed query to be previously pending.")
                    .send(HashSet::from_iter(peers))
                    .unwrap_or_else(|e| {
                        error!(
                            "Handle KademliaEvent match arm: {}. Peers: {:?}",
                            event_str, e
                        );
                    });
            }
            KademliaEvent::OutboundQueryCompleted {
                id,
                result: QueryResult::StartProviding(_),
                ..
            } => {
                let sender: oneshot::Sender<()> = self
                    .pending_start_providing
                    .remove(&id)
                    .expect("Completed query to be previously pending.");

                sender.send(()).unwrap_or_else(|_e| {
                    error!("Handle KademliaEvent match arm: {}.", event_str);
                });
            }
            KademliaEvent::OutboundQueryCompleted {
                id,
                result:
                    QueryResult::GetProviders(Ok(GetProvidersOk {
                        key: _key,
                        providers,
                        ..
                    })),
                ..
            } => {
                self.pending_list_providers
                    .remove(&id)
                    .expect("Completed query to be previously pending.")
                    .send(providers)
                    .unwrap_or_else(|e| {
                        error!(
                            "Handle KademliaEvent match arm: {}. Providers: {:?}",
                            event_str, e
                        );
                    });
            }
            _ => {}
        }
    }

    // Handles events from the `RequestResponse` for artifact exchange
    // network behaviour.
    async fn handle_request_response_event(
        &mut self,
        event: RequestResponseEvent<ArtifactRequest, ArtifactResponse>,
    ) {
        trace!("Handle RequestResponseEvent: {:?}", event);
        let event_str = format!("{:#?}", event);
        match event {
            RequestResponseEvent::Message { message, .. } => match message {
                RequestResponseMessage::Request {
                    request, channel, ..
                } => {
                    self.event_sender
                        .send(PyrsiaEvent::RequestArtifact {
                            artifact_id: request.0,
                            channel,
                        })
                        .await
                        .expect("Event receiver not to be dropped.");
                }
                RequestResponseMessage::Response {
                    request_id,
                    response,
                } => {
                    self.pending_request_artifact
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Ok(response.0))
                        .unwrap_or_else(|e| {
                            error!(
                                "Handle RequestResponseEvent match arm: {}. Error: {:?}",
                                event_str, e
                            );
                        });
                }
            },
            RequestResponseEvent::InboundFailure { .. } => {}
            RequestResponseEvent::OutboundFailure {
                request_id, error, ..
            } => {
                self.pending_request_artifact
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(error.into()))
                    .unwrap_or_else(|e| {
                        error!("Handle RequestResponseEvent match arm: {}. pending_request_artifact: {:?}", event_str, e);
                    });
            }
            RequestResponseEvent::ResponseSent { .. } => {}
        }
    }

    // Handles events from the `RequestResponse` for peer metric exchange
    // network behaviour.
    async fn handle_idle_metric_request_response_event(
        &mut self,
        event: RequestResponseEvent<IdleMetricRequest, IdleMetricResponse>,
    ) {
        trace!("Handle RequestResponseEvent: {:?}", event);
        let event_str = format!("{:#?}", event);
        match event {
            RequestResponseEvent::Message { message, .. } => match message {
                RequestResponseMessage::Request { channel, .. } => {
                    self.event_sender
                        .send(PyrsiaEvent::IdleMetricRequest { channel })
                        .await
                        .expect("Event receiver not to be dropped.");
                }
                RequestResponseMessage::Response {
                    request_id,
                    response,
                } => {
                    self.pending_idle_metric_requests
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Ok(response.0))
                        .unwrap_or_else(|e| {
                            error!("Handle RequestResponseEvent match arm: {}. pending_idle_metric_requests: {:?}", event_str, e);
                        });
                }
            },
            RequestResponseEvent::InboundFailure { .. } => {}
            RequestResponseEvent::OutboundFailure {
                request_id, error, ..
            } => {
                self.pending_idle_metric_requests
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(error.into()))
                    .unwrap_or_else(|e| {
                        error!("Handle RequestResponseEvent match arm: {}. pending_idle_metric_requests: {:?}", event_str, e);
                    });
            }
            RequestResponseEvent::ResponseSent { .. } => {}
        }
    }

    // Handles events from the `RequestResponse` for build exchange
    // network behaviour.
    async fn handle_build_request_response_event(
        &mut self,
        event: RequestResponseEvent<BuildRequest, BuildResponse>,
    ) {
        trace!("Handle BuildRequestResponseEvent: {:?}", event);
        let event_str = format!("{:#?}", event);
        match event {
            RequestResponseEvent::Message { message, .. } => match message {
                RequestResponseMessage::Request {
                    request, channel, ..
                } => {
                    debug!("RequestResponseMessage::Request {:?}", request);
                    self.event_sender
                        .send(PyrsiaEvent::RequestBuild {
                            package_type: request.0,
                            package_specific_id: request.1,
                            channel,
                        })
                        .await
                        .expect("Event receiver not to be dropped.");
                }
                RequestResponseMessage::Response {
                    request_id,
                    response: _,
                } => {
                    debug!("RequestResponseMessage::Response {:?}", request_id);
                    self.pending_request_build
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Ok(request_id.to_string()))
                        .unwrap_or_else(|e| {
                            error!(
                                "Handle RequestResponseEvent match arm: {}. Error: {:?}",
                                event_str, e
                            );
                        });
                }
            },
            RequestResponseEvent::InboundFailure { .. } => {}
            RequestResponseEvent::OutboundFailure {
                request_id, error, ..
            } => {
                debug!(
                    "RequestResponseMessage::OutboundFailure {:?} with error {:?}",
                    request_id, error
                );
                self.pending_request_build
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(error.into()))
                    .unwrap_or_else(|e| {
                        error!("Handle RequestResponseEvent match arm: {}. pending_request_build: {:?}", event_str, e);
                    });
            }
            RequestResponseEvent::ResponseSent { .. } => {}
        }
    }

    // Handles events from the `RequestResponse` for blockchain update exchange network behaviour.
    async fn handle_blockchain_request_response_event(
        &mut self,
        event: RequestResponseEvent<BlockchainRequest, BlockchainResponse>,
    ) {
        trace!("Handle RequestResponseEvent: {:?}", event);
        let event_str = format!("{:#?}", event);
        match event {
            RequestResponseEvent::Message { message, .. } => match message {
                RequestResponseMessage::Request {
                    request, channel, ..
                } => {
                    self.event_sender
                        .send(PyrsiaEvent::BlockchainRequest {
                            data: request.0,
                            channel,
                        })
                        .await
                        .expect("Event receiver not to be dropped.");
                }
                RequestResponseMessage::Response {
                    request_id,
                    response,
                    ..
                } => {
                    self.pending_blockchain_requests
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Ok(response.0))
                        .unwrap_or_else(|e| {
                            error!("Handle RequestResponseEvent match arm: {}. pending_blockchain_requests: {:?}", event_str, e);
                        });
                }
            },
            RequestResponseEvent::InboundFailure { .. } => {}
            RequestResponseEvent::OutboundFailure {
                request_id, error, ..
            } => {
                self.pending_blockchain_requests
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(From::from(error)))
                    .unwrap_or_else(|e| {
                        error!("Handle RequestResponseEvent match arm: {}. pending_blockchain_requests: {:?}", event_str, e);
                    });
            }
            RequestResponseEvent::ResponseSent { .. } => {}
        }
    }

    // Handles all other events from the libp2p `Swarm`.
    async fn handle_swarm_event(&mut self, event: SwarmEvent<PyrsiaNetworkEvent, impl Error>) {
        trace!("Handle SwarmEvent: {:?}", event);
        let event_str = format!("{:#?}", event);
        match event {
            SwarmEvent::Behaviour(_) => {
                debug!("Unmatched Behaviour swarm event found: {:?}", event);
            }
            SwarmEvent::NewListenAddr { address, .. } => {
                let local_peer_id = *self.swarm.local_peer_id();
                info!(
                    "Local node is listening on {:?}",
                    address.with(Protocol::P2p(local_peer_id.into()))
                );
            }
            SwarmEvent::ConnectionEstablished {
                peer_id, endpoint, ..
            } => {
                if endpoint.is_dialer() {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        self.swarm
                            .behaviour_mut()
                            .kademlia
                            .add_address(&peer_id, endpoint.get_remote_address().to_owned());

                        sender.send(Ok(())).unwrap_or_else(|_e| {
                            error!("Handle SwarmEvent match arm: {}", event_str);
                        });
                    }
                }
            }
            SwarmEvent::ConnectionClosed { .. } => {}
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    if peer_id == *self.swarm.local_peer_id() {
                        warn!("The dialed node has the same peer ID as the current node: '{}'. Please make sure that every node has a unique peer ID.", peer_id);
                    }
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        sender.send(Err(error.into())).unwrap_or_else(|_e| {
                            error!("Handle SwarmEvent match arm: {}", event_str);
                        });
                    }
                }
            }
            SwarmEvent::BannedPeer { .. } => {}
            SwarmEvent::Dialing(peer_id) => {
                debug!(
                    "Local Peer {} is dialing Peer {}...",
                    self.swarm.local_peer_id(),
                    peer_id
                );
            }
            SwarmEvent::ExpiredListenAddr { .. } => {}
            SwarmEvent::IncomingConnection { .. } => {}
            SwarmEvent::IncomingConnectionError { error, .. } => {
                warn!("{}", error);
            }
            SwarmEvent::ListenerClosed { .. } => {}
            SwarmEvent::ListenerError { .. } => {}
        }
    }

    // Handle incoming commands that are sent by the [`Client`].
    async fn handle_command(&mut self, command: Command) {
        trace!("Handle Command: {}", command);
        let command_str = format!("{:#?}", command);
        match command {
            Command::AddProbe {
                peer_id,
                probe_addr,
                sender,
            } => {
                if let Entry::Vacant(_) = self.pending_dial.entry(peer_id) {
                    self.pending_dial.insert(peer_id, sender);
                    self.swarm
                        .behaviour_mut()
                        .auto_nat
                        .add_server(peer_id, Some(probe_addr));
                }
            }
            Command::Listen { addr, sender } => {
                match self.swarm.listen_on(addr) {
                    Ok(_) => sender.send(Ok(())),
                    Err(e) => sender.send(Err(e.into())),
                }
                .unwrap_or_else(|_e| {
                    error!("Handle Command match arm: {}.", command_str);
                });
            }
            Command::Dial {
                peer_id,
                peer_addr,
                sender,
            } => {
                if let Entry::Vacant(_) = self.pending_dial.entry(peer_id) {
                    match self
                        .swarm
                        .dial(peer_addr.with(Protocol::P2p(peer_id.into())))
                    {
                        Ok(()) => {
                            self.pending_dial.insert(peer_id, sender);
                        }
                        Err(e) => {
                            sender.send(Err(e.into())).unwrap_or_else(|_e| {
                                error!("Handle Command match arm: {}.", command_str);
                            });
                        }
                    }
                }
            }
            Command::ListPeers { sender } => {
                let peers = HashSet::from_iter(self.swarm.connected_peers().copied());
                sender.send(peers).unwrap_or_else(|_e| {
                    error!("Handle Command match arm: {}.", command_str);
                });
            }
            Command::Status { sender } => {
                let swarm = &self.swarm;
                let local_peer_id = *swarm.local_peer_id();
                let local_peer = Protocol::P2p(local_peer_id.into());

                let externalip = read_var("PYRSIA_EXTERNAL_IP", "");

                let mut addr_map = HashSet::new();

                for addr in swarm.listeners() {
                    if !externalip.is_empty() {
                        match externalip.parse() {
                            Ok(ipv4_addr) => {
                                let new_addr =
                                    addr.replace(0, |_| Some(Protocol::Ip4(ipv4_addr))).unwrap();
                                addr_map.insert(format!("{}{}", new_addr, local_peer));
                            }
                            Err(err) => {
                                // don't map external ip, skip mapping and display error
                                addr_map.insert(format!("{}{}", addr, local_peer));
                                info!("Ipv4Addr parse error of {}: {}", externalip, err);
                            }
                        }
                    } else {
                        addr_map.insert(format!("{}{}", addr, local_peer));
                    }
                }

                let peer_addrs = addr_map.into_iter().collect::<Vec<_>>();

                let status = Status {
                    peers_count: swarm.connected_peers().count(),
                    peer_id: local_peer_id.to_string(),
                    peer_addrs,
                };

                sender.send(status).unwrap();
            }
            Command::Provide {
                artifact_id,
                sender,
            } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .start_providing(artifact_id.into_bytes().into())
                    .expect("No store error.");
                self.pending_start_providing.insert(query_id, sender);
            }
            Command::ListProviders {
                artifact_id,
                sender,
            } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .get_providers(artifact_id.into_bytes().into());
                self.pending_list_providers.insert(query_id, sender);
            }
            Command::RequestBuild {
                peer,
                package_type,
                package_specific_id,
                sender,
            } => {
                debug!("Event loop :: send build request");
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .build_request_response
                    .send_request(&peer, BuildRequest(package_type, package_specific_id));
                debug!("Event loop :: build request sent with id {:?}", request_id);
                self.pending_request_build.insert(request_id, sender);
            }
            Command::RequestArtifact {
                artifact_id,
                peer,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer, ArtifactRequest(artifact_id));
                self.pending_request_artifact.insert(request_id, sender);
            }
            Command::RespondArtifact { artifact, channel } => {
                self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, ArtifactResponse(artifact))
                    .expect("Connection to peer to be still open.");
            }
            Command::RequestIdleMetric { peer, sender } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .idle_metric_request_response
                    .send_request(&peer, IdleMetricRequest());
                self.pending_idle_metric_requests.insert(request_id, sender);
            }
            Command::RespondIdleMetric { metric, channel } => {
                self.swarm
                    .behaviour_mut()
                    .idle_metric_request_response
                    .send_response(channel, IdleMetricResponse(metric))
                    .expect("Connection to peer to be still open.");
            }
            Command::RequestBlockchain { data, peer, sender } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .blockchain_request_response
                    .send_request(&peer, BlockchainRequest(data));
                self.pending_blockchain_requests.insert(request_id, sender);
            }
            Command::RespondBlockchain { data, channel } => {
                self.swarm
                    .behaviour_mut()
                    .blockchain_request_response
                    .send_response(channel, BlockchainResponse(data))
                    .expect("Connection to peer to be still open.");
            }
        }
    }
}

#[derive(Debug)]
pub enum PyrsiaEvent {
    RequestArtifact {
        artifact_id: String,
        channel: ResponseChannel<ArtifactResponse>,
    },
    RequestBuild {
        package_type: PackageType,
        package_specific_id: String,
        channel: ResponseChannel<BuildResponse>,
    },
    IdleMetricRequest {
        channel: ResponseChannel<IdleMetricResponse>,
    },
    BlockchainRequest {
        data: Vec<u8>,
        channel: ResponseChannel<BlockchainResponse>,
    },
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::network::artifact_protocol::{ArtifactExchangeCodec, ArtifactExchangeProtocol};
    use crate::network::blockchain_protocol::{
        BlockchainExchangeCodec, BlockchainExchangeProtocol,
    };
    use crate::network::build_protocol::{BuildExchangeCodec, BuildExchangeProtocol};
    use crate::network::client::Client;
    use crate::network::idle_metric_protocol::{
        IdleMetricExchangeCodec, IdleMetricExchangeProtocol,
    };
    use libp2p::core::upgrade;
    use libp2p::core::Transport;
    use libp2p::identity::Keypair;
    use libp2p::swarm::SwarmBuilder;
    use libp2p::tcp::{self, GenTcpConfig};
    use libp2p::yamux::YamuxConfig;
    use libp2p::{autonat, dns, identify, kad, noise, request_response};
    use std::iter;
    use std::time::Duration;

    fn create_test_swarm() -> (Client, PyrsiaEventLoop) {
        let id_keys = Keypair::generate_ed25519();
        let local_public_key = id_keys.public();
        let peer_id = local_public_key.to_peer_id();

        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&id_keys)
            .expect("Signing libp2p-noise static DH keypair failed.");

        let transport = tcp::TokioTcpTransport::new(GenTcpConfig::default().nodelay(true));
        let dns = dns::TokioDnsConfig::system(transport).unwrap();

        let mem_transport = dns
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(YamuxConfig::default())
            .timeout(Duration::from_secs(20))
            .boxed();

        let behaviour = PyrsiaNetworkBehaviour {
            auto_nat: autonat::Behaviour::new(
                peer_id,
                autonat::Config {
                    retry_interval: Duration::from_secs(10),
                    refresh_interval: Duration::from_secs(30),
                    boot_delay: Duration::from_secs(5),
                    throttle_server_period: Duration::ZERO,
                    ..Default::default()
                },
            ),
            identify: identify::Identify::new(identify::IdentifyConfig::new(
                "ipfs/1.0.0".to_owned(),
                id_keys.public(),
            )),
            kademlia: kad::Kademlia::new(peer_id, kad::record::store::MemoryStore::new(peer_id)),
            request_response: request_response::RequestResponse::new(
                ArtifactExchangeCodec(),
                iter::once((
                    ArtifactExchangeProtocol(),
                    request_response::ProtocolSupport::Full,
                )),
                Default::default(),
            ),
            build_request_response: request_response::RequestResponse::new(
                BuildExchangeCodec(),
                iter::once((
                    BuildExchangeProtocol(),
                    request_response::ProtocolSupport::Full,
                )),
                Default::default(),
            ),
            idle_metric_request_response: request_response::RequestResponse::new(
                IdleMetricExchangeCodec(),
                iter::once((
                    IdleMetricExchangeProtocol(),
                    request_response::ProtocolSupport::Full,
                )),
                Default::default(),
            ),
            blockchain_request_response: request_response::RequestResponse::new(
                BlockchainExchangeCodec(),
                iter::once((
                    BlockchainExchangeProtocol(),
                    request_response::ProtocolSupport::Full,
                )),
                Default::default(),
            ),
        };

        let swarm = SwarmBuilder::new(mem_transport, behaviour, local_public_key.to_peer_id())
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build();

        let (command_sender, command_receiver) = mpsc::channel(1);
        let (event_sender, _) = mpsc::channel(1);

        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: peer_id,
        };
        let event_loop = PyrsiaEventLoop::new(swarm, command_receiver, event_sender);

        (p2p_client, event_loop)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dial_address_with_listener() {
        let (mut p2p_client_1, event_loop_1) = create_test_swarm();
        let (mut p2p_client_2, event_loop_2) = create_test_swarm();

        tokio::spawn(event_loop_1.run());
        tokio::spawn(event_loop_2.run());

        p2p_client_1
            .listen(&"/ip4/127.0.0.1/tcp/44120".parse().unwrap())
            .await
            .unwrap();
        p2p_client_2
            .listen(&"/ip4/127.0.0.1/tcp/44121".parse().unwrap())
            .await
            .unwrap();

        let result = p2p_client_2
            .dial(
                &p2p_client_1.local_peer_id,
                &"/ip4/127.0.0.1/tcp/44120".parse().unwrap(),
            )
            .await;
        assert!(result.is_ok());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dial_address_without_listener() {
        let (mut p2p_client_1, event_loop_1) = create_test_swarm();
        let (mut p2p_client_2, event_loop_2) = create_test_swarm();

        tokio::spawn(event_loop_1.run());
        tokio::spawn(event_loop_2.run());

        p2p_client_1
            .listen(&"/ip4/127.0.0.1/tcp/44125".parse().unwrap())
            .await
            .unwrap();
        p2p_client_2
            .listen(&"/ip4/127.0.0.1/tcp/44126".parse().unwrap())
            .await
            .unwrap();

        let result = p2p_client_2
            .dial(
                &p2p_client_1.local_peer_id,
                &"/ip4/127.0.0.1/tcp/44127".parse().unwrap(),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dial_with_invalid_peer_id() {
        let (mut p2p_client_1, event_loop_1) = create_test_swarm();
        let (p2p_client_2, _) = create_test_swarm();

        tokio::spawn(event_loop_1.run());

        p2p_client_1
            .listen(&"/ip4/127.0.0.1/tcp/44132".parse().unwrap())
            .await
            .unwrap();

        let result = p2p_client_1
            .dial(
                &p2p_client_2.local_peer_id,
                &"/ip4/127.0.0.1/tcp/44133".parse().unwrap(),
            )
            .await;
        assert!(result.is_err());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_request_build_loop() {
        let (mut p2p_client_1, event_loop_1) = create_test_swarm();
        let (mut p2p_client_2, event_loop_2) = create_test_swarm();

        tokio::spawn(event_loop_1.run());
        tokio::spawn(event_loop_2.run());

        p2p_client_1
            .listen(&"/ip4/127.0.0.1/tcp/44140".parse().unwrap())
            .await
            .unwrap();
        p2p_client_2
            .listen(&"/ip4/127.0.0.1/tcp/44141".parse().unwrap())
            .await
            .unwrap();

        let result_dial = p2p_client_1
            .dial(
                &p2p_client_2.local_peer_id,
                &"/ip4/127.0.0.1/tcp/44141".parse().unwrap(),
            )
            .await;
        assert!(result_dial.is_ok());

        let package_type = PackageType::Docker;
        let package_specific_id = "package_specific_id";

        let result = p2p_client_1
            .request_build(
                &p2p_client_2.local_peer_id,
                package_type,
                package_specific_id.to_string(),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "1");
    }
}
