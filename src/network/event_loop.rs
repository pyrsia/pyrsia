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

use crate::network::artifact_protocol::{ArtifactRequest, ArtifactResponse};
use crate::network::behaviour::{PyrsiaNetworkBehaviour, PyrsiaNetworkEvent};
use crate::network::client::command::Command;
use crate::network::idle_metric_protocol::{IdleMetricRequest, IdleMetricResponse, PeerMetrics};
use libp2p::core::PeerId;
use libp2p::futures::StreamExt;
use libp2p::kad::{GetClosestPeersOk, GetProvidersOk, KademliaEvent, QueryId, QueryResult};
use libp2p::multiaddr::Protocol;
use libp2p::request_response::{
    RequestId, RequestResponseEvent, RequestResponseMessage, ResponseChannel,
};
use libp2p::swarm::SwarmEvent;
use libp2p::Swarm;
use log::{debug, info, trace, warn};
use std::collections::{hash_map::Entry, HashMap, HashSet};
use std::error::Error;
use tokio::sync::{mpsc, oneshot};

type PendingDialMap = HashMap<PeerId, oneshot::Sender<anyhow::Result<()>>>;
type PendingListPeersMap = HashMap<QueryId, oneshot::Sender<HashSet<PeerId>>>;
type PendingStartProvidingMap = HashMap<QueryId, oneshot::Sender<()>>;
type PendingRequestArtifactMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<Vec<u8>>>>;
type PendingRequestIdleMetricMap = HashMap<RequestId, oneshot::Sender<anyhow::Result<PeerMetrics>>>;

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
    pending_idle_metric_requests: PendingRequestIdleMetricMap,
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
            pending_idle_metric_requests: Default::default(),
        }
    }

    /// Creates the actual event loop to begin listening for
    /// incoming events on the swarm and command channels.
    pub async fn run(mut self) {
        loop {
            tokio::select! {
                event = self.swarm.select_next_some() => match event {
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::Kademlia(kademlia_event)) => self.handle_kademlia_event(kademlia_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::RequestResponse(request_response_event)) => self.handle_request_response_event(request_response_event).await,
                    SwarmEvent::Behaviour(PyrsiaNetworkEvent::IdleMetricRequestResponse(request_response_event)) => self.handle_idle_metric_request_response_event(request_response_event).await,
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

    // Handles events from the `Kademlia` network behaviour.
    async fn handle_kademlia_event(&mut self, event: KademliaEvent) {
        trace!("Handle KademliaEvent: {:?}", event);
        match event {
            KademliaEvent::OutboundQueryCompleted {
                id,
                result: QueryResult::GetClosestPeers(Ok(GetClosestPeersOk { key: _key, peers })),
                ..
            } => {
                let _ = self
                    .pending_list_peers
                    .remove(&id)
                    .expect("Completed query to be previously pending.")
                    .send(HashSet::from_iter(peers));
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
                let _ = sender.send(());
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
                let _ = self
                    .pending_list_providers
                    .remove(&id)
                    .expect("Completed query to be previously pending.")
                    .send(providers);
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
                    let _ = self
                        .pending_request_artifact
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Ok(response.0));
                }
            },
            RequestResponseEvent::InboundFailure { .. } => {}
            RequestResponseEvent::OutboundFailure {
                request_id, error, ..
            } => {
                let _ = self
                    .pending_request_artifact
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(error.into()));
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
                    let _ = self
                        .pending_idle_metric_requests
                        .remove(&request_id)
                        .expect("Request to still be pending.")
                        .send(Ok(response.0));
                }
            },
            RequestResponseEvent::InboundFailure { .. } => {}
            RequestResponseEvent::OutboundFailure {
                request_id, error, ..
            } => {
                let _ = self
                    .pending_idle_metric_requests
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(error.into()));
            }
            RequestResponseEvent::ResponseSent { .. } => {}
        }
    }
    // Handles all other events from the libp2p `Swarm`.
    async fn handle_swarm_event(&mut self, event: SwarmEvent<PyrsiaNetworkEvent, impl Error>) {
        trace!("Handle SwarmEvent: {:?}", event);
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
                        let _ = sender.send(Ok(()));
                    }
                }
            }
            SwarmEvent::ConnectionClosed { .. } => {}
            SwarmEvent::OutgoingConnectionError { peer_id, error, .. } => {
                if let Some(peer_id) = peer_id {
                    if let Some(sender) = self.pending_dial.remove(&peer_id) {
                        let _ = sender.send(Err(error.into()));
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
            SwarmEvent::IncomingConnectionError { .. } => {}
            SwarmEvent::ListenerClosed { .. } => {}
            SwarmEvent::ListenerError { .. } => {}
        }
    }

    // Handle incoming commands that are sent by the [`Client`].
    async fn handle_command(&mut self, command: Command) {
        trace!("Handle Command: {}", command);
        match command {
            Command::Listen { addr, sender } => {
                let _ = match self.swarm.listen_on(addr) {
                    Ok(_) => sender.send(Ok(())),
                    Err(e) => sender.send(Err(e.into())),
                };
            }
            Command::Dial {
                peer_id,
                peer_addr,
                sender,
            } => {
                if let Entry::Vacant(_) = self.pending_dial.entry(peer_id) {
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, peer_addr.clone());

                    match self
                        .swarm
                        .dial(peer_addr.with(Protocol::P2p(peer_id.into())))
                    {
                        Ok(()) => {
                            self.pending_dial.insert(peer_id, sender);
                        }
                        Err(e) => {
                            let _ = sender.send(Err(e.into()));
                        }
                    }
                }
            }
            Command::ListPeers { peer_id, sender } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .get_closest_peers(peer_id);
                self.pending_list_peers.insert(query_id, sender);
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
        }
    }
}

#[derive(Debug)]
pub enum PyrsiaEvent {
    RequestArtifact {
        artifact_id: String,
        channel: ResponseChannel<ArtifactResponse>,
    },
    IdleMetricRequest {
        channel: ResponseChannel<IdleMetricResponse>,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::artifact_protocol::{ArtifactExchangeCodec, ArtifactExchangeProtocol};
    use crate::network::client::Client;
    use crate::network::idle_metric_protocol::{
        IdleMetricExchangeCodec, IdleMetricExchangeProtocol,
    };
    use libp2p::core::upgrade;
    use libp2p::core::Transport;
    use libp2p::dns;
    use libp2p::identity::Keypair;
    use libp2p::kad;
    use libp2p::noise;
    use libp2p::request_response;
    use libp2p::swarm::SwarmBuilder;
    use libp2p::tcp;
    use libp2p::yamux::YamuxConfig;
    use std::iter;

    fn create_test_swarm() -> (Client, PyrsiaEventLoop) {
        let id_keys = Keypair::generate_ed25519();
        let local_public_key = id_keys.public();
        let peer_id = local_public_key.to_peer_id();

        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&id_keys)
            .expect("Signing libp2p-noise static DH keypair failed.");

        let tcp = tcp::TokioTcpConfig::new().nodelay(true);
        let dns = dns::TokioDnsConfig::system(tcp).unwrap();

        let mem_transport = dns
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(YamuxConfig::default())
            .timeout(std::time::Duration::from_secs(20))
            .boxed();

        let behaviour = PyrsiaNetworkBehaviour {
            kademlia: kad::Kademlia::new(peer_id, kad::record::store::MemoryStore::new(peer_id)),
            request_response: request_response::RequestResponse::new(
                ArtifactExchangeCodec(),
                iter::once((
                    ArtifactExchangeProtocol(),
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
        p2p_client_1
            .listen(&"/ip4/127.0.0.1/tcp/44121".parse().unwrap())
            .await
            .unwrap();

        let result = p2p_client_2
            .dial(
                &p2p_client_1.local_peer_id,
                &"/ip4/127.0.0.1/tcp/44121".parse().unwrap(),
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
        p2p_client_1
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
}
