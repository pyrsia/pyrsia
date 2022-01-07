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

use libp2p::{
    floodsub::{Floodsub, FloodsubEvent},
    kad::{
        record::{
            store::{Error, MemoryStore},
            Key,
        },
        AddProviderOk, Kademlia, KademliaEvent, PeerRecord, PutRecordOk, QueryId, QueryResult,
        Quorum, Record,
    },
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    NetworkBehaviour, PeerId,
};
use libp2p::{gossipsub, identity, swarm::SwarmEvent, Multiaddr};
use log::{debug, error, info};

use super::torrent::add_torrent;
use std::collections::HashSet;

// We create a custom network behaviour that combines floodsub and mDNS.
// The derive generates a delegating `NetworkBehaviour` impl which in turn
// requires the implementations of `NetworkBehaviourEventProcess` for
// the events of each behaviour.
#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct MyBehaviour {
    gossipsub: gossipsub::Gossipsub,
    floodsub: Floodsub,
    kademlia: Kademlia<MemoryStore>,
    mdns: Mdns,
    #[behaviour(ignore)]
    response_sender: tokio::sync::mpsc::Sender<String>,
}

impl MyBehaviour {
    pub fn new(
        gossipsub: gossipsub::Gossipsub,
        floodsub: Floodsub,
        kademlia: Kademlia<MemoryStore>,
        mdns: Mdns,
        response_sender: tokio::sync::mpsc::Sender<String>,
    ) -> Self {
        MyBehaviour {
            gossipsub,
            floodsub,
            kademlia,
            mdns,
            response_sender,
        }
    }

    pub fn gossipsub_mut(&mut self) -> &mut gossipsub::Gossipsub {
        &mut self.gossipsub
    }

    pub fn floodsub_mut(&mut self) -> &mut Floodsub {
        &mut self.floodsub
    }

    pub async fn lookup_blob(&mut self, hash: String) {
        let num = std::num::NonZeroUsize::new(2)
            .ok_or(Error::ValueTooLarge)
            .unwrap();
        self.kademlia.get_record(&Key::new(&hash), Quorum::N(num));
    }

    pub async fn list_peers(&mut self, peer_id: PeerId) {
        self.kademlia.get_closest_peers(peer_id);
    }

    pub async fn list_peers_cmd(&mut self) {
        match get_peers(&mut self.mdns) {
            Ok(val) => println!("Peers are : {}", val),
            Err(e) => error!("failed to get peers connected: {:?}", e),
        }
    }

    pub fn advertise_blob(&mut self, hash: String, value: Vec<u8>) -> Result<QueryId, Error> {
        let num = std::num::NonZeroUsize::new(2).ok_or(Error::ValueTooLarge)?;
        self.kademlia
            .put_record(Record::new(Key::new(&hash), value), Quorum::N(num))
    }
}

impl NetworkBehaviourEventProcess<gossipsub::GossipsubEvent> for MyBehaviour {
    // Called when `gossipsub` produces an event.
    fn inject_event(&mut self, message: gossipsub::GossipsubEvent) {
        if let gossipsub::GossipsubEvent::Message {
            propagation_source,
            message_id,
            message,
        } = message
        {
            let msg_data: String = String::from_utf8(message.data).unwrap();
            if msg_data.starts_with("magnet:") {
                // Synapse RPC Integration point
                info!("Start downloading {}", msg_data);
                let server = "ws://localhost:8412/";
                let pass = "donthackme";
                let directory: Option<&str> = Some("/tmp");
                let files: Vec<&str> = vec![msg_data.as_str()];
                let r = super::torrent::add_torrent(server, pass, directory, files);
                match futures::executor::block_on(r) {
                    Err(e) => info!("Error: {}", e),
                    _ => info!("Added magnet {}", msg_data),
                };
                // This should kick-off the download
            } else {
                info!(
                    "Got message: {} with id: {} from peer: {:?}",
                    msg_data, message_id, propagation_source
                );
            }
        }
    }
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for MyBehaviour {
    // Called when `floodsub` produces an event.
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            info!(
                "Received: '{:?}' from {:?}",
                String::from_utf8_lossy(&message.data),
                message.source
            );
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, multiaddr) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                    self.kademlia.add_address(&peer, multiaddr);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, multiaddr) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                        self.kademlia.remove_address(&peer, &multiaddr);
                    }
                }
            }
        }
    }
}

impl NetworkBehaviourEventProcess<KademliaEvent> for MyBehaviour {
    // Called when `kademlia` produces an event.
    fn inject_event(&mut self, message: KademliaEvent) {
        match message {
            KademliaEvent::OutboundQueryCompleted { result, .. } => match result {
                QueryResult::GetProviders(Ok(ok)) => {
                    for peer in ok.providers {
                        println!(
                            "Peer {:?} provides key {:?}",
                            peer,
                            std::str::from_utf8(ok.key.as_ref()).unwrap()
                        );
                    }
                }

                QueryResult::GetClosestPeers(Ok(ok)) => {
                    println!("GetClosestPeers result {:?}", ok.peers);
                    let connected_peers = itertools::join(ok.peers, ",");
                    respond_send(self.response_sender.clone(), connected_peers);
                }

                QueryResult::GetProviders(Err(err)) => {
                    eprintln!("Failed to get providers: {:?}", err);
                }
                QueryResult::GetRecord(Ok(ok)) => {
                    for PeerRecord {
                        record: Record { key, value, .. },
                        ..
                    } in ok.records
                    {
                        println!(
                            "Got record {:?} {:?}",
                            std::str::from_utf8(key.as_ref()).unwrap(),
                            std::str::from_utf8(&value).unwrap(),
                        );
                    }
                }
                QueryResult::GetRecord(Err(err)) => {
                    eprintln!("Failed to get record: {:?}", err);
                }
                QueryResult::PutRecord(Ok(PutRecordOk { key })) => {
                    println!(
                        "Successfully put record {:?}",
                        std::str::from_utf8(key.as_ref()).unwrap()
                    );
                }
                QueryResult::PutRecord(Err(err)) => {
                    eprintln!("Failed to put record: {:?}", err);
                }
                QueryResult::StartProviding(Ok(AddProviderOk { key })) => {
                    println!(
                        "Successfully put provider record {:?}",
                        std::str::from_utf8(key.as_ref()).unwrap()
                    );
                }
                QueryResult::StartProviding(Err(err)) => {
                    eprintln!("Failed to put provider record: {:?}", err);
                }
                _ => {}
            },
            _ => {}
        }
    }
}
pub fn respond_send(response_sender: tokio::sync::mpsc::Sender<String>, response: String) {
    tokio::spawn(async move {
        match response_sender.send(response).await {
            Ok(_) => debug!("response for list_peers sent"),
            Err(_) => error!("failed to send response"),
        }
    });
}
pub fn get_peers(mdns: &mut Mdns) -> Result<String, Error> {
    let nodes = mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    let connected_peers = itertools::join(&unique_peers, ", ");
    Ok(connected_peers)
}
