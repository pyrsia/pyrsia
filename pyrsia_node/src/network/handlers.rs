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
    artifact_type: ArtifactType,
    artifact_hash: &str,
    channel: ResponseChannel<ArtifactResponse>,
) {
    debug!(
        "Handling request artifact: {:?}={:?}",
        artifact_type, artifact_hash
    );
    let content = match artifact_type {
        ArtifactType::Artifact => get_artifact(artifact_hash),
        ArtifactType::PackageVersion => get_package_version(artifact_hash),
    };

    match content {
        Ok(content) => p2p_client.respond_artifact(content, channel).await,
        Err(error) => info!(
            "This node does not provide artifact with type {} and hash {}. Error: {:?}",
            artifact_type, artifact_hash, error
        ),
    }
}

fn get_artifact(artifact_hash: &str) -> anyhow::Result<Vec<u8>> {
    let decoded_hash = hex::decode(&artifact_hash.get(7..).unwrap()).unwrap();
    node_manager::handlers::get_artifact(&decoded_hash, HashAlgorithm::SHA256)
}

fn get_package_version(artifact_hash: &str) -> Result<Vec<u8>, anyhow::Error> {
    let decoded_hash: Vec<&str> = artifact_hash.split('/').collect();
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
