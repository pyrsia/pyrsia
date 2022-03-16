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

pub mod args;

use pyrsia::docker::error_util::*;
use pyrsia::docker::v2::routes::make_docker_routes;
use pyrsia::logging::*;
use pyrsia::network::handlers::{dial_other_peer, handle_request_artifact};
use pyrsia::network::p2p::{self};
use pyrsia::node_api::routes::make_node_routes;

use clap::Parser;
use futures::StreamExt;
use log::{debug, info};
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use warp::Filter;

#[tokio::main]
async fn main() {
    pretty_env_logger::init();

    let args = args::parser::PyrsiaNodeArgs::parse();

    let (mut p2p_client, mut p2p_events, event_loop) = p2p::new().await.unwrap();

    tokio::spawn(event_loop.run());

    if let Some(to_dial) = args.peer {
        dial_other_peer(p2p_client.clone(), to_dial).await;
    }

    // Listen on all interfaces and whatever port the OS assigns
    p2p_client
        .listen(args.listen_address)
        .await
        .expect("Listening should not fail");

    // Get host and port from the settings. Defaults to DEFAULT_HOST and DEFAULT_PORT
    let host = args.host;
    let port = args.port;
    debug!(
        "Pyrsia Docker Node will bind to host = {}, port = {}",
        host, port
    );

    let address = SocketAddr::new(
        IpAddr::V4(host.parse::<Ipv4Addr>().unwrap()),
        port.parse::<u16>().unwrap(),
    );

    let docker_routes = make_docker_routes(p2p_client.clone());
    let node_api_routes = make_node_routes(p2p_client.clone());
    let all_routes = docker_routes.or(node_api_routes);

    let (addr, server) = warp::serve(
        all_routes
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
                    handle_request_artifact(p2p_client.clone(), &hash, channel).await
                }
            }
        }
    }
}
