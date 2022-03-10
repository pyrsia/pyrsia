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

use async_trait::async_trait;
use futures::channel::{mpsc, oneshot};
use futures::prelude::*;
use libp2p::core::upgrade::{read_length_prefixed, write_length_prefixed, ProtocolName};
use libp2p::core::{Multiaddr, PeerId};
use libp2p::identify::{Identify, IdentifyConfig, IdentifyEvent};
use libp2p::identity;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{
    GetClosestPeersOk, GetProvidersOk, Kademlia, KademliaEvent, QueryId, QueryResult,
};
use libp2p::multiaddr::Protocol;
use libp2p::request_response::{
    ProtocolSupport, RequestId, RequestResponse, RequestResponseCodec, RequestResponseEvent,
    RequestResponseMessage, ResponseChannel,
};
use libp2p::swarm::{SwarmBuilder, SwarmEvent};
use libp2p::{NetworkBehaviour, Swarm};
use log::{debug, info, trace, warn};
use std::collections::hash_map::Entry::Vacant;
use std::collections::{HashMap, HashSet};
use std::error::Error;
use std::fmt::{Display, Formatter};
use std::io;
use std::iter;

pub async fn new() -> Result<(Client, impl Stream<Item = Event>, EventLoop), Box<dyn Error>> {
    let local_keys = identity::Keypair::generate_ed25519();

    let identify_config = IdentifyConfig::new(String::from("ipfs/1.0.0"), local_keys.public());
    let local_peer_id = local_keys.public().to_peer_id();

    let swarm = SwarmBuilder::new(
        libp2p::development_transport(local_keys).await?,
        ComposedBehaviour {
            identify: Identify::new(identify_config),
            kademlia: Kademlia::new(local_peer_id, MemoryStore::new(local_peer_id)),
            request_response: RequestResponse::new(
                FileExchangeCodec(),
                iter::once((FileExchangeProtocol(), ProtocolSupport::Full)),
                Default::default(),
            ),
        },
        local_peer_id,
    )
    .build();

    let (command_sender, command_receiver) = mpsc::channel(32);
    let (event_sender, event_receiver) = mpsc::channel(32);

    Ok((
        Client {
            sender: command_sender,
            local_peer_id,
        },
        event_receiver,
        EventLoop::new(swarm, command_receiver, event_sender),
    ))
}

#[derive(Clone)]
pub struct Client {
    sender: mpsc::Sender<Command>,
    pub local_peer_id: PeerId,
}

impl Client {
    pub async fn listen(&mut self, addr: Multiaddr) -> Result<(), Box<dyn Error + Send>> {
        debug!("p2p::Client::listen {:?}", addr);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Listen { addr, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn dial(&mut self, peer_addr: Multiaddr) -> Result<(), Box<dyn Error + Send>> {
        debug!("p2p::Client::dial {:?}", peer_addr);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Dial { peer_addr, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

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

    pub async fn provide(&mut self, hash: String) {
        debug!("p2p::Client::provide {:?}", hash);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::Provide { hash, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn list_providers(&mut self, hash: String) -> HashSet<PeerId> {
        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::ListProviders { hash, sender })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

    pub async fn request_artifact(
        &mut self,
        peer: &PeerId,
        hash: String,
    ) -> Result<Vec<u8>, Box<dyn Error + Send>> {
        debug!("p2p::Client::request_artifact {:?}: {:?}", peer, hash);

        let (sender, receiver) = oneshot::channel();
        self.sender
            .send(Command::RequestArtifact {
                hash,
                peer: *peer,
                sender,
            })
            .await
            .expect("Command receiver not to be dropped.");
        receiver.await.expect("Sender not to be dropped.")
    }

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

type PendingDialMap = HashMap<Multiaddr, oneshot::Sender<Result<(), Box<dyn Error + Send>>>>;
type PendingListPeersMap = HashMap<QueryId, oneshot::Sender<HashSet<PeerId>>>;
type PendingStartProvidingMap = HashMap<QueryId, oneshot::Sender<()>>;
type PendingRequestArtifactMap =
    HashMap<RequestId, oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>>;

pub struct EventLoop {
    swarm: Swarm<ComposedBehaviour>,
    command_receiver: mpsc::Receiver<Command>,
    event_sender: mpsc::Sender<Event>,
    pending_dial: PendingDialMap,
    pending_list_peers: PendingListPeersMap,
    pending_start_providing: PendingStartProvidingMap,
    pending_list_providers: PendingListPeersMap,
    pending_request_artifact: PendingRequestArtifactMap,
}

impl EventLoop {
    fn new(
        swarm: Swarm<ComposedBehaviour>,
        command_receiver: mpsc::Receiver<Command>,
        event_sender: mpsc::Sender<Event>,
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
        }
    }

    pub async fn run(mut self) {
        loop {
            futures::select! {
                event = self.swarm.next() => self.handle_event(event.expect("Swarm stream to be infinite.")).await  ,
                command = self.command_receiver.next() => match command {
                    Some(c) => {
                        let command_name = format!("{}", c);
                        debug!("Begin handle command {}", command_name);
                        self.handle_command(c).await;
                        debug!("End handle command {}", command_name);
                    },
                    // Command channel closed, thus shutting down the network event loop.
                    None=>  { warn!("Got empty command"); return },
                },
            }
        }
    }

    async fn handle_event(&mut self, event: SwarmEvent<ComposedEvent, impl Error>) {
        trace!("Handle SwarmEvent: {:?}", event);
        match event {
            SwarmEvent::Behaviour(ComposedEvent::Identify(IdentifyEvent::Pushed { .. })) => {}
            SwarmEvent::Behaviour(ComposedEvent::Identify(IdentifyEvent::Received {
                peer_id,
                info,
            })) => {
                println!("Identify::Received: {}; {:?}", peer_id, info);
                if let Some(addr) = info.listen_addrs.get(0) {
                    if let Some(sender) = self.pending_dial.remove(addr) {
                        let _ = sender.send(Ok(()));
                    }

                    debug!(
                        "Identify::Received: adding address {:?} for peer {}",
                        addr.clone(),
                        peer_id
                    );
                    self.swarm
                        .behaviour_mut()
                        .kademlia
                        .add_address(&peer_id, addr.clone());
                }
            }
            SwarmEvent::Behaviour(ComposedEvent::Identify(IdentifyEvent::Sent { .. })) => {}
            SwarmEvent::Behaviour(ComposedEvent::Identify(IdentifyEvent::Error { .. })) => {}
            SwarmEvent::Behaviour(ComposedEvent::Kademlia(
                KademliaEvent::OutboundQueryCompleted {
                    id,
                    result: QueryResult::GetClosestPeers(Ok(GetClosestPeersOk { key: _key, peers })),
                    ..
                },
            )) => {
                let _ = self
                    .pending_list_peers
                    .remove(&id)
                    .expect("Completed query to be previously pending.")
                    .send(HashSet::from_iter(peers));
            }
            SwarmEvent::Behaviour(ComposedEvent::Kademlia(
                KademliaEvent::OutboundQueryCompleted {
                    id,
                    result: QueryResult::StartProviding(_),
                    ..
                },
            )) => {
                let sender: oneshot::Sender<()> = self
                    .pending_start_providing
                    .remove(&id)
                    .expect("Completed query to be previously pending.");
                let _ = sender.send(());
            }
            SwarmEvent::Behaviour(ComposedEvent::Kademlia(
                KademliaEvent::OutboundQueryCompleted {
                    id,
                    result:
                        QueryResult::GetProviders(Ok(GetProvidersOk {
                            key: _key,
                            providers,
                            ..
                        })),
                    ..
                },
            )) => {
                let _ = self
                    .pending_list_providers
                    .remove(&id)
                    .expect("Completed query to be previously pending.")
                    .send(providers);
            }
            SwarmEvent::Behaviour(ComposedEvent::Kademlia(_)) => {}
            SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
                RequestResponseEvent::Message { message, .. },
            )) => match message {
                RequestResponseMessage::Request {
                    request, channel, ..
                } => {
                    self.event_sender
                        .send(Event::InboundRequest {
                            hash: request.0,
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
            SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
                RequestResponseEvent::InboundFailure { .. },
            )) => {}
            SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
                RequestResponseEvent::OutboundFailure {
                    request_id, error, ..
                },
            )) => {
                let _ = self
                    .pending_request_artifact
                    .remove(&request_id)
                    .expect("Request to still be pending.")
                    .send(Err(Box::new(error)));
            }
            SwarmEvent::Behaviour(ComposedEvent::RequestResponse(
                RequestResponseEvent::ResponseSent { .. },
            )) => {}
            SwarmEvent::NewListenAddr { address, .. } => {
                let local_peer_id = *self.swarm.local_peer_id();
                info!(
                    "Local node is listening on {:?}",
                    address.with(Protocol::P2p(local_peer_id.into()))
                );
            }
            SwarmEvent::ConnectionEstablished { .. } => {}
            SwarmEvent::ConnectionClosed { .. } => {}
            SwarmEvent::OutgoingConnectionError { .. } => {}
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

    async fn handle_command(&mut self, command: Command) {
        match command {
            Command::Listen { addr, sender } => {
                let _ = match self.swarm.listen_on(addr) {
                    Ok(_) => sender.send(Ok(())),
                    Err(e) => sender.send(Err(Box::new(e))),
                };
            }
            Command::Dial { peer_addr, sender } => {
                if let Vacant(_) = self.pending_dial.entry(peer_addr.clone()) {
                    match self.swarm.dial(peer_addr.clone()) {
                        Ok(()) => {
                            self.pending_dial.insert(peer_addr, sender);
                        }
                        Err(e) => {
                            let _ = sender.send(Err(Box::new(e)));
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
            Command::Provide { hash, sender } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .start_providing(hash.into_bytes().into())
                    .expect("No store error.");
                self.pending_start_providing.insert(query_id, sender);
            }
            Command::ListProviders { hash, sender } => {
                let query_id = self
                    .swarm
                    .behaviour_mut()
                    .kademlia
                    .get_providers(hash.into_bytes().into());
                self.pending_list_providers.insert(query_id, sender);
            }
            Command::RequestArtifact { hash, peer, sender } => {
                let request_id = self
                    .swarm
                    .behaviour_mut()
                    .request_response
                    .send_request(&peer, ArtifactRequest(hash));
                self.pending_request_artifact.insert(request_id, sender);
            }
            Command::RespondArtifact { artifact, channel } => {
                self.swarm
                    .behaviour_mut()
                    .request_response
                    .send_response(channel, ArtifactResponse(artifact))
                    .expect("Connection to peer to be still open.");
            }
        }
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(out_event = "ComposedEvent")]
struct ComposedBehaviour {
    identify: Identify,
    kademlia: Kademlia<MemoryStore>,
    request_response: RequestResponse<FileExchangeCodec>,
}

#[derive(Debug)]
enum ComposedEvent {
    Identify(IdentifyEvent),
    Kademlia(KademliaEvent),
    RequestResponse(RequestResponseEvent<ArtifactRequest, ArtifactResponse>),
}

impl From<IdentifyEvent> for ComposedEvent {
    fn from(event: IdentifyEvent) -> Self {
        ComposedEvent::Identify(event)
    }
}

impl From<KademliaEvent> for ComposedEvent {
    fn from(event: KademliaEvent) -> Self {
        ComposedEvent::Kademlia(event)
    }
}

impl From<RequestResponseEvent<ArtifactRequest, ArtifactResponse>> for ComposedEvent {
    fn from(event: RequestResponseEvent<ArtifactRequest, ArtifactResponse>) -> Self {
        ComposedEvent::RequestResponse(event)
    }
}

#[derive(Debug)]
enum Command {
    Listen {
        addr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    Dial {
        peer_addr: Multiaddr,
        sender: oneshot::Sender<Result<(), Box<dyn Error + Send>>>,
    },
    ListPeers {
        peer_id: PeerId,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    Provide {
        hash: String,
        sender: oneshot::Sender<()>,
    },
    ListProviders {
        hash: String,
        sender: oneshot::Sender<HashSet<PeerId>>,
    },
    RequestArtifact {
        hash: String,
        peer: PeerId,
        sender: oneshot::Sender<Result<Vec<u8>, Box<dyn Error + Send>>>,
    },
    RespondArtifact {
        artifact: Vec<u8>,
        channel: ResponseChannel<ArtifactResponse>,
    },
}

impl Display for Command {
    fn fmt(&self, f: &mut Formatter) -> std::fmt::Result {
        let name = match self {
            Command::Listen { .. } => "Listen",
            Command::Dial { .. } => "Dial",
            Command::ListPeers { .. } => "ListPeers",
            Command::Provide { .. } => "Provide",
            Command::ListProviders { .. } => "ListProviders",
            Command::RequestArtifact { .. } => "RequestArtifact",
            Command::RespondArtifact { .. } => "RespondArtifact",
        };
        write!(f, "{}", name)
    }
}

#[derive(Debug)]
pub enum Event {
    InboundRequest {
        hash: String,
        channel: ResponseChannel<ArtifactResponse>,
    },
}

#[derive(Debug, Clone)]
struct FileExchangeProtocol();
#[derive(Clone)]
struct FileExchangeCodec();
#[derive(Debug, Clone, PartialEq, Eq)]
struct ArtifactRequest(String);
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArtifactResponse(Vec<u8>);

impl ProtocolName for FileExchangeProtocol {
    fn protocol_name(&self) -> &[u8] {
        "/file-exchange/1".as_bytes()
    }
}

#[async_trait]
impl RequestResponseCodec for FileExchangeCodec {
    type Protocol = FileExchangeProtocol;
    type Request = ArtifactRequest;
    type Response = ArtifactResponse;

    async fn read_request<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let vec = read_length_prefixed(io, 100_000_000).await?;

        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        Ok(ArtifactRequest(String::from_utf8(vec).unwrap()))
    }

    async fn read_response<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        let vec = read_length_prefixed(io, 100_000_000).await?;

        if vec.is_empty() {
            return Err(io::ErrorKind::UnexpectedEof.into());
        }

        Ok(ArtifactResponse(vec))
    }

    async fn write_request<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
        ArtifactRequest(data): ArtifactRequest,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_length_prefixed(io, data).await?;
        io.close().await?;

        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &FileExchangeProtocol,
        io: &mut T,
        ArtifactResponse(data): ArtifactResponse,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        write_length_prefixed(io, data).await?;
        io.close().await?;

        Ok(())
    }
}
