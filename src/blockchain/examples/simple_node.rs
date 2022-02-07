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

extern crate pretty_env_logger;
extern crate pyrsia_blockchain_network;
extern crate tokio;

use futures::StreamExt;
use libp2p::{
    core::upgrade,
    floodsub::{self, Floodsub, FloodsubEvent},
    identity,
    mdns::{Mdns, MdnsEvent},
    mplex,
    noise,
    swarm::{NetworkBehaviourEventProcess, SwarmBuilder, SwarmEvent},
    // `TokioTcpConfig` is available through the `tcp-tokio` feature.
    tcp::TokioTcpConfig,
    Multiaddr,
    NetworkBehaviour,
    PeerId,
    Transport,
};

use pyrsia_blockchain_network::*;
use rand::Rng;
use std::error::Error;
use tokio::io::{self, AsyncBufReadExt};

pub const BLOCK_FILE_PATH: &str = "./blockchain_storage";
pub const CONTINUE_COMMIT: &str = "1"; //allow to continuously commit
pub const APART_ONE_COMMIT: &str = "2"; //must be at least one ledger apart to commit

/// The `tokio::main` attribute sets up a tokio runtime.
#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a random PeerId
    let id_keys = blockchain::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    println!("Local peer id: {:?}", peer_id);
    let filepath = match std::env::args().nth(1) {
        Some(v) => v,
        None => String::from(BLOCK_FILE_PATH),
    };

    // Create a keypair for authenticated encryption of the transport.
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&id_keys)
        .expect("Signing libp2p-noise static DH keypair failed.");

    // Create a tokio-based TCP transport use noise for authenticated
    // encryption and Mplex for multiplexing of substreams on a TCP stream.
    let transport = TokioTcpConfig::new()
        .nodelay(true)
        .upgrade(upgrade::Version::V1)
        .authenticate(noise::NoiseConfig::xx(noise_keys).into_authenticated())
        .multiplex(mplex::MplexConfig::new())
        .boxed();

    // Create a Floodsub topic
    let floodsub_topic = floodsub::Topic::new("block");

    // We create a custom network behaviour that combines floodsub and mDNS.
    // The derive generates a delegating `NetworkBehaviour` impl which in turn
    // requires the implementations of `NetworkBehaviourEventProcess` for
    // the events of each behaviour.
    #[derive(NetworkBehaviour)]
    #[behaviour(event_process = true)]
    // TODO(prince-chrismc): Give this a much better name!
    struct MyBehaviour {
        floodsub: Floodsub,
        mdns: Mdns,
    }

    impl NetworkBehaviourEventProcess<FloodsubEvent> for MyBehaviour {
        // Called when `floodsub` produces an event.
        fn inject_event(&mut self, message: FloodsubEvent) {
            if let FloodsubEvent::Message(message) = message {
                //println!("Received: '{:?}' from {:?}", &message.data, message.source);
                let block: block::Block =
                    bincode::deserialize::<block::Block>(&message.data).unwrap();
                println!("++++++++++");
                println!("++++++++++");
                println!("Recevie a new block {:?}", block);
                let filepath = match std::env::args().nth(1) {
                    Some(v) => v,
                    None => String::from("./first"),
                };

                write_block(filepath, block);
            }
        }
    }

    impl NetworkBehaviourEventProcess<MdnsEvent> for MyBehaviour {
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

    // Create a Swarm to manage peers and events.
    let mut swarm = {
        let mdns = Mdns::new(Default::default()).await?;
        let mut behaviour = MyBehaviour {
            floodsub: Floodsub::new(peer_id),
            mdns,
        };

        behaviour.floodsub.subscribe(floodsub_topic.clone());

        SwarmBuilder::new(transport, behaviour, peer_id)
            // We want the connection background tasks to be spawned
            // onto the tokio runtime.
            .executor(Box::new(|fut| {
                tokio::spawn(fut);
            }))
            .build()
    };

    // Reach out to another node if specified
    let mut check_number = String::from(CONTINUE_COMMIT);
    if let Some(number) = std::env::args().nth(2) {
        check_number = number;
    }

    if let Some(to_dial) = std::env::args().nth(3) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial(addr)?;
        println!("Dialed {:?}", to_dial)
    }

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let ed25519_keypair = match id_keys {
        identity::Keypair::Ed25519(v) => v,
        identity::Keypair::Rsa(_) => todo!(),
        identity::Keypair::Secp256k1(_) => todo!(),
    };

    let mut transactions = vec![];

    let local_id = header::hash(&block::get_publickey_from_keypair(&ed25519_keypair).encode());
    // Kick it off
    loop {
        tokio::select! {
            line = stdin.next_line() => {
                let line = line?.expect("stdin closed");
                let transaction = block::Transaction::new(
                    block::PartialTransaction::new(
                        block::TransactionType::Create,
                        local_id,
                        line.as_bytes().to_vec(),
                        rand::thread_rng().gen::<u128>(),
                    ),
                    &ed25519_keypair,
                );
                transactions.push(transaction);
                let (parent_hash, previous_number, previous_commiter)=read_last_block(filepath.clone());
                //let parent_hash=header::hash(b"");
                //let previous_number=0;

                if check_number==APART_ONE_COMMIT && previous_commiter == local_id{

                        println!("The Commit Permission is limited, Please wait others commit");
                        continue;

                }
                let block = blockchain::new_block(&ed25519_keypair, &transactions, parent_hash, previous_number);
                println!("---------");
                println!("---------");
                println!("Add a New Block : {:?}", block);
                swarm.behaviour_mut().floodsub.publish(floodsub_topic.clone(), bincode::serialize(&block).unwrap());
                write_block(filepath.clone(), block);
            }
            event = swarm.select_next_some() => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    println!("Listening on {:?}", address);
                }
            }
        }
    }
}

//Add genesis block to the file
pub fn append_genesis_block(path: String, key: &identity::ed25519::Keypair) {
    use blockchain::GenesisBlock;
    use std::fs::OpenOptions;
    use std::io::Write;

    let g_block = GenesisBlock::new(key);
    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .append(true)
        .open(path)
        .expect("cannot open file");

    file.write_all(&bincode::serialize(&g_block).unwrap())
        .expect("write failed");
}

//read a last block from the file, and return this block hash, this block number and this block committer
pub fn read_last_block(path: String) -> (header::HashDigest, u128, header::Address) {
    use std::io::{BufRead, BufReader};
    let file = std::fs::File::open(path).unwrap();

    let buffered = BufReader::new(file);
    let line = buffered.lines().last().expect("stdin to read").unwrap();
    let block: block::Block = serde_json::from_str(&line).unwrap();

    (
        block.header.current_hash,
        block.header.number,
        block.header.committer,
    )
}

//Write a block to the file
pub fn write_block(path: String, block: block::Block) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .create_new(true)
        .write(true)
        .append(true)
        .open(path)
        .expect("cannot open file");

    file.write_all(serde_json::to_string(&block).unwrap().as_bytes())
        .expect("write failed");
    file.write_all(b"\n").expect("write failed");
}

#[cfg(test)]
mod tests {
    extern crate pyrsia_blockchain_network;
    use super::*;
    use libp2p::identity;
    use pyrsia_blockchain_network::{block, header};
    use rand::Rng;

    #[test]
    fn test_write_read() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = header::hash(&block::get_publickey_from_keypair(&keypair).encode());
        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = block::Transaction::new(
            block::PartialTransaction::new(
                block::TransactionType::Create,
                local_id,
                data.as_bytes().to_vec(),
                rand::thread_rng().gen::<u128>(),
            ),
            &keypair,
        );
        transactions.push(transaction);
        let block_header = header::Header::new(header::PartialHeader::new(
            header::hash(b""),
            local_id,
            header::hash(b""),
            1,
            rand::thread_rng().gen::<u128>(),
        ));

        let block = block::Block::new(block_header, transactions.to_vec(), &keypair);
        append_genesis_block(BLOCK_FILE_PATH.to_string(), &keypair);
        write_block(BLOCK_FILE_PATH.to_string(), block);
        let (_, number, _) = read_last_block(BLOCK_FILE_PATH.to_string());

        assert_eq!(1, number);
        Ok(())
    }

    #[test]
    fn test_main() -> Result<(), String> {
        let result = main();
        assert!(result.is_err());
        Ok(())
    }
}
