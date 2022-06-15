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

use libp2p::core::PeerId;
use libp2p::multiaddr::Protocol;
use libp2p::request_response::ResponseChannel;
use libp2p::Multiaddr;
use log::debug;
use pyrsia::artifact_service;
use pyrsia::artifact_service::service::HashAlgorithm;
use pyrsia::artifact_service::storage::ArtifactStorage;
use pyrsia::network::artifact_protocol::ArtifactResponse;
use pyrsia::network::client::{ArtifactType, Client};
use pyrsia::network::idle_metric_protocol::{IdleMetricResponse, PeerMetrics};

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

/// Respond to a RequestArtifact event by getting the artifact
/// based on the provided artifact type and hash.
pub async fn handle_request_artifact(
    mut p2p_client: Client,
    artifact_storage: ArtifactStorage,
    artifact_type: &ArtifactType,
    artifact_hash: &str,
    channel: ResponseChannel<ArtifactResponse>,
) -> anyhow::Result<()> {
    debug!(
        "Handling request artifact: {:?}={:?}",
        artifact_type, artifact_hash
    );
    let content = match artifact_type {
        ArtifactType::Artifact => {
            get_artifact(p2p_client.clone(), artifact_storage, artifact_hash).await?
        }
    };

    p2p_client.respond_artifact(content, channel).await
}

//Respond to the IdleMetricRequest event
pub async fn handle_request_idle_metric(
    mut p2p_client: Client,
    channel: ResponseChannel<IdleMetricResponse>,
) -> anyhow::Result<()> {
    let metric = artifact_service::handlers::get_quality_metric();
    let peer_metrics: PeerMetrics = PeerMetrics {
        idle_metric: metric.to_le_bytes(),
    };
    p2p_client.respond_idle_metric(peer_metrics, channel).await
}

/// Get the artifact with the provided hash from the artifact manager.
async fn get_artifact(
    p2p_client: Client,
    artifact_storage: ArtifactStorage,
    artifact_hash: &str,
) -> anyhow::Result<Vec<u8>> {
    let decoded_hash = hex::decode(&artifact_hash.get(7..).unwrap()).unwrap();
    artifact_service::handlers::get_artifact(
        p2p_client,
        &artifact_storage,
        &decoded_hash,
        HashAlgorithm::SHA256,
    )
    .await
}
