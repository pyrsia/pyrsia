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
use crate::network::build_status_protocol::{BuildStatusRequest, BuildStatusResponse};
use crate::network::client::command::Command;
use crate::network::idle_metric_protocol::{IdleMetricRequest, IdleMetricResponse, PeerMetrics};
use crate::node_api::model::request::Status;
use crate::util::env_util::read_var;
use libp2p::autonat::{Event as AutonatEvent, NatStatus};
use libp2p::core::PeerId;
use libp2p::futures::StreamExt;
use libp2p::gossipsub;
use libp2p::identify;
use libp2p::kad::{BootstrapOk, GetProvidersOk, KademliaEvent, QueryId, QueryResult};
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

type PendingBootstrapMap = HashMap<QueryId, oneshot::Sender<anyhow::Result<()>>>;
type PendingDialMap = HashMap<PeerId, oneshot::Sender<anyhow::Result<()>>>;
type PendingListProvidersMap = HashMap<QueryId, PendingListProviders>;
type PendingStartProvidingMap = HashMap<QueryId, oneshot::Sender<()>>;
type PendingRequestArtifactMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<Vec<u8>>>>;
type PendingRequestBuildMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<String>>>;
type PendingRequestIdleMetricMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<PeerMetrics>>>;
type PendingRequestBlockchainMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<Vec<u8>>>>;
type PendingBuildStatusMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<String>>>;

struct PendingListProviders {
    sender: oneshot::Sender<HashSet<PeerId>>,
    providers: HashSet<PeerId>,
}

impl PendingListProviders {
    fn new(sender: oneshot::Sender<HashSet<PeerId>>) -> Self {
        Self {
            sender,
            providers: Default::default(),
        }
    }
}

/// The `PyrsiaEventLoop` is responsible for taking care of incoming
/// events from the libp2p [`Swarm`] itself, the different network
/// behaviours that exist inside the `Swarm` and incoming commands
/// from the [`Client`].
pub struct PyrsiaEventLoop {
    swarm: Swarm<PyrsiaNetworkBehaviour>,
    command_receiver: mpsc::Receiver<Command>,
    event_sender: mpsc::Sender<PyrsiaEvent>,
    bootstrapped: bool,
    pending_bootstrap: PendingBootstrapMap,
    pending_dial: PendingDialMap,
    pending_start_providing: PendingStartProvidingMap,
    pending_list_providers: PendingListProvidersMap,
    pending_request_artifact: PendingRequestArtifactMap,
    pending_request_build: PendingRequestBuildMap,
    pending_idle_metric_requests: PendingRequestIdleMetricMap,
    pending_blockchain_requests: PendingRequestBlockchainMap,
    pending_build_status_requests: PendingBuildStatusMap,
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
            bootstrapped: false,
            pending_bootstrap: Default::default(),
            pending_dial: Default::default(),
            pending_start_providing: Default::default(),
            pending_list_providers: Default::default(),
            pending_request_artifact: Default::default(),
            pending_request_build: Default::default(),
            pending_idle_metric_requests: Default::default(),
            pending_blockchain_requests: Default::default(),
            pending_build_status_requests: Default::default(),
        }
    }

    /// Creates the actual event loop to begin listening for
    /// incoming events on the swarm and command channels.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::AutoNat(autonat_event)) => self.handle_autonat_event(autonat_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::Gossipsub(gossipsub_event)) => self.handle_gossipsub_event(gossipsub_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::Identify(identify_event)) => self.handle_identify_event(*identify_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::Kademlia(kademlia_event)) => self.handle_kademlia_event(*kademlia_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::RequestResponse(request_response_event)) => self.handle_request_response_event(request_response_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::BuildRequestResponse(build_request_response_event)) => self.handle_build_request_response_event(build_request_response_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::IdleMetricRequestResponse(request_response_event)) => self.handle_idle_metric_request_response_event(request_response_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::BlockchainRequestResponse(request_response_event)) => self.handle_blockchain_request_response_event(request_response_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::BuildStatusRequestResponse(build_status_request_response_event)) => self.handle_build_status_request_response_event(build_status_request_response_event).await,
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

    // Handles events from the `GossipSub` network behaviour.
    async fn handle_gossipsub_event(&mut self, event: gossipsub::GossipsubEvent) {
        trace!("Handle GossipsubEvent: {:?}", event);
        if let gossipsub::GossipsubEvent::Message { message, .. } = event {
            self.event_sender
                .send(PyrsiaEvent::BlockchainRequest {
                    data: message.data,
                    channel: None,
                })
                .await
                .expect("Event receiver not to be dropped.");
        }
    }

    // Handles events from the `Identify` network behaviour.
    async fn handle_identify_event(&mut self, event: identify::Event) {
        trace!("Handle IdentifyEvent: {:?}", event);
        match event {
            identify::Event::Pushed { .. } => {}
            identify::Event::Received { .. } => {}
            identify::Event::Sent { .. } => {}
            identify::Event::Error { .. } => {}
        }
    }

    // Handles events from the `Kademlia` network behaviour.
    async fn handle_kademlia_event(&mut self, event: KademliaEvent) {
        trace!("Handle KademliaEvent: {:?}", event);
        let event_str = format!("{:#?}", event);
        match event {
            KademliaEvent::OutboundQueryProgressed {
                id,
                result: QueryResult::StartProviding(_),
                ..
            } => {
                let sender: oneshot::Sender<()> = self
                    .pending_start_providing
                    .remove(&id)
                    .expect("Completed query to be previously pending.");

                sender.send(()).unwrap_or_else(|e| {
                    error!(
                        "Handle KademliaEvent match arm: {}. Error: {:?}",
                        event_str, e
                    );
                });
            }
            KademliaEvent::OutboundQueryProgressed {
                id,
                result:
                    QueryResult::GetProviders(Ok(GetProvidersOk::FoundProviders { providers, .. })),
                ..
            } => {
                self.pending_list_providers
                    .get_mut(&id)
                    .expect("Completed query to be previously pending.")
                    .providers
                    .extend(providers);
            }
            KademliaEvent::OutboundQueryProgressed {
                id,
                result:
                    QueryResult::GetProviders(Ok(GetProvidersOk::FinishedWithNoAdditionalRecord {
                        ..
                    })),
                ..
            } => {
                let pending_list_provider = self
                    .pending_list_providers
                    .remove(&id)
                    .expect("Completed query to be previously pending.");

                pending_list_provider
                    .sender
                    .send(pending_list_provider.providers)
                    .unwrap_or_else(|e| {
                        error!(
                            "Handle KademliaEvent match arm: {}. Error: {:?}",
                            event_str, e
                        );
                    });
            }
            KademliaEvent::OutboundQueryProgressed {
                id,
                result: QueryResult::Bootstrap(Ok(BootstrapOk { num_remaining, .. })),
                ..
            } => {
                if num_remaining == 0 {
                    self.pending_bootstrap
                        .remove(&id)
                        .expect("Completed query to be previously pending.")
                        .send(Ok(()))
                        .unwrap_or_else(|e| {
                            error!(
                                "Handle KademliaEvent match arm: {}. Error: {:?}",
                                event_str, e
                            );
                        });
                }
            }
            KademliaEvent::OutboundQueryProgressed {
                id,
                result: QueryResult::Bootstrap(Err(e)),
                ..
            } => {
                self.pending_bootstrap
                    .remove(&id)
                    .expect("Completed query to be previously pending.")
                    .send(Err(e.into()))
                    .unwrap_or_else(|e| {
                        error!(
                            "Handle KademliaEvent match arm: {}. Error: {:?}",
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
                    response,
                } => {
                    debug!("RequestResponseMessage::Response {:?}", request_id);
                    self.pending_request_build
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

    async fn handle_build_status_request_response_event(
        &mut self,
        event: RequestResponseEvent<BuildStatusRequest, BuildStatusResponse>,
    ) {
        trace!("Handle BuildStatusRequestResponseEvent:");
        let event_str = format!("{:#?}", event);
        match event {
            RequestResponseEvent::Message { message, .. } => match message {
                RequestResponseMessage::Request {
                    request, channel, ..
                } => {
                    debug!("RequestResponseMessage::Request {:?}", request);
                    self.event_sender
                        .send(PyrsiaEvent::RequestBuildStatus {
                            build_id: request.0,
                            channel,
                        })
                        .await
                        .expect("Event receiver not to be dropped.");
                }
                RequestResponseMessage::Response {
                    request_id,
                    response,
                } => {
                    debug!("RequestResponseMessage::Response {:?}", request_id);
                    self.pending_build_status_requests
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
                debug!(
                    "RequestResponseMessage::OutboundFailure {:?} with error {:?}",
                    request_id, error
                );
                self.pending_build_status_requests
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
                            channel: Some(channel),
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
            Command::BootstrapDht { sender } => {
                if !self.bootstrapped {
                    match self.swarm.behaviour_mut().kademlia.bootstrap() {
                        Ok(query_id) => {
                            self.bootstrapped = true;
                            self.pending_bootstrap.insert(query_id, sender);
                        }
                        Err(e) => sender.send(Err(e.into())).unwrap_or_else(|_e| {
                            error!("Handle Command match arm: {}.", command_str);
                        }),
                    }
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
                self.pending_list_providers
                    .insert(query_id, PendingListProviders::new(sender));
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
            Command::RespondBuild { build_id, channel } => {
                self.swarm
                    .behaviour_mut()
                    .build_request_response
                    .send_response(channel, BuildResponse(build_id))
                    .expect("Connection to peer to be still open.");
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
            Command::BroadcastBlock {
                topic,
                block,
                sender,
            } => {
                sender
                    .send(
                        self.swarm
                            .behaviour_mut()
                            .gossipsub
                            .publish(topic, block)
                            .map(|_| ())
                            .map_err(|e| e.into()),
                    )
                    .unwrap_or_else(|_e| {
                        error!("Handle Command match arm: {}.", command_str);
                    });
            }
            Command::RequestBuildStatus {
                peer,
                build_id,
                sender,
            } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .build_status_request_response
                    .send_request(&peer, BuildStatusRequest(build_id));

                self.pending_build_status_requests
                    .insert(request_id, sender);
            }
            Command::RespondBuildStatus { status, channel } => {
                self.swarm
                    .behaviour_mut()
                    .build_status_request_response
                    .send_response(channel, BuildStatusResponse(status))
                    .expect("Connection to peer to be still open (Build status).");
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
        channel: Option<ResponseChannel<BlockchainResponse>>,
    },
    RequestBuildStatus {
        build_id: String,
        channel: ResponseChannel<BuildStatusResponse>,
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
    use crate::network::build_status_protocol::{
        BuildStatusExchangeCodec, BuildStatusExchangeProtocol,
    };
    use crate::network::client::Client;
    use crate::network::event_loop::PyrsiaEvent;
    use crate::network::idle_metric_protocol::{
        IdleMetricExchangeCodec, IdleMetricExchangeProtocol,
    };
    use libp2p::core::upgrade;
    use libp2p::core::Transport;
    use libp2p::dns::TokioDnsConfig;
    use libp2p::gossipsub::IdentTopic;
    use libp2p::identity::Keypair;
    use libp2p::swarm::SwarmBuilder;
    use libp2p::yamux::YamuxConfig;
    use libp2p::{autonat, identify, kad, noise, request_response, tcp};
    use std::iter;
    use std::time::Duration;
    use tokio_stream::wrappers::ReceiverStream;

    fn create_test_swarm() -> (Client, PyrsiaEventLoop, ReceiverStream<PyrsiaEvent>) {
        use libp2p::gossipsub::MessageId;
        use libp2p::gossipsub::{
            Gossipsub, GossipsubMessage, IdentTopic as Topic, MessageAuthenticity, ValidationMode,
        };
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let id_keys = Keypair::generate_ed25519();
        let local_public_key = id_keys.public();
        let peer_id = local_public_key.to_peer_id();

        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&id_keys)
            .expect("Signing libp2p-noise static DH keypair failed.");

        let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
        let dns = TokioDnsConfig::system(transport).unwrap();

        let mem_transport = dns
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(YamuxConfig::default())
            .timeout(Duration::from_secs(20))
            .boxed();

        // To content-address message, we can take the hash of message and use it as an ID.
        let message_id_fn = |message: &GossipsubMessage| {
            let mut s = DefaultHasher::new();
            message.data.hash(&mut s);
            MessageId::from(s.finish().to_string())
        };

        let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
            .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
            .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
            .message_id_fn(message_id_fn) // content-address messages. No two messages of the same content will be propagated.
            .support_floodsub()
            .build()
            .expect("Valid config");
        let mut gossip_sub = Gossipsub::new(
            MessageAuthenticity::Signed(id_keys.clone()),
            gossipsub_config,
        )
        .expect("Correct configuration");
        let pyrsia_topic: Topic = Topic::new("pyrsia-blockchain-topic");
        gossip_sub
            .subscribe(&pyrsia_topic)
            .expect("Could not connect to pyrsia blockchain topic");

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
            gossipsub: gossip_sub,
            identify: identify::Behaviour::new(identify::Config::new(
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
            build_status_request_response: request_response::RequestResponse::new(
                BuildStatusExchangeCodec(),
                iter::once((
                    BuildStatusExchangeProtocol(),
                    request_response::ProtocolSupport::Full,
                )),
                Default::default(),
            ),
        };

        let swarm = SwarmBuilder::with_tokio_executor(
            mem_transport,
            behaviour,
            local_public_key.to_peer_id(),
        )
        .build();

        let (command_sender, command_receiver) = mpsc::channel(1);
        let (event_sender, event_receiver) = mpsc::channel(1);

        let p2p_client = Client::new(command_sender, peer_id, IdentTopic::new("pyrsia-topic"));
        let event_loop = PyrsiaEventLoop::new(swarm, command_receiver, event_sender);

        (p2p_client, event_loop, ReceiverStream::new(event_receiver))
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_dial_address_with_listener() {
        let (mut p2p_client_1, event_loop_1, _) = create_test_swarm();
        let (mut p2p_client_2, event_loop_2, _) = create_test_swarm();

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
        let (mut p2p_client_1, event_loop_1, _) = create_test_swarm();
        let (mut p2p_client_2, event_loop_2, _) = create_test_swarm();

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
        let (mut p2p_client_1, event_loop_1, _) = create_test_swarm();
        let (p2p_client_2, _, _) = create_test_swarm();

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
        let (mut p2p_client_1, event_loop_1, _) = create_test_swarm();
        let (mut p2p_client_2, event_loop_2, mut event_receiver_2) = create_test_swarm();

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

        let expected_build_id = uuid::Uuid::new_v4();
        let p2p_client_2_peer_id = p2p_client_2.local_peer_id;
        tokio::spawn(async move {
            loop {
                if let Some(PyrsiaEvent::RequestBuild { channel, .. }) =
                    event_receiver_2.next().await
                {
                    p2p_client_2
                        .clone()
                        .respond_build(&expected_build_id.to_string(), channel)
                        .await
                        .expect("Response to have been written");
                }
            }
        });

        let result = p2p_client_1
            .request_build(
                &p2p_client_2_peer_id,
                package_type,
                package_specific_id.to_string(),
            )
            .await;
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), expected_build_id.to_string());
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_list_providers_with_interconnected_peer() {
        let (mut p2p_client_1, event_loop_1, _) = create_test_swarm();
        let (mut p2p_client_2, event_loop_2, _) = create_test_swarm();
        let (mut p2p_client_3, event_loop_3, _) = create_test_swarm();

        tokio::spawn(event_loop_1.run());
        tokio::spawn(event_loop_2.run());
        tokio::spawn(event_loop_3.run());

        let artifact_id = "artifact_id";

        p2p_client_1
            .listen(&"/ip4/127.0.0.1/tcp/44150".parse().unwrap())
            .await
            .unwrap();
        p2p_client_2
            .listen(&"/ip4/127.0.0.1/tcp/44151".parse().unwrap())
            .await
            .unwrap();
        p2p_client_3
            .listen(&"/ip4/127.0.0.1/tcp/44152".parse().unwrap())
            .await
            .unwrap();

        let result_peer_2_dial_peer_1 = p2p_client_2
            .dial(
                &p2p_client_1.local_peer_id,
                &"/ip4/127.0.0.1/tcp/44150".parse().unwrap(),
            )
            .await;
        assert!(result_peer_2_dial_peer_1.is_ok());

        let result_peer_3_dial_peer_2 = p2p_client_3
            .dial(
                &p2p_client_2.local_peer_id,
                &"/ip4/127.0.0.1/tcp/44151".parse().unwrap(),
            )
            .await;
        assert!(result_peer_3_dial_peer_2.is_ok());

        let result_provide = p2p_client_1.provide(artifact_id).await;
        assert!(result_provide.is_ok());

        let result_list_providers = p2p_client_3.list_providers(artifact_id).await;
        assert!(result_list_providers.is_ok());

        let mut expected_providers = HashSet::new();
        expected_providers.insert(p2p_client_1.local_peer_id);
        assert_eq!(expected_providers, result_list_providers.unwrap());
    }
}
