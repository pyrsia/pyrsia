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

use anyhow::bail;
use bincode::{deserialize, serialize};
use libp2p::multiaddr::Protocol;
use libp2p::request_response::ResponseChannel;
use libp2p::{Multiaddr, PeerId};
use log::debug;

use pyrsia::artifact_service::model::PackageType;
use pyrsia::artifact_service::service::ArtifactService;
use pyrsia::blockchain_service::service::{BlockchainCommand, BlockchainService};
use pyrsia::build_service::error::BuildError;
use pyrsia::build_service::event::BuildEventClient;
use pyrsia::network::artifact_protocol::ArtifactResponse;
use pyrsia::network::blockchain_protocol::BlockchainResponse;
use pyrsia::network::build_protocol::BuildResponse;
use pyrsia::network::client::Client;
use pyrsia::network::idle_metric_protocol::{IdleMetricResponse, PeerMetrics};
use pyrsia::peer_metrics;
use pyrsia_blockchain_network::error::BlockchainError;
use pyrsia_blockchain_network::structures::block::Block;
use pyrsia_blockchain_network::structures::header::Ordinal;
use std::sync::Arc;
use tokio::sync::Mutex;

/// Reach out to another node with the specified address
pub async fn dial_other_peer(mut p2p_client: Client, to_dial: &Multiaddr) -> anyhow::Result<()> {
    match to_dial.iter().last() {
        Some(Protocol::P2p(hash)) => match PeerId::from_multihash(hash) {
            Ok(peer_id) => p2p_client.dial(&peer_id, to_dial).await,
            Err(_) => anyhow::bail!("Invalid hash provided for Peer ID."),
        },
        _ => anyhow::bail!("Expect peer address to contain Peer ID."),
    }
}

/// AutoNAT probe another node with the specified address
pub async fn probe_other_peer(mut p2p_client: Client, to_probe: &Multiaddr) -> anyhow::Result<()> {
    match to_probe.iter().last() {
        Some(Protocol::P2p(hash)) => match PeerId::from_multihash(hash) {
            Ok(peer_id) => p2p_client.add_probe_address(&peer_id, to_probe).await,
            Err(_) => anyhow::bail!("Invalid hash provided for Peer ID."),
        },
        _ => anyhow::bail!("Expect peer address to contain Peer ID."),
    }
}

/// Respond to a RequestArtifact event by getting the artifact
/// based on the provided artifact id.
pub async fn handle_request_artifact(
    mut artifact_service: ArtifactService,
    artifact_id: &str,
    channel: ResponseChannel<ArtifactResponse>,
) -> anyhow::Result<()> {
    debug!("Handling request artifact: {:?}", artifact_id);

    let content = artifact_service.get_artifact_locally(artifact_id).await?;

    artifact_service
        .p2p_client
        .respond_artifact(content, channel)
        .await
}

/// Respond to a RequestBuild event by getting the build
/// based on the provided package_type and package_specific_id.
pub async fn handle_request_build(
    build_event_client: BuildEventClient,
    package_type: PackageType,
    package_specific_id: &str,
    _: ResponseChannel<BuildResponse>,
) -> Result<String, BuildError> {
    debug!(
        "Handling request build: {:?} : {}",
        package_type, package_specific_id
    );

    build_event_client
        .start_build(package_type, package_specific_id.to_string())
        .await
}

//Respond to the IdleMetricRequest event
pub async fn handle_request_idle_metric(
    mut p2p_client: Client,
    channel: ResponseChannel<IdleMetricResponse>,
) -> anyhow::Result<()> {
    let metric = peer_metrics::metrics::get_quality_metric();
    let peer_metrics: PeerMetrics = PeerMetrics {
        idle_metric: metric.to_le_bytes(),
    };
    p2p_client.respond_idle_metric(peer_metrics, channel).await
}

//Respond to the BlockchainRequest event
pub async fn handle_request_blockchain(
    artifact_service: ArtifactService,
    blockchain_service: Arc<Mutex<BlockchainService>>,
    data: Vec<u8>,
    channel: ResponseChannel<BlockchainResponse>,
) -> anyhow::Result<()> {
    debug!("Handling request blockchain");
    match BlockchainCommand::try_from(data[0])? {
        BlockchainCommand::Broadcast => {
            debug!("Blockchain receives BlockchainCommand::Broadcast");
            let block_ordinal: Ordinal = deserialize(&data[1..17])?;
            let block: Block = deserialize(&data[17..])?;
            handle_broadcast_blockchain(
                artifact_service,
                blockchain_service,
                block_ordinal,
                block,
                channel,
            )
            .await
        }

        BlockchainCommand::PullFromPeer => {
            debug!("Blockchain receives BlockchainCommand::PullFromPeer");
            let start_ordinal: Ordinal = deserialize(&data[1..17])?;
            let end_ordinal: Ordinal = deserialize(&data[17..])?;
            handle_pull_blockchain_from_peer(
                blockchain_service,
                start_ordinal,
                end_ordinal,
                channel,
            )
            .await
        }

        BlockchainCommand::QueryHighestBlockOrdinal => {
            debug!("Blockchain receives BlockchainCommand::QueryHighestBlockOrdinal");
            handle_query_block_ordinal_from_peer(blockchain_service, channel).await
        }

        _ => {
            debug!("Blockchain receives other command");
            todo!()
        }
    }
}

pub async fn handle_broadcast_blockchain(
    mut artifact_service: ArtifactService,
    blockchain_service: Arc<Mutex<BlockchainService>>,
    block_ordinal: Ordinal,
    block: Block,
    channel: ResponseChannel<BlockchainResponse>,
) -> anyhow::Result<()> {
    debug!("Handling broadcast blocks");

    let mut blockchain_service = blockchain_service.lock().await;

    let payloads = block.fetch_payload();
    blockchain_service
        .add_block(block_ordinal, Box::new(block))
        .await?;

    artifact_service.handle_block_added(payloads).await?;

    let response_data = vec![0u8];

    artifact_service
        .p2p_client
        .respond_blockchain(response_data, channel)
        .await
}

pub async fn handle_pull_blockchain_from_peer(
    blockchain_service: Arc<Mutex<BlockchainService>>,
    start_ordinal: Ordinal,
    end_ordinal: Ordinal,
    channel: ResponseChannel<BlockchainResponse>,
) -> anyhow::Result<()> {
    debug!(
        "Handling pull blocks from {:?} to {:?} ",
        start_ordinal, end_ordinal
    );

    let mut blockchain_service = blockchain_service.lock().await;

    match blockchain_service
        .pull_blocks(start_ordinal, end_ordinal)
        .await
    {
        Ok(v) => {
            blockchain_service
                .p2p_client
                .respond_blockchain(serialize(&v).unwrap(), channel)
                .await?
        }
        Err(e) => bail!(e),
    }

    Ok(())
}

pub async fn handle_query_block_ordinal_from_peer(
    blockchain_service: Arc<Mutex<BlockchainService>>,
    channel: ResponseChannel<BlockchainResponse>,
) -> anyhow::Result<()> {
    debug!("Handling query block ordinal");

    let mut blockchain_service = blockchain_service.lock().await;
    let latest_ordinal: Ordinal = match blockchain_service.query_last_block().await {
        Some(v) => v.header.ordinal,
        None => bail!(BlockchainError::InvalidBlockchainLength(0)),
    };

    blockchain_service
        .p2p_client
        .respond_blockchain(serialize(&latest_ordinal).unwrap(), channel)
        .await?;

    Ok(())
}
