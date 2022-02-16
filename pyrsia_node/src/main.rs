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

use pyrsia::docker::error_util::*;
use pyrsia::docker::v2::handlers::blobs::GetBlobsHandle;
use pyrsia::docker::v2::routes::*;
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
use log::{debug, info};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    sync::Arc,
};
use tokio::{
    sync::{mpsc, Mutex},
};
use warp::Filter;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "7888";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // Initiate the document store with it's first document
    let index_one = "index_one";
    let field1 = "most_significant_field";
    // The actual first index is not necessary here, it is preserved in the documentStore
    IndexSpec::new(index_one, vec![field1]);

    let matches: ArgMatches = App::new("Pyrsia Node")
        .version("0.1.0")
        .author(clap::crate_authors!(", "))
        .about("Application to connect to and participate in the Pyrsia network")
        .arg(
            Arg::new("host")
                .short('H')
                .long("host")
                .value_name("HOST")
                .default_value(DEFAULT_HOST)
                .takes_value(true)
                .required(false)
                .multiple_occurrences(false)
                .help("Sets the host address to bind to for the Docker API"),
        )
        .arg(
            Arg::new("port")
                .short('p')
                .long("port")
                .value_name("PORT")
                .default_value(DEFAULT_PORT)
                .takes_value(true)
                .required(false)
                .multiple_occurrences(false)
                .help("Sets the port to listen to for the Docker API"),
        )
        .arg(
            Arg::new("peer")
                //.short("p")
                .long("peer")
                .takes_value(true)
                .required(false)
                .multiple_occurrences(false)
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
        &local_key,
        local_peer_id,
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

    // Listen on all interfaces and whatever port the OS assigns
    swarm
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

    // Get host and port from the settings. Defaults to DEFAULT_HOST and DEFAULT_PORT
    let host = matches.value_of("host").unwrap();
    let port = matches.value_of("port").unwrap();
    debug!(
        "Pyrsia Docker Node will bind to host = {}, port = {}",
        host, port
    );

    let address = SocketAddr::new(
        IpAddr::V4(host.parse::<Ipv4Addr>().unwrap()),
        port.parse::<u16>().unwrap(),
    );

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

    let docker_routes = make_docker_routes(b1, tx1);
    let routes = docker_routes.or(make_node_routes(tx2, my_stats));

    let (addr, server) = warp::serve(
        routes
            .and(http::log_headers())
            .recover(custom_recover)
            .with(warp::log("pyrsia_registry")),
    )
    .bind_ephemeral(address);

    info!(
        "Pyrsia Docker Node is now running on port {}:{}!",
        addr.ip(),
        addr.port()
    );

    tokio::spawn(server);

    // Kick it off
    loop {
        let evt = {
            tokio::select! {
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

            }
        };

        if let Some(event) = evt {
            match event {
                EventType::Message(message) => match message.as_str() {
                    cmd if cmd.starts_with("peers") || cmd.starts_with("status") => {
                        swarm.behaviour_mut().list_peers(local_peer_id).await
                    }
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
                    cmd if cmd.starts_with("get_blobs") => {
                        swarm.behaviour_mut().lookup_blob(message).await
                    }
                    _ => info!("message received from peers: {}", message),
                },
            }
        }
    }
}

enum EventType {
    Message(String),
}