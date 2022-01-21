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

extern crate clap;
extern crate futures;
extern crate libp2p;
extern crate log;
extern crate pretty_env_logger;
extern crate pyrsia;
extern crate tokio;
extern crate warp;

use pyrsia::block_chain::block_chain::Ledger;
use pyrsia::block_chain::*;
use pyrsia::docker::error_util::*;
use pyrsia::docker::v2::handlers::blobs::GetBlobsHandle;
use pyrsia::docker::v2::routes::*;
use pyrsia::document_store::document_store::DocumentStore;
use pyrsia::document_store::document_store::IndexSpec;
use pyrsia::logging::*;
use pyrsia::network::swarm::{self, MyBehaviourSwarm};
use pyrsia::network::transport::{new_tokio_tcp_transport, TcpTokioTransport};
use pyrsia::node_api::routes::make_node_routes;

use clap::{App, Arg, ArgMatches};
use futures::StreamExt;
use libp2p::{
    core::identity,
    floodsub::{self, Topic},
    swarm::SwarmEvent,
    Multiaddr, PeerId,
};
use log::{debug, error, info};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{
    io::{self, AsyncBufReadExt},
    sync::{mpsc, Mutex, MutexGuard},
};
use warp::Filter;

const DEFAULT_PORT: &str = "7878";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // create the connection to the documentStore.
    let index_one = "index_one";
    let field1 = "most_significant_field";
    let idx1 = IndexSpec::new(index_one, vec![field1]);

    DocumentStore::create("document_store", vec![idx1]).expect("Failed to create DocumentStore");
    let doc_store = DocumentStore::get("document_store").unwrap();
    doc_store.ping();

    let matches: ArgMatches = App::new("Pyrsia Node")
        .version("0.1.0")
        .author(clap::crate_authors!(", "))
        .about("Application to connect to and participate in the Pyrsia network")
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .default_value(DEFAULT_PORT)
                .takes_value(true)
                .required(false)
                .multiple(false)
                .help("Sets the port to listen to for the Docker API"),
        )
        .arg(
            Arg::new("peer")
                //.short("p")
                .long("peer")
                .takes_value(true)
                .required(false)
                .multiple(false)
                .help("Provide an explicit peerId to connect with"),
        )
        .get_matches();

    let local_key: identity::Keypair = identity::Keypair::generate_ed25519();
    let local_peer_id = PeerId::from(local_key.public());
    let transport: TcpTokioTransport = new_tokio_tcp_transport(&local_key); // Create a tokio-based TCP transport using noise for authenticated

    let (respond_tx, respond_rx) = mpsc::channel(32);
    let floodsub_topic: Topic = floodsub::Topic::new("pyrsia_node_converstation");
    let gossip_topic: libp2p::gossipsub::IdentTopic =
        libp2p::gossipsub::IdentTopic::new("pyrsia_file_share_topic");

    // Create a Swarm to manage peers and events.
    let mut swarm: MyBehaviourSwarm = swarm::new(
        gossip_topic.clone(),
        floodsub_topic.clone(),
        transport,
        local_key,
        respond_tx,
    )
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

    let mut blobs_need_hash = GetBlobsHandle::new();
    let b1 = blobs_need_hash.clone();
    //docker node specific tx
    let tx1 = tx.clone();

    // swarm specific tx,rx
    // need better handling of all these channel resources
    let shared_stats = Arc::new(Mutex::new(respond_rx));

    let my_stats = shared_stats.clone();
    let tx2 = tx.clone();

    // We need to have two channels (to seperate the handling)
    // 1. API to main
    // 2. main to API
    let (blocks_get_tx_to_main, mut blocks_get_rx_from_api) = mpsc::channel(32); // Request Channel
    let (blocks_get_tx_answer_to_api, blocks_get_rx_answers_from_main) = mpsc::channel(32); // Response Channel

    let docker_routes = make_docker_routes(b1, tx1);
    let routes = docker_routes.or(make_node_routes(
        tx2,
        my_stats,
        blocks_get_tx_to_main.clone(),
        blocks_get_rx_answers_from_main,
    ));

    let (addr, server) = warp::serve(
        routes
            .and(http::log_headers())
            .recover(custom_recover)
            .with(warp::log("pyrsia_registry")),
    )
    .bind_ephemeral(address);

    info!("Pyrsia Docker Node is now running on port {}!", addr.port());

    tokio::spawn(server);
    let tx4 = tx.clone();

    let raw_chain = block_chain::BlockChain::new();
    let bc = Arc::new(Mutex::new(raw_chain));
    // Kick it off
    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                message = rx.recv() => Some(EventType::Message(message.expect("message exists"))),

                new_hash = blobs_need_hash.select_next_some() => {
                    debug!("Looking for {}", new_hash);
                    swarm.behaviour_mut().lookup_blob(new_hash).await;
                    None
                },

                event = swarm.select_next_some() =>  {
                    if let SwarmEvent::NewListenAddr { address, .. } = event {
                        info!("Listening on {:?}", address);
                    }

                    //SwarmEvent::Behaviour(e) => panic!("Unexpected event: {:?}", e),
                    None
                },
                get_blocks_request_input = blocks_get_rx_from_api.recv() => //BlockChain::handle_api_requests(),

                // Channels are only one type (both tx and rx must match)
                // We need to have two channel for each API call to send and receive
                // The request and the response.

                {
                    info!("Processessing 'GET /blocks' {} request", get_blocks_request_input.unwrap());
                    let bc1 = bc.clone();
                    let block_chaing_instance = bc1.lock().await.clone();
                    blocks_get_tx_answer_to_api.send(block_chaing_instance).await.expect("send to work");

                    None
                }
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
                    cmd if cmd.starts_with("magnet:") => {
                        info!(
                            "{}",
                            swarm
                                .behaviour_mut()
                                .gossipsub_mut()
                                .publish(gossip_topic.clone(), cmd)
                                .unwrap()
                        )
                    }
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
                    "blocks" => {
                        let bc_state: Arc<_> = bc.clone();
                        let mut bc_instance: MutexGuard<_> = bc_state.lock().await;
                        let new_block =
                            bc_instance.mk_block("happy_new_block".to_string()).unwrap();

                        let new_chain = bc_instance
                            .clone()
                            .add_entry(new_block)
                            .expect("should have added");
                        *bc_instance = new_chain;
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
