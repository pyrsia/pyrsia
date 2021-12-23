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

extern crate bytes;
extern crate clap;
extern crate easy_hasher;
extern crate lazy_static;
extern crate log;
extern crate pretty_env_logger;
extern crate serde;
extern crate tokio;
extern crate uuid;
extern crate warp;
<<<<<<< HEAD
#[macro_use]
extern crate lazy_static;

=======
>>>>>>> upstream/main
//local module imports
mod artifact_manager;
mod block_chain;
mod docker;
mod document_store;
mod logging;
mod metadata_manager;
mod network;
mod node_api;
mod node_manager;
<<<<<<< HEAD

=======
mod utils;

use block_chain::block::Block;
use block_chain::block_chain::BlockChain;
>>>>>>> upstream/main
use docker::error_util::*;
use document_store::document_store::DocumentStore;
use document_store::document_store::IndexSpec;
use network::swarm::{new as new_swarm, MyBehaviourSwarm};
use network::transport::{new_tokio_tcp_transport, TcpTokioTransport};

use clap::{App, Arg, ArgMatches};
use futures::StreamExt;
use libp2p::{
    floodsub::{self, Topic},
    identity,
    swarm::SwarmEvent,
    Multiaddr, PeerId,
};
use log::{debug, error, info};
use tokio::sync::mpsc;

use crate::docker::v2::routes::*;
use crate::node_api::routes::*;
use std::sync::Arc;
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tokio::io::{self, AsyncBufReadExt};
use tokio::sync::Mutex;
use warp::Filter;

const DEFAULT_PORT: &str = "7878";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // create the connection to the documentStore.
    let index_one = "index_one";
    let field1 = "mostSignificantField";
    let idx1 = IndexSpec::new(index_one, vec![field1]);

    DocumentStore::create("document_store", vec![idx1]).expect("Failed to create DocumentStore");
    let doc_store = DocumentStore::get("document_store").unwrap();
    doc_store.ping();

    let matches: ArgMatches = App::new("Pyrsia Node")
        .version("0.1.0")
        .author(clap::crate_authors!(", "))
        .about("Application to connect to and participate in the Pyrsia network")
        .arg(
            Arg::with_name("port")
                .short("p")
                .long("port")
                .value_name("PORT")
                .default_value(DEFAULT_PORT)
                .takes_value(true)
                .required(false)
                .multiple(false)
                .help("Sets the port to listen to for the Docker API"),
        )
        .arg(
            Arg::with_name("peer")
                //.short("p")
                .long("peer")
                .takes_value(true)
                .required(false)
                .multiple(false)
                .help("Provide an explicit peerId to connect with"),
        )
        .get_matches();

    let local_key = identity::Keypair::generate_ed25519();

    let local_peer_id = PeerId::from(local_key.public());

    let transport: TcpTokioTransport = new_tokio_tcp_transport(&local_key); // Create a tokio-based TCP transport using noise for authenticated
                                                                            //let floodsub_topic: Topic = floodsub::Topic::new("pyrsia-node-converstation"); // Create a Floodsub topic
    let (respond_tx, respond_rx) = mpsc::channel(32);
    let floodsub_topic: Topic = floodsub::Topic::new("pyrsia-node-converstation");
    // Create a Swarm to manage peers and events.
    let mut swarm: MyBehaviourSwarm =
        new_swarm(floodsub_topic.clone(), transport, local_peer_id, respond_tx)
            .await
            .unwrap();

    // Reach out to another node if specified
    if let Some(to_dial) = matches.value_of("peer") {
        let addr: Multiaddr = to_dial.parse().unwrap();
        swarm.dial(addr).unwrap();
        info!("Dialed {:?}", to_dial)
    }

    // Read full lines from stdin
    let mut stdin = io::BufReader::new(io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

    let mut address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    if let Some(p) = matches.value_of("port") {
        address.set_port(p.parse::<u16>().unwrap());
    }

    let (tx, mut rx) = mpsc::channel(32);

    let (upload_tx, mut upload_rx) = tokio::sync::mpsc::channel(32);
    let utx1 = upload_tx.clone();
    let mut blobs_need_hash = GetBlobsHandle::new();
    let b1 = blobs_need_hash.clone();
    //docker node specific tx
    let tx1 = tx.clone();

    //swarm specific tx,rx
    // need better handling of all these channel resources
    let shared_stats = Arc::new(Mutex::new(respond_rx));

    let my_stats = shared_stats.clone();
    let tx2 = tx.clone();

    let my_stats1 = shared_stats.clone();
    let tx3 = tx.clone();

    let docker_routes = make_docker_routes(tx1);
    let routes = docker_routes.or(make_node_routes(tx2, my_stats, tx3, my_stats1));

    let (addr, server) = warp::serve(
        routes
            .and(utils::log::log_headers())
            .recover(custom_recover)
            .with(warp::log("pyrsia_registry")),
    )
    .bind_ephemeral(address);

    info!("Pyrsia Docker Node is now running on port {}!", addr.port());

    tokio::spawn(server);
    let tx4 = tx.clone();

    let mut bc = BlockChain::new();
    bc.genesis();
    // Kick it off
    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                message = rx.recv() => Some(EventType::Message(message.expect("message exists"))),

                // TODO(prince-chrismc): Merge Conflict -- Test
                new_hash = blobs_need_hash.select_next_some() => Some(EventType::Response(new_hash)),

                event = swarm.select_next_some() =>  {
                    if let SwarmEvent::NewListenAddr { address, .. } = event {
                        info!("Listening on {:?}", address);
                    }

                    //SwarmEvent::Behaviour(e) => panic!("Unexpected event: {:?}", e),
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                EventType::Response(resp) => {
                    //here we have to manage which events to publish to floodsub
                    swarm
                        .behaviour_mut()
                        .floodsub_mut()
                        .publish(floodsub_topic.clone(), resp.as_bytes());
                }
                EventType::Input(line) => match line.as_str() {
                    "peers" => swarm.behaviour_mut().list_peers_cmd().await,
                    _ => match tx4.send(line).await {
                        Ok(_) => debug!("line sent"),
                        Err(_) => error!("failed to send stdin input"),
                    },
                },
                EventType::Message(message) => match message.as_str() {
                    cmd if cmd.starts_with("peers") || cmd.starts_with("status") => {
                        swarm.behaviour_mut().list_peers(local_peer_id).await
                    }
                    cmd if cmd.starts_with("get_blobs") => {
                        swarm.behaviour_mut().lookup_blob(message).await
                    }
                    "block" => {
                        // assuming the message is a json version of the block

                        let block = Block {
                            id: 0,
                            hash: "".to_string(),
                            previous_hash: "".to_string(),
                            timestamp: 0,
                            data: "".to_string(),
                            nonce: 0,
                        };
                        bc.add_block(block);
                    }
                    _ => info!("message received from peers: {}", message),
                },
            }
        }
    }
}

enum EventType {
    Response(String),
    Message(String),
    Input(String),
}
