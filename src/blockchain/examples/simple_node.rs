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
    floodsub::{self, Floodsub},
    identity,
    mdns::Mdns,
    mplex, noise,
    swarm::{SwarmBuilder, SwarmEvent},
    tcp::TokioTcpConfig,
    Multiaddr, PeerId, Transport,
};
use rand::Rng;
use std::error::Error;
use tokio::io::{self, AsyncBufReadExt};

use pyrsia_blockchain_network::crypto::hash_algorithm::HashDigest;
use pyrsia_blockchain_network::*;

pub const CONTINUE_COMMIT: &str = "1"; // Allow to continuously commit
pub const APART_ONE_COMMIT: &str = "2"; // Must be at least one ledger apart to commit

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a random PeerId
    let id_keys = blockchain::generate_ed25519();
    let peer_id = PeerId::from(id_keys.public());
    println!("Local peer id: {:?}", peer_id);
    let filepath = match std::env::args().nth(1) {
        Some(v) => v,
        None => String::from(storage::BLOCK_FILE_PATH),
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

    // Create a Swarm to manage peers and events.
    let mut swarm = {
        let mdns = Mdns::new(Default::default()).await?;
        let mut behaviour = network::Behaviour {
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

    storage::append_genesis_block(&filepath, &ed25519_keypair);

    let local_id = HashDigest::new(&block::get_publickey_from_keypair(&ed25519_keypair).encode());
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
                    ),
                    &ed25519_keypair,
                );
                transactions.push(transaction);
                let (parent_hash, previous_number, previous_commiter) = storage::read_last_block(&filepath);

                if check_number == APART_ONE_COMMIT && previous_commiter == local_id {
                        println!("The Commit Permission is limited, Please wait others commit");
                        continue;
                }

                let block = blockchain::new_block(&ed25519_keypair, &transactions, parent_hash, previous_number);
                println!("---------");
                println!("---------");
                println!("Add a New Block : {:?}", block);
                swarm.behaviour_mut().floodsub.publish(floodsub_topic.clone(), bincode::serialize(&block).unwrap());
                storage::write_block(&filepath, block);
            }
            event = swarm.select_next_some() => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    println!("Listening on {:?}", address);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn test_main() -> Result<(), String> {
        let result = main();
        assert!(result.is_err());
        Ok(())
    }
}
