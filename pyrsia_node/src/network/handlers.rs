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

use bincode::deserialize;
use libp2p::multiaddr::Protocol;
use libp2p::request_response::ResponseChannel;
use libp2p::{Multiaddr, PeerId};
use log::debug;

use pyrsia::artifact_service::service::ArtifactService;
use pyrsia::blockchain_service::service::{BlockchainCommand, BlockchainService};
use pyrsia::network::artifact_protocol::ArtifactResponse;
use pyrsia::network::blockchain_protocol::BlockchainResponse;
use pyrsia::network::client::Client;
use pyrsia::network::idle_metric_protocol::{IdleMetricResponse, PeerMetrics};
use pyrsia::peer_metrics;
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
    artifact_service: Arc<Mutex<ArtifactService>>,
    artifact_id: &str,
    channel: ResponseChannel<ArtifactResponse>,
) -> anyhow::Result<()> {
    debug!("Handling request artifact: {:?}", artifact_id);

    let mut artifact_service = artifact_service.lock().await;
    let content = artifact_service.get_artifact_locally(artifact_id).await?;

    artifact_service
        .p2p_client
        .respond_artifact(content, channel)
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

pub async fn handle_request_blockchain(
    artifact_service: Arc<Mutex<ArtifactService>>,
    data: Vec<u8>,
    channel: ResponseChannel<BlockchainResponse>,
) -> anyhow::Result<()> {
    debug!("Handling request blockchain: {:?}", data);

    let mut artifact_service = artifact_service.lock().await;

    let payloads = block.fetch_payload();
    artifact_service
        .blockchain_service
        .add_block(block_ordinal, block)
        .await;
    artifact_service.handle_block_added(payloads).await?;

    let cmd: BlockchainCommand = BlockchainCommand::try_from(data[0])?;

    let block_ordinal: Ordinal = deserialize(&data[1..=9])?;
    let block: Box<Block> = Box::new(deserialize(&data[10..])?);
    blockchain_service.add_block(block_ordinal, block).await;
    artifact_service..p2p_client.respond_blockchain().await
}
