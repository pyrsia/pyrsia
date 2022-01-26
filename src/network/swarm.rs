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

use super::behavior::MyBehaviour;
use super::transport::TcpTokioTransport;
use libp2p::gossipsub::MessageId;
use libp2p::gossipsub::{GossipsubMessage, IdentTopic, MessageAuthenticity, ValidationMode};
use libp2p::{
    floodsub::{Floodsub, Topic},
    kad::{record::store::MemoryStore, Kademlia},
    mdns::Mdns,
    swarm::SwarmBuilder,
    PeerId, Swarm,
};
use libp2p::{gossipsub, identity};
use std::collections::hash_map::DefaultHasher;

use std::hash::{Hash, Hasher};
use std::time::Duration;
use crate::node_manager::handlers::ART_MGR;

pub type MyBehaviourSwarm = Swarm<MyBehaviour>;

pub async fn new(
    gossip_topic: IdentTopic,
    topic: Topic,
    transport: TcpTokioTransport,
    local_key: identity::Keypair,
    response_sender: tokio::sync::mpsc::Sender<String>,
) -> Result<MyBehaviourSwarm, ()> {
    let local_peer_id = PeerId::from(local_key.public());
    //create kad
    let store = MemoryStore::new(local_peer_id);
    let kademlia = Kademlia::new(local_peer_id, store);
    ART_MGR.set_peer_id(local_peer_id).expect("Failed to set peer_id in artifact manager.");

    // To content-address message, we can take the hash of message and use it as an ID.
    let message_id_fn = |message: &GossipsubMessage| {
        let mut s = DefaultHasher::new();
        message.data.hash(&mut s);
        MessageId::from(s.finish().to_string())
    };

    // Set a custom gossipsub
    let gossipsub_config = gossipsub::GossipsubConfigBuilder::default()
        .heartbeat_interval(Duration::from_secs(10)) // This is set to aid debugging by not cluttering the log space
        .validation_mode(ValidationMode::Strict) // This sets the kind of message validation. The default is Strict (enforce message signing)
        .message_id_fn(message_id_fn) // content-address messages. No two messages of the
        // same content will be propagated.
        .build()
        .expect("Valid config");
    // build a gossipsub network behaviour
    let mut gossipsub: gossipsub::Gossipsub =
        gossipsub::Gossipsub::new(MessageAuthenticity::Signed(local_key), gossipsub_config)
            .expect("Correct configuration");

    // subscribes to our gossip topic
    gossipsub.subscribe(&gossip_topic).unwrap();

    let mdns = Mdns::new(Default::default()).await.unwrap();
    let mut behaviour = MyBehaviour::new(
        gossipsub,
        Floodsub::new(local_peer_id),
        kademlia,
        mdns,
        response_sender,
    );
    behaviour.floodsub_mut().subscribe(topic.clone());

    let swarm = SwarmBuilder::new(transport, behaviour, local_peer_id)
        // We want the connection background tasks to be spawned
        // onto the tokio runtime.
        .executor(Box::new(|fut| {
            tokio::spawn(fut);
        }))
        .build();
    Ok(swarm)
}

// TODO: It would be nicer to have a struct with functions but the high level code is highly coupled with the API of libp2p's Swarm

// pub struct MyBehaviourSwarm {
//     swarm: Swarm<MyBehaviour>,
// }

// impl MyBehaviourSwarm {
//     pub async fn new(
//         topic: Topic,
//         transport: TcpTokioTransport,
//         peer_id: PeerId,
//     ) -> Result<MyBehaviourSwarm, ()> {
//         let mdns = Mdns::new(Default::default()).await.unwrap();
//         let mut behaviour = MyBehaviour::new(Floodsub::new(peer_id.clone()), mdns);
//         behaviour.floodsub().subscribe(topic.clone());
//         let swarm = SwarmBuilder::new(transport, behaviour, peer_id)
//             // We want the connection background tasks to be spawned
//             // onto the tokio runtime.
//             .executor(Box::new(|fut| {
//                 tokio::spawn(fut);
//             }))
//             .build();
//         Ok(MyBehaviourSwarm { swarm })
//     }
// }
