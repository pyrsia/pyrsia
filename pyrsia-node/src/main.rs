extern crate async_std;
extern crate clap;
extern crate log;
extern crate pretty_env_logger;
extern crate tokio;
extern crate warp;

use async_std::task;
use std::convert::Infallible;
use std::fs;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use clap::{App, Arg, ArgMatches};
use futures::StreamExt;
use libp2p::core::muxing::StreamMuxerBox;
use libp2p::core::transport::Boxed;
use libp2p::kad::record::store::MemoryStore;
use libp2p::kad::{GetClosestPeersError, Kademlia, KademliaConfig, KademliaEvent, QueryResult};
use libp2p::{
    development_transport,
    identity::Keypair,
    swarm::{Swarm, SwarmEvent},
    Multiaddr, PeerId,
};
use log::{debug, info};
use std::{env, str::FromStr, time::Duration};
use warp::http::HeaderMap;
use warp::Filter;

const DEFAULT_PORT: &str = "7878";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let boot_nodes: Vec<&'static str> = vec![
        "QmYiFu1AiWLu3Me73kVrE7z1fmcjjHVwJ7GZRZoNcTLjRF",
        "QmNnooDu7bfjPFoTZYxMNLWUQJyrVwtbZg5gBMjTezGAJN",
        "QmQCU2EcMqAqQPR2i9bChDtGNJchTbq5TbXJJ16u19uLTa",
        "QmbLHAnMoJPWSCR5Zhtx6BHJX9KiKNN6tpvbUcqanj75Nb",
        "QmcZf59bWwK5XFi76CZX8cbJ4BhTzzA3gU1ZjYZcYW3dwt",
        "QmTS3CRTSVsYXzD1nb1yD2N6XhWSWApZnpaxtCn8vLKHmd",
        "QmQUZCnMVJSQkBsSQba6dvijsLnesqhvB532XeTp1GDhxa",
    ];

    // Create a random key for ourselves.
    let local_key: Keypair = Keypair::generate_ed25519();
    let local_peer_id: PeerId = PeerId::from(local_key.public());

    // Set up a an encrypted DNS-enabled TCP Transport over the Mplex protocol
    let transport: Boxed<(PeerId, StreamMuxerBox)> =
        development_transport(local_key).await.unwrap();

    let matches: ArgMatches = App::new("Pyrsia Node")
        .version("0.1.0")
        .author(clap::crate_authors!(", "))
        .about("Application to connect to and participate in the Pyrsia network")
        .arg(
            Arg::with_name("verbose")
                .short("v")
                .long("verbose")
                .takes_value(false)
                .required(false)
                .multiple(true)
                .help("Enables verbose output"),
        )
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

    let verbosity: u64 = matches.occurrences_of("verbose");
    if verbosity > 0 {
        dbg!("Verbosity Level: {}", verbosity.to_string());
    }

    // Create a swarm to manage peers and events.
    let mut swarm: Swarm<Kademlia<MemoryStore>> = {
        // Create a Kademlia behaviour.
        let mut cfg: KademliaConfig = KademliaConfig::default();
        cfg.set_query_timeout(Duration::from_secs(5 * 60));
        let store: MemoryStore = MemoryStore::new(local_peer_id);
        let mut behaviour: Kademlia<MemoryStore> = Kademlia::with_config(local_peer_id, store, cfg);

        // Add the bootnodes to the local routing table. `libp2p-dns` built
        // into the `transport` resolves the `dnsaddr` when Kademlia tries
        // to dial these nodes.
        let bootaddr: Multiaddr = Multiaddr::from_str("/dnsaddr/bootstrap.libp2p.io").unwrap();
        for peer in &boot_nodes {
            behaviour.add_address(&PeerId::from_str(peer).unwrap(), bootaddr.clone());
        }

        Swarm::new(transport, behaviour, local_peer_id)
    };
    dbg!("swarm");

    let to_search: PeerId;
    // Order Kademlia to search for a peer.
    if matches.occurrences_of("peer") > 0 {
        let peer_id: String = String::from(matches.value_of("peer").unwrap());
        to_search = PeerId::from_str(&peer_id).unwrap();
    } else {
        to_search = Keypair::generate_ed25519().public().into();
    }

    println!("Searching for the closest peers to {:?}", to_search);
    swarm.behaviour_mut().get_closest_peers(to_search);

    // Kick it off!
    task::block_on(async move {
        loop {
            let event: SwarmEvent<KademliaEvent, std::io::Error> = swarm.select_next_some().await;
            if let SwarmEvent::Behaviour(KademliaEvent::OutboundQueryCompleted {
                result: QueryResult::GetClosestPeers(result),
                ..
            }) = event
            {
                match result {
                    Ok(ok) => {
                        if !ok.peers.is_empty() {
                            println!("Query finished with closest peers: {:#?}", ok.peers)
                        } else {
                            // The example is considered failed as there
                            // should always be at least 1 reachable peer.
                            println!("Query finished with no closest peers.")
                        }
                    }
                    Err(GetClosestPeersError::Timeout { peers, .. }) => {
                        if !peers.is_empty() {
                            println!("Query timed out with closest peers: {:#?}", peers)
                        } else {
                            // The example is considered failed as there
                            // should always be at least 1 reachable peer.
                            println!("Query timed out with no closest peers.");
                        }
                    }
                };

                break;
            }
        }
    });

    let mut address = SocketAddr::new(IpAddr::V4(Ipv4Addr::new(127, 0, 0, 1)), 8080);
    if let Some(p) = matches.value_of("port") {
        address.set_port(p.parse::<u16>().unwrap());
    }

    info!("Pyrsia Node is now running on port {}!", address.port());

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

    let v2_blobs = warp::path!("v2" / String / "blobs" / String)
        .and(warp::get().or(warp::head()).unify())
        .and_then(handle_get_blobs);

    let routes = warp::any()
        .and(log_headers())
        .and(v2_base.or(v2_manifests).or(v2_blobs))
        .with(warp::log("pyrsia_registry"));

    warp::serve(routes).run(address).await;
}

async fn handle_get_manifests(
    name: String,
    tag: String,
) -> Result<impl warp::Reply, warp::Rejection> {
    let colon = tag.find(':');
    let mut hash = String::from(&tag);
    if colon == None {
        let manifest = format!(
            "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current/link",
            name, tag
        );
        let manifest_content = fs::read_to_string(manifest);
        if manifest_content.is_err() {
            // todo: generate error response as specified in https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
            return Err(warp::reject::not_found());
        }
        hash = manifest_content.unwrap();
    }

    let blob = format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data",
        hash.get(7..9).unwrap(),
        hash.get(7..).unwrap()
    );
    let blob_content = fs::read_to_string(blob);
    if blob_content.is_err() {
        // todo: generate error response as specified in https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
        return Err(warp::reject::not_found());
    }

    let content = blob_content.unwrap();
    return Ok(warp::http::response::Builder::new()
        .header(
            "Content-Type",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .header("Content-Length", content.len())
        .status(200)
        .body(content)
        .unwrap());
}

async fn handle_get_blobs(
    _name: String,
    hash: String,
) -> Result<impl warp::Reply, warp::Rejection> {
    let blob = format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data",
        hash.get(7..9).unwrap(),
        hash.get(7..).unwrap()
    );
    let blob_content = fs::read(blob);
    if blob_content.is_err() {
        // todo: generate error response as specified in https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
        return Err(warp::reject::not_found());
    }

    let content = blob_content.unwrap();
    return Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", content.len())
        .status(200)
        .body(content)
        .unwrap());
}

fn log_headers() -> impl Filter<Extract = (), Error = Infallible> + Copy {
    warp::header::headers_cloned()
        .map(|headers: HeaderMap| {
            for (k, v) in headers.iter() {
                // Error from `to_str` should be handled properly
                debug!(target: "pyrsia_registry", "{}: {}", k, v.to_str().expect("Failed to print header value"))
            }
        })
        .untuple_one()
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
}
