mod artifact_manager;

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
mod docker;
mod network;
mod utils;

use docker::error_util::*;
use docker::v2::handlers::blobs::*;
use docker::v2::handlers::manifests::*;
use network::swarm::{new as new_swarm, MyBehaviourSwarm};
use network::transport::{new_tokio_tcp_transport, TcpTokioTransport};
use std::path::Path;
use warp::Rejection;
use warp::Reply;

use identity::Keypair;
use std::collections::HashMap;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use clap::{App, Arg, ArgMatches};
use futures::StreamExt;
use libp2p::{
    floodsub::{self, Topic},
    identity,
    swarm::SwarmEvent,
    Multiaddr, PeerId,
};
use log::{debug, info};
use std::env;
use tokio::io::{self, AsyncBufReadExt};
use warp::Filter;

const DEFAULT_PORT: &str = "7878";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

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

    let empty_json = "{}";
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

    let pending_get_providers: Vec<libp2p::kad::QueryId> = Default::default();

    let handle_get_blobs_with_fallback =
        |name: String, hash: String| -> Result<impl Reply, Rejection> {
            let blob = format!(
                "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data",
                hash.get(7..9).unwrap(),
                hash.get(7..).unwrap()
            );

            debug!("Searching for blob: {}", blob);
            let blob_path = Path::new(&blob);
            if !blob_path.exists() {
                let query: libp2p::kad::QueryId =
                    swarm.behaviour_mut().lookup_blob(hash.to_string()).unwrap();
                pending_get_providers.push(query);
            }

            Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::BlobDoesNotExist(hash),
            }))
        };

    let v2_blobs = warp::path!("v2" / String / "blobs" / String)
        .and(warp::get().or(warp::head()).unify())
        .and(warp::path::end())
        .and_then(handle_get_blobs_with_fallback);
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
        tokio::select! {
            line = stdin.next_line() => {
                let line = line.unwrap().expect("stdin closed");
                swarm.behaviour_mut().floodsub().publish(floodsub_topic.clone(), line.as_bytes());
            }
            event = swarm.select_next_some().await => {
                if let SwarmEvent::NewListenAddr { address, .. } = event {
                    info!("Listening on {:?}", address);
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    //use super::*;
}
