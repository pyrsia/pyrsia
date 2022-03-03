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
    mdns::{Mdns, MdnsEvent},
    swarm::NetworkBehaviourEventProcess,
    NetworkBehaviour,
};

use super::*;

#[derive(NetworkBehaviour)]
#[behaviour(event_process = true)]

pub struct Behaviour {
    pub floodsub: Floodsub,
    pub mdns: Mdns,
}

impl NetworkBehaviourEventProcess<FloodsubEvent> for Behaviour {
    // Called when `floodsub` produces an event.
    fn inject_event(&mut self, message: FloodsubEvent) {
        if let FloodsubEvent::Message(message) = message {
            //println!("Received: '{:?}' from {:?}", &message.data, message.source);
            let block: block::Block = bincode::deserialize::<block::Block>(&message.data).unwrap();
            println!("++++++++++"); // TODO(fishseasbowl): Replace with logging methods
            println!("++++++++++");
            println!("Recevie a new block {:?}", block);
            let _filepath = match std::env::args().nth(1) {
                Some(v) => v,
                None => String::from("./first"),
            };
            // this needs to be blockchain.add_block(block)
            //write_block(filepath, block);
        }
    }
}

impl NetworkBehaviourEventProcess<MdnsEvent> for Behaviour {
    // Called when `mdns` produces an event.
    fn inject_event(&mut self, event: MdnsEvent) {
        match event {
            MdnsEvent::Discovered(list) => {
                for (peer, _) in list {
                    self.floodsub.add_node_to_partial_view(peer);
                }
            }
            MdnsEvent::Expired(list) => {
                for (peer, _) in list {
                    if !self.mdns.has_node(&peer) {
                        self.floodsub.remove_node_from_partial_view(&peer);
                    }
                }
            }
        }
    }
}
