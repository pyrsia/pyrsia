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

use pyrsia::artifact_manager::HashAlgorithm;
use pyrsia::docker::error_util::*;
use pyrsia::docker::v2::routes::*;
use pyrsia::logging::*;
use pyrsia::network::p2p::{self};
use pyrsia::node_api::routes::make_node_routes;
use pyrsia::node_manager::handlers::get_artifact;

use clap::{App, Arg, ArgMatches};
use futures::StreamExt;
use libp2p::{multiaddr::Protocol, Multiaddr, PeerId};
use log::{debug, error, info};
use std::{
    env,
    net::{IpAddr, Ipv4Addr, SocketAddr},
};
use warp::Filter;

const DEFAULT_HOST: &str = "127.0.0.1";
const DEFAULT_PORT: &str = "7888";

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

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
                .short('P')
                .long("peer")
                .takes_value(true)
                .required(false)
                .multiple_occurrences(false)
                .help("Provide an explicit peerId to connect with"),
        )
        .get_matches();

    let (mut p2p_client, mut p2p_events, event_loop) = p2p::new().await.unwrap();

    tokio::spawn(event_loop.run());

    // Reach out to another node if specified
    let mut final_peer_id: Option<PeerId> = None;
    if let Some(to_dial) = matches.value_of("peer") {
        let addr: Multiaddr = to_dial.parse().unwrap();
        let peer_id = match addr.iter().last() {
            Some(Protocol::P2p(hash)) => Ok(PeerId::from_multihash(hash).expect("Valid hash.")),
            _ => Err("Expect peer multiaddr to contain peer ID."),
        };
        match peer_id {
            Ok(peer_id) => {
                final_peer_id = Some(peer_id);
                p2p_client
                    .dial(peer_id, addr)
                    .await
                    .expect("Dial to succeed.");
                info!("Dialed {:?}", to_dial)
            }
            Err(e) => error!("Failed to dial peer: {}", e),
        };
    }

    // Listen on all interfaces and whatever port the OS assigns
    p2p_client
        .listen("/ip4/0.0.0.0/tcp/0".parse().unwrap())
        .await
        .expect("Listening should not fail");

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

    let docker_routes = make_docker_routes(p2p_client.clone(), final_peer_id);
    let routes = docker_routes.or(make_node_routes(p2p_client.clone()));

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

    loop {
        if let Some(event) = p2p_events.next().await {
            match event {
                // Reply with the content of the artifact on incoming requests.
                pyrsia::network::p2p::Event::InboundRequest { hash, channel } => {
                    let decoded_hash = hex::decode(&hash.get(7..).unwrap()).unwrap();
                    match get_artifact(&decoded_hash, HashAlgorithm::SHA256) {
                        Ok(content) => p2p_client.respond_artifact(content, channel).await,
                        Err(e) => info!(
                            "This node does not provide artifact {}. Error: {:?}",
                            hash, e
                        ),
                    }
                }
            }
        }
    }
}
