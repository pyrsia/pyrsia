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

use anyhow::{bail, Result};
use args::parser::PyrsiaNodeArgs;
use network::handlers;
use pyrsia::artifact_service::service::ArtifactService;
use pyrsia::artifact_service::storage::ARTIFACTS_DIR;
use pyrsia::build_service::event::{BuildEventClient, BuildEventLoop};
use pyrsia::build_service::service::BuildService;
use pyrsia::docker::error_util::*;
use pyrsia::docker::v2::routes::make_docker_routes;
use pyrsia::java::maven2::routes::make_maven_routes;
use pyrsia::logging::*;
use pyrsia::network::client::Client;
use pyrsia::network::p2p;
use pyrsia::node_api::routes::make_node_routes;
use pyrsia::util::keypair_util::{self, KEYPAIR_FILENAME};
use pyrsia::verification_service::service::VerificationService;
use pyrsia_blockchain_network::blockchain::Blockchain;

use clap::Parser;
use log::{debug, info, warn};
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};
use tokio_stream::StreamExt;
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init_timed();

    debug!("Parse CLI arguments");
    let args = PyrsiaNodeArgs::parse();

    debug!("Create p2p components");
    let (p2p_client, mut p2p_events, event_loop) = p2p::setup_libp2p_swarm(args.max_provided_keys)?;

    debug!("Create blockchain components");
    let blockchain = setup_blockchain()?;

    debug!("Start p2p event loop");
    tokio::spawn(event_loop.run());

    debug!("Create pyrsia services");
    let artifact_service = setup_pyrsia_services(p2p_client.clone(), &args)?;

    debug!("Setup HTTP server");
    setup_http(&args, artifact_service.clone());

    debug!("Start p2p components");
    setup_p2p(p2p_client.clone(), &args).await?;

    debug!("Listen for p2p events");
    loop {
        if let Some(event) = p2p_events.next().await {
            match event {
                // Reply with the content of the artifact on incoming requests.
                pyrsia::network::event_loop::PyrsiaEvent::RequestArtifact {
                    artifact_id,
                    channel,
                } => {
                    if let Err(error) = handlers::handle_request_artifact(
                        artifact_service.clone(),
                        &artifact_id,
                        channel,
                    )
                    .await
                    {
                        warn!(
                            "This node failed to provide artifact with id {}. Error: {:?}",
                            artifact_id, error
                        );
                    }
                }
                pyrsia::network::event_loop::PyrsiaEvent::IdleMetricRequest { channel } => {
                    if let Err(error) =
                        handlers::handle_request_idle_metric(p2p_client.clone(), channel).await
                    {
                        warn!(
                            "This node failed to provide idle metrics. Error: {:?}",
                            error
                        );
                    }
                }
                pyrsia::network::event_loop::PyrsiaEvent::BlockUpdateRequest {
                    block_ordinal,
                    block,
                } => {
                    if let Err(error) = handlers::handle_request_block_update(
                        p2p_client.clone(),
                        blockchain.clone(),
                        block_ordinal,
                        block.clone(),
                    )
                    .await
                    {
                        warn!("This node failed to update blockchain Error: {:?}", error);
                    }
                }
            }
        }
    }
}

async fn setup_p2p(mut p2p_client: Client, args: &PyrsiaNodeArgs) -> anyhow::Result<()> {
    p2p_client.listen(&args.listen_address).await?;
    if let Some(to_probe) = &args.probe {
        info!("Invoking probe");
        handlers::probe_other_peer(p2p_client.clone(), to_probe).await
    } else if let Some(to_dial) = &args.peer {
        info!("Invoking dial");
        handlers::dial_other_peer(p2p_client.clone(), to_dial).await
    } else {
        info!("Looking up bootstrap node");
        let peer_addrs = load_peer_addrs(&args.bootstrap_url).await?;
        // Turbofish! https://doc.rust-lang.org/std/primitive.str.html#method.parse
        let pa = peer_addrs.parse::<libp2p::Multiaddr>()?;
        info!("Probing {:?}", pa);
        handlers::probe_other_peer(p2p_client.clone(), &pa).await
    }
}

async fn load_peer_addrs(peer_url: &str) -> anyhow::Result<String> {
    use anyhow::anyhow;

    let client = reqwest::Client::new();
    let response = client.get(peer_url).send().await;
    match response {
        Ok(body) => {
            let text: String = body.text().await?;
            let jv: json::JsonValue = json::parse(&text)?;
            let arr = &jv["peer_addrs"];
            match arr {
                json::JsonValue::Array(vec_jv) => {
                    if vec_jv.len() < 1 {
                        return Err(anyhow!(
                            "Could not read status from {} error {:?}",
                            peer_url,
                            "did not receive a valid array of peer_addrs in JSON"
                        ));
                    }
                    let peer_addrs: String = vec_jv[0].to_string();
                    info!("Found bootstrap peer_addr {:?}", peer_addrs);
                    if peer_addrs.len() > 0 {
                        Ok(peer_addrs)
                    } else {
                        Err(anyhow!("Could not read peer_addrs from {}", peer_url))
                    }
                }
                _ => Err(anyhow!(
                    "Could not read status from {} error {:?}",
                    peer_url,
                    "did not receive an array of peer_addrs in JSON"
                )),
            }
        }
        Err(err) => Err(anyhow!(
            "Could not read status from {} error {:?}",
            peer_url,
            err
        )),
    }
}

fn setup_blockchain() -> Result<Arc<Mutex<Blockchain>>> {
    let local_keypair =
        keypair_util::load_or_generate_ed25519(PathBuf::from(KEYPAIR_FILENAME.as_str()));

    let ed25519_keypair = match local_keypair {
        libp2p::identity::Keypair::Ed25519(v) => v,
        _ => {
            bail!("Keypair Format Error");
        }
    };

    Ok(Arc::new(Mutex::new(Blockchain::new(&ed25519_keypair))))
}

fn setup_pyrsia_services(
    p2p_client: Client,
    args: &PyrsiaNodeArgs,
) -> Result<Arc<Mutex<ArtifactService>>> {
    let artifact_path = PathBuf::from(ARTIFACTS_DIR.as_str());
    let (build_event_sender, build_event_receiver) = mpsc::channel(32);
    let build_event_client = BuildEventClient::new(build_event_sender);

    debug!("Create artifact service");
    let artifact_service =
        setup_artifact_service(&artifact_path, build_event_client.clone(), p2p_client)?;

    debug!("Create build service");
    let build_service = setup_build_service(&artifact_path, build_event_client.clone(), args)?;

    debug!("Create verification service");
    let verification_service = setup_verification_service(build_event_client)?;

    debug!("Start build event loop");
    let build_event_loop = BuildEventLoop::new(
        artifact_service.clone(),
        build_service,
        verification_service,
        build_event_receiver,
    );
    tokio::spawn(build_event_loop.run());

    Ok(artifact_service)
}

fn setup_artifact_service(
    artifact_path: &Path,
    build_event_client: BuildEventClient,
    p2p_client: Client,
) -> Result<Arc<Mutex<ArtifactService>>> {
    let artifact_service = ArtifactService::new(artifact_path, build_event_client, p2p_client)?;

    Ok(Arc::new(Mutex::new(artifact_service)))
}

fn setup_build_service(
    artifact_path: &Path,
    build_event_client: BuildEventClient,
    args: &PyrsiaNodeArgs,
) -> Result<Arc<Mutex<BuildService>>> {
    let build_service = BuildService::new(
        &artifact_path,
        build_event_client,
        &args.mapping_service_endpoint,
        &args.pipeline_service_endpoint,
    )?;

    Ok(Arc::new(Mutex::new(build_service)))
}

fn setup_verification_service(
    build_event_client: BuildEventClient,
) -> Result<Arc<Mutex<VerificationService>>> {
    let verification_service = VerificationService::new(build_event_client)?;

    Ok(Arc::new(Mutex::new(verification_service)))
}

fn setup_http(args: &PyrsiaNodeArgs, artifact_service: Arc<Mutex<ArtifactService>>) {
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
    let docker_routes = make_docker_routes(artifact_service.clone());
    let maven_routes = make_maven_routes(artifact_service.clone());
    let node_api_routes = make_node_routes(artifact_service);
    let all_routes = docker_routes.or(maven_routes).or(node_api_routes);

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

#[cfg(test)]
mod tests {
    use crate::setup_blockchain;

    #[test]
    fn setup_blockchain_success() {
        let blockchain = setup_blockchain();
        assert!(blockchain.is_ok());
    }
}
