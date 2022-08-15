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

use super::crypto::hash_algorithm::HashDigest;
use super::identities::verify_key::VerifyKey;
use super::signature::{MultiSignature, Signature};
use super::structures::block::Block;

use aleph_bft::{NodeIndex, Recipient, TaskHandle};
use codec::{Decode, Encode};
use futures::{
    channel::{
        mpsc::{self, UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    prelude::*,
    Future, FutureExt, StreamExt,
};
use libp2p::core::identity::ed25519::PublicKey;
use libp2p::tcp::{self, GenTcpConfig};
use libp2p::{
    core::upgrade,
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex,
    noise,
    request_response::{
        ProtocolSupport, RequestResponse, RequestResponseCodec, RequestResponseConfig,
        RequestResponseEvent, RequestResponseMessage,
    },
    swarm::{NetworkBehaviourEventProcess, SwarmBuilder},
    NetworkBehaviour,
    PeerId,
    Swarm,
    Transport,
    // NOTE: `TokioTcpTransport` is available through the `tcp-tokio` feature.
    // tcp::{GenTcpConfig, TokioTcpTransport},
};
use log::{debug, info, trace, warn};
use std::{collections::HashMap, error::Error, io, iter, time::Duration};

#[derive(Clone)]
pub struct Spawner;

impl aleph_bft::SpawnHandle for Spawner {
    fn spawn(&self, _: &str, task: impl Future<Output = ()> + Send + 'static) {
        tokio::spawn(task);
    }
    fn spawn_essential(
        &self,
        _: &str,
        task: impl Future<Output = ()> + Send + 'static,
    ) -> TaskHandle {
        Box::pin(async move { tokio::spawn(task).await.map_err(|_| ()) })
    }
}

const PYRSIA_BLOCKCHAIN_PROTOCOL_NAME: &str = "/pyrsia/blockchain/1";

pub type NetworkData = aleph_bft::NetworkData<HashDigest, Block, Signature, MultiSignature>;

#[allow(clippy::large_enum_variant)]
#[derive(Clone, Encode, Decode)]
enum Message {
    Auth(NodeIndex),
    Consensus(NetworkData),
    Block(Block),
    PublicKey(NodeIndex, VerifyKey),
}

/// Implements the libp2p [`RequestResponseCodec`] trait.
/// GenericCodec is a suitably adjusted version of the GenericCodec implemented in sc-network in substrate.
/// Defines how streams of bytes are turned into requests and responses and vice-versa.
#[derive(Debug, Clone)]
pub struct GenericCodec {}

type Request = Vec<u8>;
// The Response type is empty -- we use RequestResponse just to send regular messages (requests).
type Response = ();

#[async_trait::async_trait]
impl RequestResponseCodec for GenericCodec {
    type Protocol = Vec<u8>;
    type Request = Request;
    type Response = Response;

    async fn read_request<T>(
        &mut self,
        _: &Self::Protocol,
        mut io: &mut T,
    ) -> io::Result<Self::Request>
    where
        T: AsyncRead + Unpin + Send,
    {
        let length = unsigned_varint::aio::read_usize(&mut io)
            .await
            .map_err(|err| io::Error::new(io::ErrorKind::InvalidInput, err))?;
        let mut buffer = vec![0; length];
        io.read_exact(&mut buffer).await?;
        Ok(buffer)
    }

    async fn read_response<T>(
        &mut self,
        _: &Self::Protocol,
        mut _io: &mut T,
    ) -> io::Result<Self::Response>
    where
        T: AsyncRead + Unpin + Send,
    {
        Ok(())
    }

    async fn write_request<T>(
        &mut self,
        _: &Self::Protocol,
        io: &mut T,
        req: Self::Request,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        let mut buffer = unsigned_varint::encode::usize_buffer();
        io.write_all(unsigned_varint::encode::usize(req.len(), &mut buffer))
            .await?;

        io.write_all(&req).await?;

        io.close().await?;
        Ok(())
    }

    async fn write_response<T>(
        &mut self,
        _: &Self::Protocol,
        _io: &mut T,
        _res: Self::Response,
    ) -> io::Result<()>
    where
        T: AsyncWrite + Unpin + Send,
    {
        Ok(())
    }
}

#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct Behaviour {
    mdns: Mdns,
    rq_rp: RequestResponse<GenericCodec>,

    #[behaviour(ignore)]
    peers: Vec<PeerId>,
    #[behaviour(ignore)]
    peer_by_index: HashMap<NodeIndex, PeerId>,
    #[behaviour(ignore)]
    consensus_tx: mpsc::UnboundedSender<NetworkData>,
    #[behaviour(ignore)]
    block_tx: mpsc::UnboundedSender<Block>,
    #[behaviour(ignore)]
    node_ix: NodeIndex,
    #[behaviour(ignore)]
    public_key: VerifyKey,
    #[behaviour(ignore)]
    new_authority_tx: UnboundedSender<(NodeIndex, PublicKey)>,
}

impl Behaviour {
    fn send_consensus_message(&mut self, message: NetworkData, recipient: Recipient) {
        let message: Vec<u8> = Message::Consensus(message).encode();
        trace!("Dispatching consensus message: {}", hex::encode(&message));
        use Recipient::*;
        match recipient {
            Node(node_ix) => {
                if let Some(peer_id) = self.peer_by_index.get(&node_ix) {
                    self.rq_rp.send_request(peer_id, message);
                } else {
                    warn!("No peer_id known for node {:?}.", node_ix);
                }
            }
            Everyone => {
                for peer_id in self.peers.iter() {
                    self.rq_rp.send_request(peer_id, message.clone());
                }
            }
        }
    }

    fn send_block_message(&mut self, block: Block) {
        info!("✈️ Sending block {}", block.header.ordinal);
        let message = Message::Block(block).encode();
        for peer_id in self.peers.iter() {
            self.rq_rp.send_request(peer_id, message.clone());
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
    fn inject_event(&mut self, event: MdnsEvent) {
        if let MdnsEvent::Discovered(list) = event {
            trace!("Processing discovery event with new list {:?}", list);
            let auth_message = Message::Auth(self.node_ix).encode();
            let key_message = Message::PublicKey(self.node_ix, self.public_key.clone()).encode();
            for (peer, _) in list {
                if self.peers.iter().any(|p| *p == peer) {
                    continue;
                }
                self.peers.push(peer);
                trace!("Sending authentication message to {:?}", peer);
                self.rq_rp.send_request(&peer, auth_message.clone());

                trace!("Sending public key message to {:?}", peer);
                self.rq_rp.send_request(&peer, key_message.clone());
            }
        }
    }
}

impl NetworkBehaviourEventProcess<RequestResponseEvent<Request, Response>> for Behaviour {
    fn inject_event(&mut self, event: RequestResponseEvent<Request, Response>) {
        if let RequestResponseEvent::Message {
            peer: peer_id,
            message,
        } = event
        {
            match message {
                RequestResponseMessage::Request {
                    request_id: _,
                    request,
                    channel: _,
                } => {
                    if !self.peers.iter().any(|p| *p == peer_id) {
                        info!("An unknown {:?} has sent us a message!", peer_id);
                        self.peers.push(peer_id);

                        trace!("Sending authentication message to {:?}", peer_id);
                        let auth_message = Message::Auth(self.node_ix).encode();
                        self.rq_rp.send_request(&peer_id, auth_message);

                        trace!("Sending public key message to {:?}", peer_id);
                        let key_message =
                            Message::PublicKey(self.node_ix, self.public_key.clone()).encode();
                        self.rq_rp.send_request(&peer_id, key_message);
                    }

                    let result = Message::decode(&mut &request[..]);
                    match result {
                        Err(e) => warn!(
                            "Failed to decode inbound request as Message: {} -- {}",
                            e,
                            hex::encode(&request)
                        ),
                        Ok(message) => match message {
                            Message::Consensus(msg) => {
                                debug!("📌 New consensus message: {:?}", msg);

                                self.consensus_tx
                                    .unbounded_send(msg)
                                    .expect("Network must listen");
                            }
                            Message::Auth(node_ix) => {
                                debug!("🖇️ Authenticated peer: {:?} {:?}", node_ix, peer_id);

                                if self.peer_by_index.get(&node_ix).is_none() {
                                    trace!("Sending authentication message to {:?}", peer_id);
                                    let auth_message = Message::Auth(self.node_ix).encode();
                                    self.rq_rp.send_request(&peer_id, auth_message);

                                    trace!("Sending public key message to {:?}", peer_id);
                                    let key_message =
                                        Message::PublicKey(self.node_ix, self.public_key.clone())
                                            .encode();
                                    self.rq_rp.send_request(&peer_id, key_message);
                                }

                                self.peer_by_index.insert(node_ix, peer_id);
                            }
                            Message::Block(block) => {
                                debug!(
                                    "Received block num {:?} from {:?}",
                                    block.header.ordinal, peer_id
                                );
                                self.block_tx
                                    .unbounded_send(block)
                                    .expect("Blockchain process must listen");
                            }
                            Message::PublicKey(node_ix, key) => {
                                debug!("📑 Received a new public key from {:?}", node_ix);
                                self.new_authority_tx
                                    .unbounded_send((node_ix, key.public()))
                                    .expect("PublicKey process must listen");
                            }
                        }, // We do not send back a response to a request. We treat them simply as one-way messages.}
                    }
                }
                RequestResponseMessage::Response { .. } => {
                    //We ignore the response, as it is empty anyway.
                }
            }
        }
    }
}

pub struct Network {
    msg_to_manager_tx: mpsc::UnboundedSender<(NetworkData, Recipient)>,
    msg_from_manager_rx: mpsc::UnboundedReceiver<NetworkData>,
}

#[async_trait::async_trait]
impl aleph_bft::Network<HashDigest, Block, Signature, MultiSignature> for Network {
    fn send(&self, data: NetworkData, recipient: Recipient) {
        trace!("Sending a message to: {:?}", recipient);
        if let Err(e) = self.msg_to_manager_tx.unbounded_send((data, recipient)) {
            warn!("Failed network send: {:?}", e);
        }
    }
    async fn next_event(&mut self) -> Option<NetworkData> {
        let msg = self.msg_from_manager_rx.next().await;
        msg.map(|m| {
            trace!(
                "New event received of network data {}",
                hex::encode(m.encode())
            );
            m
        })
    }
}

pub struct NetworkManager {
    swarm: Swarm<Behaviour>,
    consensus_rx: UnboundedReceiver<(NetworkData, Recipient)>,
    block_rx: UnboundedReceiver<Block>,
}

impl Network {
    pub async fn new(
        node_ix: NodeIndex,
        key_pair: identity::ed25519::Keypair,
        peer_by_index: HashMap<NodeIndex, PeerId>,
        new_authority_tx: UnboundedSender<(NodeIndex, PublicKey)>,
    ) -> Result<
        (
            Self,
            NetworkManager,
            UnboundedSender<Block>,
            UnboundedReceiver<Block>,
            UnboundedSender<NetworkData>,
            UnboundedReceiver<NetworkData>,
        ),
        Box<dyn Error>,
    > {
        let local_key: identity::Keypair = identity::Keypair::Ed25519(key_pair.clone());
        let public_key: libp2p::identity::ed25519::PublicKey = key_pair.public();
        let local_peer_id = PeerId::from(local_key.public());
        info!("Local peer id: {:?}", local_peer_id);

        // Create a keypair for authenticated encryption of the transport.
        let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
            .into_authentic(&local_key)
            .expect("Signing libp2p-noise static DH keypair failed.");

        // Create a tokio-based TCP transport use noise for authenticated
        // encryption and Mplex for multiplexing of substreams on a TCP stream.
        let transport = tcp::TokioTcpTransport::new(GenTcpConfig::default().nodelay(true))
            .upgrade(upgrade::Version::V1)
            .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
            .multiplex(mplex::MplexConfig::new())
            .boxed();

        let (msg_to_manager_tx, msg_to_manager_rx) = mpsc::unbounded();
        let (msg_for_store, msg_from_manager) = mpsc::unbounded();
        let (msg_for_network, msg_from_store) = mpsc::unbounded();
        let (block_to_data_io_tx, block_to_data_io_rx) = mpsc::unbounded();
        let (block_from_data_io_tx, block_from_data_io_rx) = mpsc::unbounded();
        let mut swarm = {
            let mut rr_cfg = RequestResponseConfig::default();
            rr_cfg.set_connection_keep_alive(Duration::from_secs(10));
            rr_cfg.set_request_timeout(Duration::from_secs(4));
            let protocol_support = ProtocolSupport::Full;
            let rq_rp = RequestResponse::new(
                GenericCodec {},
                iter::once((
                    PYRSIA_BLOCKCHAIN_PROTOCOL_NAME.as_bytes().to_vec(),
                    protocol_support,
                )),
                rr_cfg,
            );

            let mdns = Mdns::new(Default::default()).await?;
            let behaviour = Behaviour {
                rq_rp,
                mdns,
                peers: vec![],
                peer_by_index,
                consensus_tx: msg_for_store,
                block_tx: block_to_data_io_tx,
                node_ix,
                public_key: VerifyKey { public: public_key },
                new_authority_tx,
            };
            SwarmBuilder::new(transport, behaviour, local_peer_id)
                .executor(Box::new(|fut| {
                    tokio::spawn(fut);
                }))
                .build()
        };

        swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

        let network = Network {
            msg_to_manager_tx,
            msg_from_manager_rx: msg_from_store,
        };

        let network_manager = NetworkManager {
            swarm,
            consensus_rx: msg_to_manager_rx,
            block_rx: block_from_data_io_rx,
        };

        Ok((
            network,
            network_manager,
            block_from_data_io_tx,
            block_to_data_io_rx,
            msg_for_network,
            msg_from_manager,
        ))
    }
}

impl NetworkManager {
    pub async fn run(&mut self, mut exit: oneshot::Receiver<()>) {
        loop {
            futures::select! {
                maybe_msg = self.consensus_rx.next() => {
                    if let Some((consensus_msg, recipient)) = maybe_msg {
                        let handle = &mut self.swarm.behaviour_mut();
                        handle.send_consensus_message(consensus_msg, recipient);
                    }
                }
                maybe_block = self.block_rx.next() => {
                    if let Some(block) = maybe_block {
                        let handle = &mut self.swarm.behaviour_mut();
                        handle.send_block_message(block);
                    }
                }
                event = self.swarm.next().fuse() => {
                    match event {
                        Some(event) => {
                            trace!("Received a swarm event: {:?}", event);
                        }
                        None => {
                            panic!("Swarm stream ended");
                        }
                    }
                }
               _ = &mut exit  => break,
            }
        }
    }
}
