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
use libp2p::identity::Keypair;
use libp2p::PeerId;
use network::handlers;
use pyrsia::artifact_service::service::ArtifactService;
use pyrsia::artifact_service::storage::ARTIFACTS_DIR;
use pyrsia::blockchain_service::service::BlockchainService;
use pyrsia::build_service::event::{BuildEventClient, BuildEventLoop};
use pyrsia::build_service::service::BuildService;
use pyrsia::docker::error_util::*;
use pyrsia::docker::v2::routes::make_docker_routes;
use pyrsia::java::maven2::routes::make_maven_routes;
use pyrsia::logging::*;
use pyrsia::network::client::Client;
use pyrsia::network::p2p;
use pyrsia::node_api::routes::make_node_routes;
use pyrsia::transparency_log::log::TransparencyLogService;
use pyrsia::util::env_util::read_var;
use pyrsia::util::keypair_util::{self, KEYPAIR_FILENAME};
use pyrsia::verification_service::service::VerificationService;

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
    let (p2p_client, local_keypair, mut p2p_events, event_loop) =
        p2p::setup_libp2p_swarm(args.max_provided_keys)?;

    debug!("Start p2p event loop");
    tokio::spawn(event_loop.run());

    debug!("Create blockchain service component");
    let blockchain_service =
        setup_blockchain_service(local_keypair, p2p_client.clone(), &args).await?;
    let blockchain_service = Arc::new(Mutex::new(blockchain_service));

    debug!("Create transparency log service");
    let transparency_log_service = setup_transparency_log_service(blockchain_service.clone())?;

    debug!("Create pyrsia services");
    let (build_event_client, artifact_service) =
        setup_pyrsia_services(transparency_log_service.clone(), p2p_client.clone(), &args).await?;

    debug!("Setup HTTP server");
    setup_http(
        &args,
        artifact_service.clone(),
        transparency_log_service,
        p2p_client.clone(),
    );

    debug!("Start p2p components");
    let other_peer_id = setup_p2p(p2p_client.clone(), &args).await?;

    if !args.init_blockchain {
        pull_block_from_other_nodes(
            artifact_service.clone(),
            blockchain_service.clone(),
            other_peer_id,
        )
        .await?;
    }

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
                pyrsia::network::event_loop::PyrsiaEvent::RequestBuild {
                    package_type,
                    package_specific_id,
                    channel,
                } => {
                    debug!(
                        "Main::p2p request build: {:?} : {}",
                        package_type, package_specific_id
                    );
                    if let Err(error) = handlers::handle_request_build(
                        build_event_client.clone(),
                        package_type,
                        &package_specific_id,
                        channel,
                    )
                    .await
                    {
                        warn!(
                            "This node failed to provide build with package type {:?} and id {}. Error: {:?}",
                            package_type, package_specific_id, error
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
                pyrsia::network::event_loop::PyrsiaEvent::BlockchainRequest { data, channel } => {
                    if let Err(error) = handlers::handle_request_blockchain(
                        artifact_service.clone(),
                        blockchain_service.clone(),
                        data,
                        channel,
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

async fn setup_p2p(
    mut p2p_client: Client,
    args: &PyrsiaNodeArgs,
) -> anyhow::Result<Option<PeerId>> {
    p2p_client.listen(&args.listen_address).await?;
    let mut other_peer_id: Option<PeerId> = None;
    if let Some(to_probe) = &args.probe {
        info!("Invoking probe");
        handlers::probe_other_peer(p2p_client.clone(), to_probe).await?;
        other_peer_id = libp2p::PeerId::try_from_multiaddr(to_probe);
    } else if let Some(to_dial) = &args.peer {
        info!("Invoking dial");
        handlers::dial_other_peer(p2p_client.clone(), to_dial).await?;
        other_peer_id = libp2p::PeerId::try_from_multiaddr(to_dial);
    } else if args.listen_only {
        info!("Pyrsia node will listen only. No attempt to connect to other nodes.");
    } else {
        info!("Looking up bootstrap node");
        let peer_addrs = load_peer_addrs(&args.bootstrap_url).await?;
        // Turbofish! https://doc.rust-lang.org/std/primitive.str.html#method.parse
        let pa = peer_addrs.parse::<libp2p::Multiaddr>()?;
        info!("Probing {:?}", pa);
        handlers::probe_other_peer(p2p_client.clone(), &pa).await?;
        other_peer_id = libp2p::PeerId::try_from_multiaddr(&pa);
    }

    Ok(other_peer_id)
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
                    if vec_jv.is_empty() {
                        return Err(anyhow!(
                            "Could not read status from {} error {:?}",
                            peer_url,
                            "did not receive a valid array of peer_addrs in JSON"
                        ));
                    }
                    let peer_addrs: String = vec_jv[0].to_string();
                    info!("Found bootstrap peer_addr {:?}", peer_addrs);
                    if !peer_addrs.is_empty() {
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

async fn setup_blockchain_service(
    local_keypair: Keypair,
    p2p_client: Client,
    args: &PyrsiaNodeArgs,
) -> Result<BlockchainService> {
    let local_ed25519_keypair = match local_keypair {
        libp2p::identity::Keypair::Ed25519(v) => v,
        _ => {
            bail!("Keypair Format Error");
        }
    };

    let pyrsia_blockchain_path = read_var("PYRSIA_BLOCKCHAIN_PATH", "pyrsia/blockchain");

    if args.init_blockchain {
        let blockchain_keypair =
            keypair_util::load_or_generate_ed25519(PathBuf::from(KEYPAIR_FILENAME.as_str()));

        let blockchain_ed25519_keypair = match blockchain_keypair {
            libp2p::identity::Keypair::Ed25519(v) => v,
            _ => {
                bail!("Keypair Format Error");
            }
        };

        // Refactor to overloading(trait) later
        BlockchainService::init_first_blockchain_node(
            &local_ed25519_keypair,
            &blockchain_ed25519_keypair,
            p2p_client,
            pyrsia_blockchain_path,
        )
        .await
        .map_err(|e| e.into())
    } else {
        BlockchainService::init_other_blockchain_node(
            &local_ed25519_keypair,
            p2p_client,
            pyrsia_blockchain_path,
        )
        .map_err(|e| e.into())
    }
}

fn setup_transparency_log_service(
    blockchain_service: Arc<Mutex<BlockchainService>>,
) -> Result<TransparencyLogService> {
    let artifact_path = PathBuf::from(ARTIFACTS_DIR.as_str());

    let transparency_log_service = TransparencyLogService::new(&artifact_path, blockchain_service)?;

    Ok(transparency_log_service)
}

async fn setup_pyrsia_services(
    transparency_log_service: TransparencyLogService,
    p2p_client: Client,
    args: &PyrsiaNodeArgs,
) -> Result<(BuildEventClient, ArtifactService)> {
    let artifact_path = PathBuf::from(ARTIFACTS_DIR.as_str());

    debug!("Create build event client");
    let (build_event_sender, build_event_receiver) = mpsc::channel(32);
    let build_event_client = BuildEventClient::new(build_event_sender);

    debug!("Create artifact service");
    let artifact_service = setup_artifact_service(
        &artifact_path,
        transparency_log_service,
        build_event_client.clone(),
        p2p_client,
    )?;

    debug!("Create build service");
    let build_service = setup_build_service(&artifact_path, build_event_client.clone(), args)?;

    debug!("Create verification service");
    let verification_service = VerificationService::new(build_event_client.clone())?;

    debug!("Start build event loop");
    let build_event_loop = BuildEventLoop::new(
        artifact_service.clone(),
        build_service,
        verification_service,
        build_event_receiver,
    );
    tokio::spawn(build_event_loop.run());

    Ok((build_event_client, artifact_service))
}

fn setup_artifact_service(
    artifact_path: &Path,
    transparency_log_service: TransparencyLogService,
    build_event_client: BuildEventClient,
    p2p_client: Client,
) -> Result<ArtifactService> {
    let artifact_service = ArtifactService::new(
        artifact_path,
        transparency_log_service,
        build_event_client,
        p2p_client,
    )?;

    Ok(artifact_service)
}

fn setup_build_service(
    artifact_path: &Path,
    build_event_client: BuildEventClient,
    args: &PyrsiaNodeArgs,
) -> Result<BuildService> {
    let build_service = BuildService::new(
        &artifact_path,
        build_event_client,
        &args.mapping_service_endpoint,
        &args.pipeline_service_endpoint,
    )?;

    Ok(build_service)
}

fn setup_http(
    args: &PyrsiaNodeArgs,
    artifact_service: ArtifactService,
    transparency_log_service: TransparencyLogService,
    p2p_client: Client,
) {
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
    let node_api_routes = make_node_routes(artifact_service, p2p_client, transparency_log_service);
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

async fn pull_block_from_other_nodes(
    mut artifact_service: ArtifactService,
    blockchain_service: Arc<Mutex<BlockchainService>>,
    other_peer_id: Option<PeerId>,
) -> anyhow::Result<()> {
    if let Some(other_peer_id) = other_peer_id {
        debug!("Blockchain start pulling from other nodes");

        let mut blockchain_service = blockchain_service.lock().await;

        let ordinal = blockchain_service
            .init_pull_from_others(&other_peer_id)
            .await?;
        for block in blockchain_service.pull_blocks(1, ordinal).await? {
            let payloads = block.fetch_payload();
            artifact_service.handle_block_added(payloads).await?;
        }
    }
    Ok(())
}
