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

use libp2p::request_response::ResponseChannel;
use libp2p::Multiaddr;
use log::{debug, info};
use pyrsia::artifacts_repository::hash_util::HashAlgorithm;
use pyrsia::network::artifact_protocol::ArtifactResponse;
use pyrsia::network::client::{ArtifactType, Client};
use pyrsia::network::idle_metric_protocol::{IdleMetricResponse, PeerMetrics};
use pyrsia::node_manager;

/// Reach out to another node with the specified address
pub async fn dial_other_peer(mut p2p_client: Client, to_dial: &Multiaddr) {
    p2p_client.dial(to_dial).await.expect("Dial to succeed.");
    info!("Dialed {:?}", to_dial);
}

/// Provide all known artifacts on the p2p network
pub async fn provide_artifacts(mut p2p_client: Client) {
    if let Ok(artifact_hashes) = node_manager::handlers::get_artifact_hashes() {
        debug!("Start providing {} artifacts", artifact_hashes.len());
        for artifact_hash in artifact_hashes.iter() {
            p2p_client
                .provide(ArtifactType::Artifact, artifact_hash.into())
                .await;
        }
    }
}

/// Respond to a RequestArtifact event by getting the artifact from
/// the ArtifactManager.
pub async fn handle_request_artifact(
    mut p2p_client: Client,
    hash: &str,
    channel: ResponseChannel<ArtifactResponse>,
) {
    let decoded_hash = hex::decode(&hash.get(7..).unwrap()).unwrap();
    match node_manager::handlers::get_artifact(&decoded_hash, HashAlgorithm::SHA256) {
        Ok(content) => p2p_client.respond_artifact(content, channel).await,
        Err(e) => info!(
            "This node does not provide artifact {}. Error: {:?}",
            hash, e
        ),
    }
}

//Respond to the IdleMetricRequest event
pub async fn handle_request_idle_metric(
    mut p2p_client: Client,
    channel: ResponseChannel<IdleMetricResponse>,
) {
    let metric = node_manager::handlers::get_quality_metric();
    let peer_metrics: PeerMetrics = PeerMetrics {
        idle_metric: metric.to_le_bytes(),
    };
    p2p_client.respond_idle_metric(peer_metrics, channel).await;
}
