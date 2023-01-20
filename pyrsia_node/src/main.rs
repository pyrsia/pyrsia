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

use anyhow::Result;
use args::parser::PyrsiaNodeArgs;
use libp2p::identity::Keypair;
use libp2p::PeerId;
use network::handlers;
use pyrsia::artifact_service::service::ArtifactService;
use pyrsia::artifact_service::storage::ARTIFACTS_DIR;
use pyrsia::blockchain_service::event::{BlockchainEventClient, BlockchainEventLoop};
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
use pyrsia::peer_metrics::metrics::PeerMetrics;
use pyrsia::util::env_util::read_var;
use pyrsia::util::keypair_util::{self, KEYPAIR_FILENAME};
use pyrsia::verification_service::service::VerificationService;

use clap::Parser;
use log::{debug, info, warn};
use std::error::Error;
use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::{Path, PathBuf};
use tokio::sync::mpsc;
use tokio_stream::StreamExt;
use warp::Filter;

#[tokio::main]
async fn main() -> Result<(), Box<dyn Error>> {
    pretty_env_logger::init_timed();

    debug!("Parse CLI arguments");
    let args = PyrsiaNodeArgs::parse();

    let mut peer_metrics = PeerMetrics::new();

    debug!("Create p2p components");
    let (mut p2p_client, local_keypair, mut p2p_events, event_loop) =
        p2p::setup_libp2p_swarm(args.max_provided_keys)?;

    debug!("Start p2p event loop");
    tokio::spawn(event_loop.run());

    debug!("Create pyrsia services");
    let (blockchain_event_client, build_event_client, artifact_service) =
        setup_pyrsia_services(p2p_client.clone(), local_keypair, &args).await?;

    debug!("Setup HTTP server");
    setup_http(&args, artifact_service.clone(), p2p_client.clone());

    debug!("Establishing connection with p2p network");
    establish_connection_with_p2p_network(
        p2p_client.clone(),
        artifact_service.clone(),
        blockchain_event_client.clone(),
        args.clone(),
    )
    .await;

    debug!("Provide local artifacts");
    artifact_service.clone().provide_local_artifacts().await?;

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
                        p2p_client.clone(),
                        build_event_client.clone(),
                        package_type,
                        &package_specific_id,
                        channel,
                    )
                    .await
                    {
                        warn!(
                            "This node failed to start build with package type {:?} and id {}. Error: {:?}",
                            package_type, package_specific_id, error
                        );
                    }
                }
                pyrsia::network::event_loop::PyrsiaEvent::IdleMetricRequest { channel } => {
                    if let Err(error) = handlers::handle_request_idle_metric(
                        p2p_client.clone(),
                        &mut peer_metrics,
                        channel,
                    )
                    .await
                    {
                        warn!(
                            "This node failed to provide idle metrics. Error: {:?}",
                            error
                        );
                    }
                }
                pyrsia::network::event_loop::PyrsiaEvent::BlockchainRequest { data, channel } => {
                    match handlers::handle_incoming_blockchain_command(
                        blockchain_event_client.clone(),
                        data,
                    )
                    .await
                    {
                        Ok(response_data) => {
                            if let Some(channel) = channel {
                                if let Err(err) =
                                    p2p_client.respond_blockchain(response_data, channel).await
                                {
                                    warn!(
                                        "This node failed to update blockchain. Error: {:?}",
                                        err
                                    );
                                }
                            }
                        }
                        Err(err) => {
                            warn!("This node failed to update blockchain. Error: {:?}", err);
                        }
                    }
                }
                pyrsia::network::event_loop::PyrsiaEvent::RequestBuildStatus {
                    build_id,
                    channel,
                } => {
                    debug!(
                        "Main::p2p request build status based build ID: {:?}",
                        build_id
                    );
                    if let Err(error) = handlers::handle_request_build_status(
                        p2p_client.clone(),
                        build_event_client.clone(),
                        &build_id,
                        channel,
                    )
                    .await
                    {
                        warn!(
                            "This node failed to obtain build status {:?}. Error: {:?}",
                            build_id, error
                        );
                    }
                }
            }
        }
    }
}

async fn establish_connection_with_p2p_network(
    p2p_client: Client,
    artifact_service: ArtifactService,
    blockchain_event_client: BlockchainEventClient,
    args: PyrsiaNodeArgs,
) {
    tokio::spawn(async move {
        if let Some(other_peer_id) = connect_to_p2p_network(p2p_client, &args)
            .await
            .unwrap_or_else(|err| {
                warn!("Failed to establish connection with p2p network: {:?}", err);
                None
            })
        {
            if !args.init_blockchain {
                if let Err(err) = pull_block_from_other_nodes(
                    artifact_service.clone(),
                    blockchain_event_client,
                    &other_peer_id,
                )
                .await
                {
                    panic!("Failed to pull blocks from p2p network: {:?}", err);
                }
            }
        }
    });
}

async fn connect_to_p2p_network(
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
        info!("Looking up bootstrap node: {:?}", &args.bootstrap_url);
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

async fn setup_pyrsia_services(
    p2p_client: Client,
    local_keypair: Keypair,
    args: &PyrsiaNodeArgs,
) -> Result<(BlockchainEventClient, BuildEventClient, ArtifactService)> {
    let Keypair::Ed25519(local_ed25519_keypair) = local_keypair;

    let artifact_path = PathBuf::from(ARTIFACTS_DIR.as_str());

    let pyrsia_blockchain_path = read_var("PYRSIA_BLOCKCHAIN_PATH", "pyrsia/blockchain");

    debug!("Create blockchain service");
    let blockchain_service = if args.init_blockchain {
        let blockchain_keypair =
            keypair_util::load_or_generate_ed25519(PathBuf::from(KEYPAIR_FILENAME.as_str()));

        let Keypair::Ed25519(blockchain_ed25519_keypair) = blockchain_keypair;

        // Refactor to overloading(trait) later
        BlockchainService::init_first_blockchain_node(
            &local_ed25519_keypair,
            &blockchain_ed25519_keypair,
            p2p_client.clone(),
            pyrsia_blockchain_path,
        )
        .await
    } else {
        BlockchainService::init_other_blockchain_node(
            &local_ed25519_keypair,
            p2p_client.clone(),
            pyrsia_blockchain_path,
        )
    }?;

    debug!("Create blockchain event client");
    let (blockchain_event_sender, blockchain_event_receiver) = mpsc::channel(32);
    let blockchain_event_client = BlockchainEventClient::new(blockchain_event_sender);

    debug!("Create build event client");
    let (build_event_sender, build_event_receiver) = mpsc::channel(32);
    let build_event_client = BuildEventClient::new(build_event_sender);

    debug!("Create artifact service");
    let artifact_service = setup_artifact_service(
        &artifact_path,
        blockchain_event_client.clone(),
        build_event_client.clone(),
        p2p_client,
    )?;

    debug!("Create build service");
    let build_service = setup_build_service(&artifact_path, build_event_client.clone(), args)?;

    debug!("Create verification service");
    let verification_service = VerificationService::new(build_event_client.clone())?;

    debug!("Start blockchain event loop");
    let blockchain_event_loop = BlockchainEventLoop::new(
        artifact_service.clone(),
        blockchain_service,
        blockchain_event_receiver,
    );
    tokio::spawn(blockchain_event_loop.run());

    debug!("Start build event loop");
    let build_event_loop = BuildEventLoop::new(
        artifact_service.clone(),
        build_service,
        verification_service,
        build_event_receiver,
    );
    tokio::spawn(build_event_loop.run());

    Ok((
        blockchain_event_client,
        build_event_client,
        artifact_service,
    ))
}

fn setup_artifact_service(
    artifact_path: &Path,
    blockchain_event_client: BlockchainEventClient,
    build_event_client: BuildEventClient,
    p2p_client: Client,
) -> Result<ArtifactService> {
    let artifact_service = ArtifactService::new(
        artifact_path,
        blockchain_event_client,
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
        artifact_path,
        build_event_client,
        &args.mapping_service_endpoint,
        &args.pipeline_service_endpoint,
    )?;

    Ok(build_service)
}

fn setup_http(args: &PyrsiaNodeArgs, artifact_service: ArtifactService, p2p_client: Client) {
    // Get host and port from the settings. Defaults to DEFAULT_HOST and DEFAULT_PORT
    debug!(
        "Pyrsia Node will bind to host = {}, port = {}",
        args.host, args.port
    );

    let address = SocketAddr::new(
        IpAddr::V4(args.host.parse::<Ipv4Addr>().unwrap()),
        args.port.parse::<u16>().unwrap(),
    );

    debug!("Setup HTTP routing");
    let docker_routes = make_docker_routes(artifact_service.clone());
    let maven_routes = make_maven_routes(artifact_service.clone());
    let node_api_routes = make_node_routes(artifact_service, p2p_client);
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
        "Pyrsia Node will start running on {}:{}",
        addr.ip(),
        addr.port()
    );

    tokio::spawn(server);
}

async fn pull_block_from_other_nodes(
    mut artifact_service: ArtifactService,
    blockchain_event_client: BlockchainEventClient,
    other_peer_id: &PeerId,
) -> anyhow::Result<()> {
    debug!("Blockchain start pulling blocks from other peers");

    let ordinal = blockchain_event_client
        .pull_blocks_from_peer(other_peer_id)
        .await?;

    for block in blockchain_event_client
        .pull_blocks_local(1, ordinal)
        .await?
    {
        let payloads = block.fetch_payload();
        artifact_service.handle_block_added(payloads).await?;
    }

    Ok(())
}
