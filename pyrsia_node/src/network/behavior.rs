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
    NetworkBehaviour,
};
use log::info;

// We create a custom network behaviour that combines floodsub and mDNS.
// The derive generates a delegating `NetworkBehaviour` impl which in turn
// requires the implementations of `NetworkBehaviourEventProcess` for
// the events of each behaviour.
#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]
pub struct MyBehaviour {
    floodsub: Floodsub,
    kademlia: Kademlia<MemoryStore>,
    mdns: Mdns,
}

impl MyBehaviour {
    pub fn new(floodsub: Floodsub, kademlia: Kademlia<MemoryStore>, mdns: Mdns) -> Self {
        MyBehaviour {
            floodsub,
            kademlia,
            mdns,
        }
    }

    pub fn floodsub_mut(&mut self) -> &mut Floodsub {
        &mut self.floodsub
    }

    pub fn lookup_blob(&mut self, hash: String) -> Result<QueryId, Error> {
        let num = std::num::NonZeroUsize::new(2).ok_or(Error::ValueTooLarge)?;
        Ok(self.kademlia.get_record(&Key::new(&hash), Quorum::N(num)))
    }

    pub fn advertise_blob(&mut self, hash: String, value: Vec<u8>) -> Result<QueryId, Error> {
        let num = std::num::NonZeroUsize::new(2).ok_or(Error::ValueTooLarge)?;
        self.kademlia
            .put_record(Record::new(Key::new(&hash), value), Quorum::N(num))
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
