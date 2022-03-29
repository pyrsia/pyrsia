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
use std::error::Error;
use tokio::io::{self, AsyncBufReadExt};

use pyrsia_blockchain_network::blockchain::Blockchain;
use pyrsia_blockchain_network::network::Behaviour;
use pyrsia_blockchain_network::structures::{
    block::Block,
    transaction::{Transaction, TransactionType},
};

pub const BLOCK_FILE_PATH: &str = "./blockchain_storage";

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    // Create a random PeerId
    let id_keys = identity::ed25519::Keypair::generate();
    let peer_id = PeerId::from(identity::PublicKey::Ed25519(id_keys.public()));

    println!("Local peer id: {:?}", peer_id);
    let _filepath = match std::env::args().nth(1) {
        Some(v) => v,
        None => String::from(BLOCK_FILE_PATH),
    };

    // Create a keypair for authenticated encryption of the transport.
    let noise_keys = noise::Keypair::<noise::X25519Spec>::new()
        .into_authentic(&libp2p::identity::Keypair::Ed25519(id_keys.clone()))
        .expect("Signing libp2p-noise static DH keypair failed.");

    let mut chain = Blockchain::new(&id_keys);
    chain.add_block_listener(move |b: Block| {
        println!("---------");
        println!("---------");
        println!("Add a New Block : {:?}", b);
        // TODO(chb0github): Should be wrapped in mutex
        // write_block(&filepath.clone(), b);
    });

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
        let mut behaviour = Behaviour {
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

    if let Some(to_dial) = std::env::args().nth(3) {
        let addr: Multiaddr = to_dial.parse()?;
        swarm.dial(addr)?;
        println!("Dialed {:?}", to_dial);
    }

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    // Kick it off
    loop {
        tokio::select! {
            line = stdin.next_line() => {
                let l = line.expect("stdin closed");
                let transaction = Transaction::new(
                        TransactionType::Create,
                        peer_id.into(),
                        l.unwrap().as_bytes().to_vec(),
                    &id_keys,
                );

                // eventually this will trigger a block action
                chain.submit_transaction(transaction.clone(),move |t: Transaction| {
                    println!("transaction {:?} submitted",t);
                });
            }
            event = swarm.select_next_some() => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    println!("Listening on {:?}", address);
                }
            }
        }
    }
}

pub fn write_block(path: &str, block: Block) {
    use std::fs::OpenOptions;
    use std::io::Write;

    let mut file = OpenOptions::new()
        .write(true)
        .append(true)
        .create(true)
        .open(path)
        .expect("cannot open file");

    file.write_all(serde_json::to_string(&block).unwrap().as_bytes())
        .expect("write failed");
    file.write_all(b"\n").expect("write failed");
}
