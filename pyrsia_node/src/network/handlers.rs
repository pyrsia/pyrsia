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

use futures::lock::Mutex;
use libp2p::request_response::ResponseChannel;
use libp2p::Multiaddr;
use log::{debug, info};
use pyrsia::artifact_service::service::ArtifactService;
use pyrsia::network::artifact_protocol::ArtifactResponse;
use pyrsia::network::client::{ArtifactType, Client};
use pyrsia::network::idle_metric_protocol::{IdleMetricResponse, PeerMetrics};
use pyrsia::peer_metrics;
use std::sync::Arc;

/// Reach out to another node with the specified address
pub async fn dial_other_peer(mut p2p_client: Client, to_dial: &Multiaddr) {
    p2p_client.dial(to_dial).await.expect("Dial to succeed.");
    info!("Dialed {:?}", to_dial);
}

/// Respond to a RequestArtifact event by getting the artifact
/// based on the provided artifact type and hash.
pub async fn handle_request_artifact(
    artifact_service: Arc<Mutex<ArtifactService>>,
    artifact_type: &ArtifactType,
    artifact_id: &str,
    channel: ResponseChannel<ArtifactResponse>,
) -> anyhow::Result<()> {
    debug!(
        "Handling request artifact: {:?}={:?}",
        artifact_type, artifact_id
    );
    let content = match artifact_type {
        ArtifactType::Artifact => artifact_service
            .lock()
            .await
            .get_artifact_locally(artifact_id)?,
    };

    artifact_service
        .lock()
        .await
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
