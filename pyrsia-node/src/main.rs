extern crate bytes;
extern crate clap;
extern crate log;
extern crate pretty_env_logger;
extern crate tokio;
extern crate uuid;
extern crate warp;

use std::convert::Infallible;
use std::fs;
use std::io::prelude::*;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};

use bytes::{Buf, Bytes};
use clap::{App, Arg, ArgMatches};
use log::{debug, info};
use uuid::Uuid;
use warp::Filter;
use warp::http::{HeaderMap, StatusCode};
use warp::reject::Reject;

const DEFAULT_PORT: &str = "7878";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let mut authors: Vec<&'static str> = Vec::new();
    authors.push("Joeri Sykora <joeri@sertik.net>");
    authors.push("Elliott Frisch <elliottf@jfrog.com>");
    let matches: ArgMatches = App::new("Pyrsia Node")
        .version("0.1.0")
        .author(&*authors.join(", "))
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
        .get_matches();

    let verbosity: u64 = matches.occurrences_of("verbose");
    if verbosity > 0 {
        info!("Verbosity Level: {}", verbosity.to_string())
    }

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
        .with(warp::reply::with::header("Content-Length", empty_json.len()))
        .with(warp::reply::with::header("Content-Type", "application/json"));

    let v2_manifests = warp::path!("v2" / String / "manifests" / String)
        .and(warp::get().or(warp::head()).unify())
        .and_then(handle_get_manifests);

    let v2_blobs = warp::path!("v2" / String / "blobs" / String)
        .and(warp::get().or(warp::head()).unify())
        .and_then(handle_get_blobs);
    let v2_blobs_create = warp::path!("v2" / String / "blobs" / "uploads")
        .and(warp::post())
        .and_then(handle_create_blob);
    let v2_blobs_patch = warp::path!("v2" / String / "blobs" / "uploads" / String)
        .and(warp::patch())
        .and(warp::body::bytes())
        .and_then(handle_patch_blob);

    let routes = warp::any().and(log_headers()).and(
        v2_base.or(v2_manifests).or(v2_blobs).or(v2_blobs_create).or(v2_blobs_patch)
    ).with(warp::log("pyrsia_registry"));

    warp::serve(routes)
        .run(address)
        .await;
}

#[derive(Debug)]
struct CustomError(String);

impl Reject for CustomError {}

async fn handle_get_manifests(name: String, tag: String) -> Result<impl warp::Reply, warp::Rejection> {
    let colon = tag.find(':');
    let mut hash = String::from(&tag);
    if colon == None {
        let manifest = format!("/tmp/registry/docker/registry/v2/repositories/{}/_manifests/tags/{}/current/link", name, tag);
        let manifest_content = fs::read_to_string(manifest);
        if manifest_content.is_err() {
            // todo: generate error response as specified in https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
            return Err(warp::reject::not_found());
        }
        hash = manifest_content.unwrap();
    }

    let blob = format!("/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data", hash.get(7..9).unwrap(), hash.get(7..).unwrap());
    let blob_content = fs::read_to_string(blob);
    if blob_content.is_err() {
        // todo: generate error response as specified in https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
        return Err(warp::reject::not_found());
    }

    let content = blob_content.unwrap();
    return Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/vnd.docker.distribution.manifest.v2+json")
        .header("Content-Length", content.len())
        .status(StatusCode::OK)
        .body(content)
        .unwrap());
}

async fn handle_get_blobs(_name: String, hash: String) -> Result<impl warp::Reply, warp::Rejection> {
    let blob = format!("/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data", hash.get(7..9).unwrap(), hash.get(7..).unwrap());
    let blob_content = fs::read(blob);
    if blob_content.is_err() {
        // todo: generate error response as specified in https://github.com/opencontainers/distribution-spec/blob/main/spec.md#error-codes
        return Err(warp::reject::not_found());
    }

    let content = blob_content.unwrap();
    return Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .header("Content-Length", content.len())
        .status(StatusCode::OK)
        .body(content)
        .unwrap());
}

async fn handle_create_blob(name: String) -> Result<impl warp::Reply, warp::Rejection> {
    let id = Uuid::new_v4();

    if let Err(e) = fs::create_dir_all(format!("/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}", name, id)) {
        return Err(warp::reject::custom(CustomError(e.to_string())));
    }

    Ok(warp::http::response::Builder::new()
        .header("Location", format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, id))
        .status(StatusCode::ACCEPTED)
        .body("")
        .unwrap()
    )
}

async fn handle_patch_blob(name: String, id: String, mut bytes: Bytes) -> Result<impl warp::Reply, warp::Rejection> {
    let blob = format!("/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data", name, id);
    debug!("Patching blob: {}", blob);
    let file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(blob);
    if let Ok(mut f) = file {
        while bytes.has_remaining() {
            let bytes_remaining = bytes.remaining();
            let bytes_to_read = if bytes_remaining <= 4096 { bytes_remaining } else { 4096 };
            let mut b = vec![0; bytes_to_read];
            bytes.copy_to_slice(&mut b);
            if let Err(e) = f.write_all(&b) {
                return Err(warp::reject::custom(CustomError(e.to_string())));
            }
        }
    } else {
        let e = file.err().unwrap();
        return Err(warp::reject::custom(CustomError(e.to_string())));
    }

    Ok(warp::http::response::Builder::new()
        .header("Location", format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, id))
        .status(StatusCode::ACCEPTED)
        .body("")
        .unwrap()
    )
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
