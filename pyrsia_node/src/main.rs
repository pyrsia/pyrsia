extern crate bytes;
extern crate clap;
extern crate easy_hasher;
extern crate log;
extern crate pretty_env_logger;
extern crate serde;
extern crate tokio;
extern crate uuid;
extern crate warp;
#[macro_use]
extern crate lazy_static;
//local module imports
mod artifact_manager;
mod docker;
mod document_store;
mod network;
mod node_manager;
mod utils;

use docker::error_util::*;
use document_store::document_store::DocumentStore;
use network::swarm::{new as new_swarm, MyBehaviourSwarm};
use network::transport::{new_tokio_tcp_transport, TcpTokioTransport};

use clap::{App, Arg, ArgMatches};
use futures::StreamExt;
use identity::Keypair;
use libp2p::{
    floodsub::{self, Topic},
    identity,
    swarm::SwarmEvent,
    Multiaddr, PeerId,
};
use log::{debug, error, info};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use tokio::io::{self, AsyncBufReadExt};
use warp::Filter;
use std::sync::Arc;
use tokio::sync::mpsc;
use tokio::sync::Mutex;
use crate::docker::v2::routes::*;
use crate::node_api::routes::*;

const DEFAULT_PORT: &str = "7879";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // create the connection to the documentStore.
    let doc_store = DocumentStore::new();
    doc_store.ping();
    // Create a random PeerId
    let id_keys: Keypair = identity::Keypair::generate_ed25519();
    let peer_id: PeerId = PeerId::from(id_keys.public());

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

    let transport: TcpTokioTransport = new_tokio_tcp_transport(&id_keys); // Create a tokio-based TCP transport using noise for authenticated
    let floodsub_topic: Topic = floodsub::Topic::new("pyrsia-node-converstation"); // Create a Floodsub topic

    // Create a Swarm to manage peers and events.
    let mut swarm: MyBehaviourSwarm = new_swarm(floodsub_topic.clone(), transport, peer_id)
        .await
        .unwrap();

    // Reach out to another node if specified
    if let Some(to_dial) = matches.value_of("peer") {
        let addr: Multiaddr = to_dial.parse().unwrap();
        swarm.dial_addr(addr).unwrap();
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

    //docker node specific tx
    let tx1 = tx.clone();

    //swarm specific tx,rx
    // need better handling of all these channel resources
    let shared_stats = Arc::new(Mutex::new(respond_rx));
    let my_stats = shared_stats.clone();
    let tx2 = tx.clone();

    let my_stats1 = shared_stats.clone();
    let tx3 = tx.clone();

    let docker_routes = get_docker_routes(tx1);
    let routes = docker_routes.or(get_node_routes(tx2, my_stats, tx3, my_stats1));

    let (addr, server) = warp::serve(
        routes
            .and(utils::log::log_headers())
            .recover(custom_recover)
            .with(warp::log("pyrsia_registry")),
    )
    .bind_ephemeral(address);

    info!("Pyrsia Docker Node is now running on port {}!", addr.port());

    tokio::spawn(server);
    let tx2 = tx.clone();

    // Kick it off
    loop {
        tokio::select! {
            line = stdin.next_line() => {
                let line = line.unwrap().expect("stdin closed");
                debug!("next line!");
                match tx2.send(line).await {
                    Ok(_) => debug!("line sent"),
                    Err(_) => error!("failed to send stdin input")
                }
            }
            event = swarm.select_next_some() => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    info!("Listening on {:?}", address);
                }
            }
            Some(message) = rx.recv() => {
                info!("New message: {}", message);
                swarm.behaviour_mut().floodsub_mut().publish(floodsub_topic.clone(), message.as_bytes());
                swarm.behaviour_mut().lookup_blob(message).unwrap();
            }
        }
    }
}
