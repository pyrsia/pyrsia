extern crate async_std;
extern crate bytes;
extern crate clap;
extern crate easy_hasher;
extern crate log;
extern crate pretty_env_logger;
extern crate serde;
extern crate tokio;
extern crate uuid;
extern crate warp;

mod blockchain_manager;

use async_std::task;
use std::collections::HashMap;
use std::convert::Infallible;
use std::fmt;
use std::fs;
use std::io::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::Path;

use bytes::{Buf, Bytes};
use clap::{App, Arg, ArgMatches};
use easy_hasher::easy_hasher::{file_hash, raw_sha256, Hash};
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
use log::{debug, error, info};
use serde::{Deserialize, Serialize};
use std::{env, str::FromStr, time::Duration};
use uuid::Uuid;
use warp::http::{HeaderMap, StatusCode};
use warp::reject::Reject;
use warp::{Filter, Rejection, Reply};

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
        .and(log_headers())
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
    warp::serve(routes).run(address).await;
}

#[derive(Debug, Deserialize, Serialize)]
enum RegistryErrorCode {
    BlobUnknown,
    ManifestUnknown,
    Unknown(String),
}

impl fmt::Display for RegistryErrorCode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let printable = match &self {
            RegistryErrorCode::BlobUnknown => "BLOB_UNKNOWN".to_string(),
            RegistryErrorCode::ManifestUnknown => "MANIFEST_UNKNOWN".to_string(),
            RegistryErrorCode::Unknown(m) => format!("UNKNOWN({})", m),
        };
        write!(f, "{}", printable)
    }
}

#[derive(Debug)]
struct RegistryError {
    code: RegistryErrorCode,
}

impl Reject for RegistryError {}

async fn handle_get_manifests(name: String, tag: String) -> Result<impl Reply, Rejection> {
    let colon = tag.find(':');
    let mut hash = String::from(&tag);
    if colon == None {
        let manifest = format!(
            "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current/link",
            name, tag
        );
        let manifest_content = fs::read_to_string(manifest);
        if manifest_content.is_err() {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::ManifestUnknown,
            }));
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
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::ManifestUnknown,
        }));
    }

    let content = blob_content.unwrap();
    return Ok(warp::http::response::Builder::new()
        .header(
            "Content-Type",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .header("Content-Length", content.len())
        .status(StatusCode::OK)
        .body(content)
        .unwrap());
}

async fn handle_put_manifest(
    name: String,
    reference: String,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    let id = Uuid::new_v4();

    // temporary upload of manifest
    let blob_upload_dest_dir = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    );
    if let Err(e) = fs::create_dir_all(&blob_upload_dest_dir) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    let mut blob_upload_dest = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data",
        name, id
    );
    let append = append_to_blob(&mut blob_upload_dest, bytes);
    if let Err(e) = append {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    } else {
        // calculate sha256 checksum on manifest file
        let file256 = file_hash(raw_sha256, &blob_upload_dest);
        let digest: Hash;
        match file256 {
            Ok(hash) => digest = hash,
            Err(e) => {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }))
            }
        }

        let hash = digest.to_hex_string();
        debug!(
            "Generated hash for manifest {}/{}: {}",
            name, reference, hash
        );
        let mut blob_dest = format!(
            "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}",
            hash.get(0..2).unwrap(),
            hash
        );
        if let Err(e) = fs::create_dir_all(&blob_dest) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }
        blob_dest.push_str("/data");

        // copy temporary upload to final blob location
        if let Err(e) = fs::copy(&blob_upload_dest, &blob_dest) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }

        // remove temporary files
        if let Err(e) = fs::remove_dir_all(blob_upload_dest_dir) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }

        // create manifest link file in revisions
        let mut manifest_rev_dest = format!(
            "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/revisions/sha256/{}",
            name, hash
        );
        if let Err(e) = fs::create_dir_all(&manifest_rev_dest) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }
        manifest_rev_dest.push_str("/link");
        if let Err(e) = fs::write(manifest_rev_dest, format!("sha256:{}", hash)) {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }

        // create manifest link file in tags if reference is a tag (no colon)
        let colon = reference.find(':');
        if let None = colon {
            let mut manifest_tag_dest = format!(
                "/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current",
                name, reference
            );
            if let Err(e) = fs::create_dir_all(&manifest_tag_dest) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }
            manifest_tag_dest.push_str("/link");
            if let Err(e) = fs::write(manifest_tag_dest, format!("sha256:{}", hash)) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }
        }

        Ok(warp::http::response::Builder::new()
            .header(
                "Location",
                format!(
                    "http://localhost:7878/v2/{}/manifests/sha256:{}",
                    name, hash
                ),
            )
            .header("Docker-Content-Digest", format!("sha256:{}", hash))
            .status(StatusCode::CREATED)
            .body("")
            .unwrap())
    }
}

async fn handle_get_blobs(_name: String, hash: String) -> Result<impl Reply, Rejection> {
    let blob = format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data",
        hash.get(7..9).unwrap(),
        hash.get(7..).unwrap()
    );
    debug!("Getting blob: {}", blob);
    if !Path::new(&blob).is_file() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::BlobUnknown,
        }));
    }

    let blob_content = fs::read(blob);
    if blob_content.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::BlobUnknown,
        }));
    }

    let content = blob_content.unwrap();
    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(content)
        .unwrap())
}

async fn handle_post_blob(name: String) -> Result<impl Reply, Rejection> {
    let id = Uuid::new_v4();

    if let Err(e) = fs::create_dir_all(format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    )) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    Ok(warp::http::response::Builder::new()
        .header(
            "Location",
            format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, id),
        )
        .header("Range", "0-0")
        .status(StatusCode::ACCEPTED)
        .body("")
        .unwrap())
}

async fn handle_patch_blob(
    name: String,
    id: String,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    let mut blob_upload_dest = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data",
        name, id
    );
    let append = append_to_blob(&mut blob_upload_dest, bytes);
    if let Err(e) = append {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    } else {
        let append_result = append.ok().unwrap();
        let range = format!(
            "{}-{}",
            append_result.0,
            append_result.0 + append_result.1 - 1
        );
        debug!("Patch blob range: {}", range);
        return Ok(warp::http::response::Builder::new()
            .header(
                "Location",
                format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, id),
            )
            .header("Range", &range)
            .status(StatusCode::ACCEPTED)
            .body("")
            .unwrap());
    }
}

async fn handle_put_blob(
    name: String,
    id: String,
    params: HashMap<String, String>,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    let blob_upload_dest_dir = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    );
    let mut blob_upload_dest_data = blob_upload_dest_dir.clone();
    blob_upload_dest_data.push_str("/data");
    if let Err(e) = append_to_blob(&blob_upload_dest_data, bytes) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    let digest = match params.get("digest") {
        Some(v) => v,
        None => {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(String::from("missing digest")),
            }))
        }
    };

    let mut blob_dest = String::from(format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}",
        digest.get(7..9).unwrap(),
        digest.get(7..).unwrap()
    ));
    if let Err(e) = fs::create_dir_all(&blob_dest) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    blob_dest.push_str("/data");
    if let Err(e) = fs::copy(&blob_upload_dest_data, &blob_dest) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    if let Err(e) = fs::remove_dir_all(&blob_upload_dest_dir) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    Ok(warp::http::response::Builder::new()
        .header(
            "Location",
            format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, digest),
        )
        .status(StatusCode::CREATED)
        .body("")
        .unwrap())
}

fn append_to_blob(blob: &str, mut bytes: Bytes) -> Result<(u64, u64), std::io::Error> {
    debug!("Patching blob: {}", blob);
    let file = fs::OpenOptions::new().create(true).append(true).open(blob);
    let mut total_bytes_read: u64 = 0;
    let initial_file_length: u64;
    if let Ok(mut f) = file {
        initial_file_length = f.metadata().unwrap().len();
        while bytes.has_remaining() {
            let bytes_remaining = bytes.remaining();
            let bytes_to_read = if bytes_remaining <= 4096 {
                bytes_remaining
            } else {
                4096
            };
            total_bytes_read += bytes_to_read as u64;
            let mut b = vec![0; bytes_to_read];
            bytes.copy_to_slice(&mut b);
            if let Err(e) = f.write_all(&b) {
                error!("{}", e);
                return Err(e);
            }
        }
    } else {
        let e = file.err().unwrap();
        error!("{}", e);
        return Err(e);
    }

    Ok((initial_file_length, total_bytes_read))
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

#[derive(Debug, Deserialize, Serialize)]
struct ErrorMessage {
    code: RegistryErrorCode,
    message: String,
}
#[derive(Debug, Deserialize, Serialize)]
struct ErrorMessages {
    errors: Vec<ErrorMessage>,
}

async fn custom_recover(err: Rejection) -> Result<impl Reply, Infallible> {
    let mut status_code = StatusCode::INTERNAL_SERVER_ERROR;
    let mut error_message = ErrorMessage {
        code: RegistryErrorCode::Unknown("".to_string()),
        message: "".to_string(),
    };

    debug!("Rejection: {:?}", err);
    if let Some(e) = err.find::<RegistryError>() {
        match &e.code {
            RegistryErrorCode::BlobUnknown => {
                status_code = StatusCode::NOT_FOUND;
                error_message.code = RegistryErrorCode::BlobUnknown;
            }
            RegistryErrorCode::ManifestUnknown => {
                status_code = StatusCode::NOT_FOUND;
                error_message.code = RegistryErrorCode::ManifestUnknown;
            }
            RegistryErrorCode::Unknown(m) => {
                error_message.message = m.clone();
            }
        }
    } else if let Some(e) = err.find::<warp::reject::InvalidHeader>() {
        status_code = StatusCode::BAD_REQUEST;
        error_message.message = format!("{}", e);
    }

    debug!("ErrorMessage: {:?}", error_message);
    Ok(warp::reply::with_status(
        warp::reply::json(&ErrorMessages {
            errors: vec![error_message],
        }),
        status_code,
    )
    .into_response())
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
}
