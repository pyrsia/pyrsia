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
pub mod network;

use args::parser::PyrsiaNodeArgs;
use network::handlers;
use pyrsia::docker::error_util::*;
use pyrsia::docker::v2::routes::make_docker_routes;
use pyrsia::logging::*;
use pyrsia::network::client::Client;
use pyrsia::network_central::p2p;
use pyrsia::node_api::routes::make_node_routes;

use clap::Parser;
use futures::StreamExt;
use log::{debug, info, warn};
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init();

    debug!("Parse CLI arguments");
    let args = PyrsiaNodeArgs::parse();

    debug!("Create p2p components");
    let (p2p_client, mut p2p_events, event_loop) = p2p::setup_libp2p_swarm(args.max_provided_keys)?;

    debug!("Start p2p event loop");
    tokio::spawn(event_loop.run());

    debug!("Setup HTTP server");
    setup_http(&args, p2p_client.clone());

    debug!("Start p2p components");
    setup_p2p(p2p_client.clone(), args).await;

    debug!("Listen for p2p events");
    loop {
        if let Some(event) = p2p_events.next().await {
            match event {
                // Reply with the content of the artifact on incoming requests.
                pyrsia::network_central::event_loop::PyrsiaEvent::RequestArtifact {
                    artifact_type,
                    artifact_hash,
                    channel,
                } => {
                    if let Err(error) = handlers::handle_request_artifact(
                        p2p_client.clone(),
                        &artifact_type,
                        &artifact_hash,
                        channel,
                    )
                    .await
                    {
                        warn!(
                            "This node failed to provide artifact with type {} and hash {}. Error: {:?}",
                            artifact_type, artifact_hash, error
                        );
                    }
                }
                pyrsia::network_central::event_loop::PyrsiaEvent::IdleMetricRequest { channel } => {
                    if let Err(error) =
                        handlers::handle_request_idle_metric(p2p_client.clone(), channel).await
                    {
                        warn!(
                            "This node failed to provide idle metrics. Error: {:?}",
                            error
                        );
                    }
                }
            }
        }
    }
}

fn setup_http(args: &PyrsiaNodeArgs, p2p_client: Client) {
    // Get host and port from the settings. Defaults to DEFAULT_HOST and DEFAULT_PORT
    debug!(
        "Pyrsia Docker Node will bind to host = {}, port = {}",
        args.host, args.port
    );

    let address = SocketAddr::new(
        IpAddr::V4(args.host.parse::<Ipv4Addr>().unwrap()),
        args.port.parse::<u16>().unwrap(),
    );

    debug!("Setup HTTP routing");
    let docker_routes = make_docker_routes(p2p_client.clone());
    let node_api_routes = make_node_routes(p2p_client);
    let all_routes = docker_routes.or(node_api_routes);

    debug!("Setup HTTP server");
    let (addr, server) = warp::serve(
        all_routes
            .and(http::log_headers())
            .recover(custom_recover)
            .with(warp::log("pyrsia_registry")),
    )
    .bind_ephemeral(address);

    info!(
        "Pyrsia Docker Node will start running on {}:{}",
        addr.ip(),
        addr.port()
    );

    tokio::spawn(server);
}

async fn setup_p2p(mut p2p_client: Client, args: PyrsiaNodeArgs) {
    p2p_client
        .listen(&args.listen_address)
        .await
        .expect("Listening should not fail");

    if let Some(to_dial) = args.peer {
        handlers::dial_other_peer(p2p_client.clone(), &to_dial).await;
    }

    debug!("Provide local artifacts");
    if let Err(error) = handlers::provide_artifacts(p2p_client.clone()).await {
        warn!(
            "An error occured while providing local artifacts. Error: {:?}",
            error
        );
    }
}
