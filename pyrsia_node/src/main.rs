extern crate bytes;
extern crate clap;
extern crate easy_hasher;
extern crate log;
extern crate pretty_env_logger;
extern crate serde;
extern crate tokio;
extern crate uuid;
extern crate warp;

//local module imports
mod artifact_manager;
mod docker;
mod document_store;
mod network;
mod utils;

use libp2p::{
    swarm:: Swarm,
};

use log::{error, info};
use once_cell::sync::Lazy;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use tokio::{fs, io::AsyncBufReadExt, sync::mpsc};

use crate::network::behavior::MyBehaviour;

use docker::error_util::*;

use docker::v2::handlers::blobs::*;
use docker::v2::handlers::manifests::*;
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
use std::collections::HashMap;
use std::env;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use warp::Filter;


type Result<T> = std::result::Result<T, Box<dyn std::error::Error + Send + Sync + 'static>>;

const DEFAULT_PORT: &str = "8082";
static ID_KEYS: Lazy<identity::Keypair> = Lazy::new(|| identity::Keypair::generate_ed25519());
static PEER_ID: Lazy<PeerId> = Lazy::new(|| PeerId::from(ID_KEYS.public()));
static TOPIC: Lazy<Topic> = Lazy::new(|| Topic::new("pyrsia-node-converstation"));


#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    // create the connection to the documentStore.
    let doc_store = DocumentStore::new();
    doc_store.ping();
    // Create a random PeerId
    let id_keys: Keypair = identity::Keypair::generate_ed25519();
    let peer_id: PeerId = PeerId::from(id_keys.public());
    info!("pyrsia Peer id is {}:",peer_id);

    let (response_sender, mut response_rcv) = tokio::sync::mpsc::unbounded_channel();


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
                .help("Sets the port to listen to"),
        )
        .arg(
            Arg::with_name("peer")
                //.short("p")
                .long("peer")
                .takes_value(true)
                .required(false)
                .multiple(false)
                .help("Provide an explicit peerId"),
        )
        .get_matches();

    let transport: TcpTokioTransport = new_tokio_tcp_transport(&id_keys); // Create a tokio-based TCP transport using noise for authenticated
    //let floodsub_topic: Topic = floodsub::Topic::newTOPIC.clone()); // Create a Floodsub topic


    // Create a Swarm to manage peers and events.
    let mut swarm_inst: MyBehaviourSwarm = new_swarm(TOPIC.clone(), transport, peer_id,response_sender)
        .await
        .unwrap();

    // Reach out to another node if specified
    if let Some(to_dial) = matches.value_of("peer") {
        let addr: Multiaddr = to_dial.parse().unwrap();
        swarm_inst.dial_addr(addr).unwrap();
        info!("pyrsia Dialed {:?}", to_dial)
    }

    // Read full lines from stdin
    let mut stdin = tokio::io::BufReader::new(tokio::io::stdin()).lines();

    // Listen on all interfaces and whatever port the OS assigns
    swarm_inst
        .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .unwrap();

    let mut address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 0);
    if let Some(p) = matches.value_of("port") {
        address.set_port(p.parse::<u16>().unwrap());
    }

    let empty_json = "{i am alive}";
    let v2_base = warp::path("v2")
        .and(warp::get())
        .and(warp::path::end())
        .map(move || empty_json)
        .with(warp::reply::with::header(
            "Content-Length",
            empty_json.len(),
        ))
        .with(warp::reply::with::header(
            "Content-Type",
            "application/json",
        ));

    let v2_manifests = warp::path!("v2" / String / "manifests" / String)
        .and(warp::get().or(warp::head()).unify())
        .and_then(handle_get_manifests);
    let v2_manifests_put_docker = warp::path!("v2" / String / "manifests" / String)
        .and(warp::put())
        .and(warp::header::exact(
            "Content-Type",
            "application/vnd.docker.distribution.manifest.v2+json",
        ))
        .and(warp::body::bytes())
        .and_then(handle_put_manifest);

    let v2_blobs = warp::path!("v2" / String / "blobs" / String)
        .and(warp::get().or(warp::head()).unify())
        .and(warp::path::end())
        .and_then(handle_get_blobs);
    let v2_blobs_post = warp::path!("v2" / String / "blobs" / "uploads")
        .and(warp::post())
        .and_then(handle_post_blob);
    let v2_blobs_patch = warp::path!("v2" / String / "blobs" / "uploads" / String)
        .and(warp::patch())
        .and(warp::body::bytes())
        .and_then(handle_patch_blob);
    let v2_blobs_put = warp::path!("v2" / String / "blobs" / "uploads" / String)
        .and(warp::put())
        .and(warp::query::<HashMap<String, String>>())
        .and(warp::body::bytes())
        .and_then(handle_put_blob);

    let routes = warp::any()
        .and(utils::log::log_headers())
        .and(
            v2_base
                .or(v2_manifests)
                .or(v2_manifests_put_docker)
                .or(v2_blobs)
                .or(v2_blobs_post)
                .or(v2_blobs_patch)
                .or(v2_blobs_put),
        )
        .recover(custom_recover)
        .with(warp::log("pyrsia_registry"));
    let (addr, server) = warp::serve(routes).bind_ephemeral(address);
    info!("Pyrsia Docker Node is now running on port {}!", addr.port());

    tokio::task::spawn(server);

    // Kick it off
    loop {
        let evt = {
            tokio::select! {
                line = stdin.next_line() => Some(EventType::Input(line.expect("can get line").expect("can read line from stdin"))),
                response = response_rcv.recv() => Some(EventType::Response(response.expect("response exists"))),
                event = swarm_inst.select_next_some() => {
                    if let SwarmEvent::NewListenAddr { address, .. } = event {
                        info!("Listening on {:?}", address);
                    }

                    //info!("Swarm Event: {:?}", event);
                    None
                },
            }
        };

        if let Some(event) = evt {
            match event {
                EventType::Response(resp) => {
                    let json = serde_json::to_string(&resp).expect("can jsonify response");
                    swarm_inst
                        .behaviour_mut()
                        .floodsub
                        .publish(TOPIC.clone(), json.as_bytes());
                }
                EventType::Input(line) => match line.as_str() {
                    "ls p" => handle_list_peers(&mut swarm_inst).await,
                    _ => error!("unknown command"),
                },
            }
        }
    }

}




#[derive(Debug, Serialize, Deserialize)]
enum ListMode {
    ALL,
    One(String),
}

#[derive(Debug, Serialize, Deserialize)]
struct ListRequest {
    mode: ListMode,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ListResponse {
    mode: ListMode,
    receiver: String,
}

enum EventType {
    Response(ListResponse),
    Input(String),
}


async fn handle_list_peers(swarm: &mut Swarm<MyBehaviour>) {
    info!("Discovered Peers:");
    let nodes = swarm.behaviour().mdns.discovered_nodes();
    let mut unique_peers = HashSet::new();
    for peer in nodes {
        unique_peers.insert(peer);
    }
    unique_peers.iter().for_each(|p| info!("{}", p));
}





#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    //use super::*;
}
