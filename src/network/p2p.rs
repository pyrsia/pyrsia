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

use crate::network::artifact_protocol::{ArtifactExchangeCodec, ArtifactExchangeProtocol};
use crate::network::behaviour::PyrsiaNetworkBehaviour;
use crate::network::blockchain_protocol::{BlockchainExchangeCodec, BlockchainExchangeProtocol};
use crate::network::client::Client;
use crate::network::event_loop::{PyrsiaEvent, PyrsiaEventLoop};
use crate::network::idle_metric_protocol::{IdleMetricExchangeCodec, IdleMetricExchangeProtocol};
use crate::util::keypair_util;
use crate::util::keypair_util::KEYPAIR_FILENAME;

use crate::network::build_protocol::{BuildExchangeCodec, BuildExchangeProtocol};
use crate::network::build_status_protocol::{
    BuildStatusExchangeCodec, BuildStatusExchangeProtocol,
};
use libp2p::identity::Keypair;
use libp2p::kad::record::store::{MemoryStore, MemoryStoreConfig};
use libp2p::request_response::{ProtocolSupport, RequestResponse};
use libp2p::swarm::{Swarm, SwarmBuilder};
use libp2p::{
    autonat, core, dns, gossipsub, identify, identity, kad, mplex, noise, tcp, yamux, Transport,
};
use std::collections::hash_map::DefaultHasher;
use std::error::Error;
use std::hash::{Hash, Hasher};
use std::iter;
use std::time::Duration;
use tokio::sync::mpsc;
use tokio_stream::wrappers::ReceiverStream;
use tokio_stream::Stream;

/// Sets up the libp2p [`Swarm`] with the necessary components, doing the following things:
///
/// * load a keypair that is used for the libp2p identity
/// * create a libp2p swarm
/// * create a mpsc channel for sending and receiving client commands
/// * create a mpsc channel for sending and receiving custom events
/// * create a [`Client`] for sending client commands
/// * create an [`PyrsiaEventLoop`] to process swarm events and client commands
///
/// The Swarm is created with a [`NetworkBehaviour`] that is implemented by the
/// [`PyrsiaNetworkBehaviour`]. The PyrsiaNetworkBehaviour contains the following
/// components:
///
/// * autonat: a protocol for establishing Network Address Translation function
/// * Identify: a protocol for exchanging identity information between peers
/// * Kademlia: a DHT to share information over the libp2p network
/// * RequestResponse: a generic request/response protocol implementation for
/// the [`FileExchangeProtocol`]
///
/// The maximum number of provided keys for the memory store that is used by
/// Kademlia can be provided with the `max_provided_keys` parameter. This number
/// should be equal to or higher than the total number of artifacts and manifests
/// that the pyrsia node will be providing.
///
/// The Client uses the command channel to send commands that interact with the libp2p
/// network. This is the main entry point for an application to perform actions on the
/// libp2p network, i.e. dialing other peers, listing available providers, ...
///
/// The PyrsiaEventLoop uses the swarm and command channel for receiving swarm events
/// and client commands respectively. It implements the actual logic of the commands
/// by interacting with the libp2p swarm. The run method of the PyrsiaEventLoop must
/// be called in order to start listening for swarm events and client commands.
/// Ideally, this is done in a separate thread.
///
/// To get an idea of how these components are used, we explain this by following what
/// happens when a client wants to announce itself as a provider of a specific hash.
///
/// 1. An application calls: `client.provide(&some_hash)`.
/// 2. The Client creates a oneshot channel.
/// 3. The Client creates a [`Command::Provide`] that contains the hash and the sender
///    of the oneshot channel.
/// 4. The Client sends the command to the sender of the command channel.
/// 5. The Client now awaits the receiver of the oneshot channel for the incoming
///    response by the oneshot sender.
/// 6. The PyrsiaEventLoop receives the command via the receiver of the command channel.
/// 7. The PyrsiaEventLoop calls `handle_command` to start processing the command.
/// 8. The implementation of Command::Provide will announce itself as a provider of
///    `some_hash` on the Kademlia DHT and receives a QueryId.
/// 9. The QueryId is stored in a map with the QueryId as the key and the sender of the
///    oneshot channel as the value (which was passed down with the command).
/// 10. The Kademlia DHT is doing its thing to make the peer known as a provider of the
///     hash. When the operation has finished, Kademlia sends a Swarm event to notify
///     the completion.
/// 11. The PyrsiaEventLoop receives the event via the swarm listener.
/// 12. The PyrsiaEventLoop calls `handle_kademlia_event` to start processing the event.
/// 13. The Kademlia Event contains the QueryId and the Key that was provided.
/// 14. The PyrsiaEventLoop looks up the oneshot sender in the map via the QueryId.
/// 15. The PyrsiaEventLoop sends the result Ok() on the oneshot sender.
/// 16. The Client receiver receives the incoming response and can now safely return
///     to the application.
///
/// This function returns the following components:
///  * the Client
///  * the receiver part of the event channel
///  * the PyrsiaEventLoop
pub fn setup_libp2p_swarm(
    max_provided_keys: usize,
) -> Result<
    (
        Client,
        Keypair,
        impl Stream<Item = PyrsiaEvent>,
        PyrsiaEventLoop,
    ),
    Box<dyn Error>,
> {
    let local_keypair = keypair_util::load_or_generate_ed25519(KEYPAIR_FILENAME.as_str());

    let (swarm, local_peer_id) = create_swarm(local_keypair.clone(), max_provided_keys)?;
    let (command_sender, command_receiver) = mpsc::channel(32);
    let (event_sender, event_receiver) = mpsc::channel(32);

    let mut pyrsia_event_loop = PyrsiaEventLoop::new(swarm, command_receiver, event_sender);
    pyrsia_event_loop.initialize()?;

    Ok((
        Client::new(command_sender, local_peer_id),
        local_keypair,
        ReceiverStream::new(event_receiver),
        pyrsia_event_loop,
    ))
}

// create the libp2p transport for the swarm
fn create_transport(
    keypair: identity::Keypair,
) -> std::io::Result<core::transport::Boxed<(core::PeerId, core::muxing::StreamMuxerBox)>> {
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&keypair)
        .expect("Signing libp2p-noise static DH keypair failed.");

    let transport = tcp::tokio::Transport::new(tcp::Config::default().nodelay(true));
    let dns = dns::TokioDnsConfig::system(transport)?;

    Ok(dns
        .upgrade(core::upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(core::upgrade::SelectUpgrade::new(
            yamux::YamuxConfig::default(),
            mplex::MplexConfig::default(),
        ))
        .timeout(std::time::Duration::from_secs(20))
        .boxed())
}

// create the libp2p swarm
fn create_swarm(
    keypair: identity::Keypair,
    max_provided_keys: usize,
) -> Result<(Swarm<PyrsiaNetworkBehaviour>, core::PeerId), Box<dyn Error>> {
    let peer_id = keypair.public().to_peer_id();

    let identify_config = identify::Config::new("ipfs/1.0.0".to_owned(), keypair.public());

    let memory_store_config = MemoryStoreConfig {
        max_provided_keys,
        ..Default::default()
    };

    let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
        .validation_mode(gossipsub::ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
        .message_id_fn(|message: &gossipsub::GossipsubMessage| {
            // To content-address message, we can take the hash of message and use it as an ID.
            let mut hasher = DefaultHasher::new();
            message.data.hash(&mut hasher);
            gossipsub::MessageId::from(hasher.finish().to_string())
        }) // content-address messages. No two messages of the same content will be propagated.
        .build()?;

    Ok((
        SwarmBuilder::with_tokio_executor(
            create_transport(keypair.clone())?,
            PyrsiaNetworkBehaviour {
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
                gossipsub: gossipsub::Gossipsub::new(
                    gossipsub::MessageAuthenticity::Signed(keypair),
                    gossipsub_config,
                )?,
                identify: identify::Behaviour::new(identify_config),
                kademlia: kad::Kademlia::new(
                    peer_id,
                    MemoryStore::with_config(peer_id, memory_store_config),
                ),
                request_response: RequestResponse::new(
                    ArtifactExchangeCodec(),
                    iter::once((ArtifactExchangeProtocol(), ProtocolSupport::Full)),
                    Default::default(),
                ),
                build_request_response: RequestResponse::new(
                    BuildExchangeCodec(),
                    iter::once((BuildExchangeProtocol(), ProtocolSupport::Full)),
                    Default::default(),
                ),
                idle_metric_request_response: RequestResponse::new(
                    IdleMetricExchangeCodec(),
                    iter::once((IdleMetricExchangeProtocol(), ProtocolSupport::Full)),
                    Default::default(),
                ),
                blockchain_request_response: RequestResponse::new(
                    BlockchainExchangeCodec(),
                    iter::once((BlockchainExchangeProtocol(), ProtocolSupport::Full)),
                    Default::default(),
                ),
                build_status_request_response: RequestResponse::new(
                    BuildStatusExchangeCodec(),
                    iter::once((BuildStatusExchangeProtocol(), ProtocolSupport::Full)),
                    Default::default(),
                ),
            },
            peer_id,
        )
        .build(),
        peer_id,
    ))
}
