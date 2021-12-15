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
use libp2p::{
    floodsub::{Floodsub, Topic},
    kad::{record::store::MemoryStore, Kademlia},
    mdns::Mdns,
    swarm::SwarmBuilder,
    PeerId, Swarm,
};

pub type MyBehaviourSwarm = Swarm<MyBehaviour>;

pub async fn new(
    topic: Topic,
    transport: TcpTokioTransport,
    peer_id: PeerId,
) -> Result<MyBehaviourSwarm, ()> {
    let store = MemoryStore::new(peer_id);
    let kademlia = Kademlia::new(peer_id, store);
    let mdns = Mdns::new(Default::default()).await.unwrap();
    let mut behaviour = MyBehaviour::new(Floodsub::new(peer_id), kademlia, mdns);
    behaviour.floodsub_mut().subscribe(topic.clone());

    let swarm = SwarmBuilder::new(transport, behaviour, peer_id)
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
