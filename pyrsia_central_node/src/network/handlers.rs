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
use libp2p::request_response::ResponseChannel;
use libp2p::Multiaddr;
use log::{debug, info};
use pyrsia::artifacts_repository::hash_util::HashAlgorithm;
use pyrsia::docker::constants::*;
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
pub async fn provide_artifacts(mut p2p_client: Client) -> anyhow::Result<()> {
    if let Ok(package_versions) = node_manager::handlers::METADATA_MGR.list_package_versions() {
        debug!(
            "Start providing {} package versions",
            package_versions.len()
        );
        for package_version in package_versions.iter() {
            p2p_client
                .provide(ArtifactType::PackageVersion, package_version.into())
                .await?;
        }
    }

    if let Ok(artifact_hashes) = node_manager::handlers::get_artifact_hashes() {
        debug!("Start providing {} artifacts", artifact_hashes.len());
        for artifact_hash in artifact_hashes.iter() {
            p2p_client
                .provide(ArtifactType::Artifact, artifact_hash.into())
                .await?;
        }
    }

    Ok(())
}

/// Respond to a RequestArtifact event by getting the artifact
/// based on the provided artifact type and hash.
pub async fn handle_request_artifact(
    mut p2p_client: Client,
    artifact_type: &ArtifactType,
    artifact_hash: &str,
    channel: ResponseChannel<ArtifactResponse>,
) -> anyhow::Result<()> {
    debug!(
        "Handling request artifact: {:?}={:?}",
        artifact_type, artifact_hash
    );
    let content = match artifact_type {
        ArtifactType::Artifact => get_artifact(artifact_hash)?,
        ArtifactType::PackageVersion => get_package_version(artifact_hash)?,
    };

    p2p_client.respond_artifact(content, channel).await
}

//Respond to the IdleMetricRequest event
pub async fn handle_request_idle_metric(
    mut p2p_client: Client,
    channel: ResponseChannel<IdleMetricResponse>,
) -> anyhow::Result<()> {
    let metric = node_manager::handlers::get_quality_metric();
    let peer_metrics: PeerMetrics = PeerMetrics {
        idle_metric: metric.to_le_bytes(),
    };
    p2p_client.respond_idle_metric(peer_metrics, channel).await
}

/// Get the artifact with the provided hash from the artifact manager.
fn get_artifact(artifact_hash: &str) -> anyhow::Result<Vec<u8>> {
    let decoded_hash = hex::decode(&artifact_hash.get(7..).unwrap()).unwrap();
    node_manager::handlers::get_artifact(&decoded_hash, HashAlgorithm::SHA256)
}

/// Get the artifact from the package version for the provided package
/// version identifier. The identifier is a string that contains the following
/// three components that uniquely identify a package version. Each part is
/// separated by a forward slash (`/`):
///
///  * namespace id
///  * name
///  * version
///
/// This is an example of an identifier: `4658011310974e1bb5c46fd4df7e78b9/alpine/3.15.4`
fn get_package_version(package_version_identifier: &str) -> Result<Vec<u8>, anyhow::Error> {
    let decoded_hash: Vec<&str> = package_version_identifier.split('/').collect();
    if let Some(package_version) = node_manager::handlers::METADATA_MGR.get_package_version(
        decoded_hash[0],
        decoded_hash[1],
        decoded_hash[2],
    )? {
        if let Some(artifact) = package_version.get_artifact_by_mime_type(vec![
            MEDIA_TYPE_SCHEMA_1,
            MEDIA_TYPE_IMAGE_MANIFEST,
            MEDIA_TYPE_MANIFEST_LIST,
        ]) {
            return node_manager::handlers::get_artifact(artifact.hash(), HashAlgorithm::SHA512);
        }
    }

    bail!("Manifest not available on this node")
}
