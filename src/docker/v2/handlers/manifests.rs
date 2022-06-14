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

use crate::artifact_service::handlers::get_artifact;
use crate::artifact_service::service::HashAlgorithm;
use crate::artifact_service::storage::ArtifactStorage;
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use crate::network::client::Client;
use crate::transparency_log::log::TransparencyLog;
use log::debug;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

// Handles GET endpoint documented at https://docs.docker.com/registry/spec/api/#manifest
pub async fn fetch_manifest(
    mut transparency_log: TransparencyLog,
    p2p_client: Client,
    artifact_storage: ArtifactStorage,
    name: String,
    tag: String,
) -> Result<impl Reply, Rejection> {
    debug!("Fetching manifest for {} with tag: {}", name, tag);

    //get package_version from metadata
    debug!(
        "Step 1: Does manifest for {} with tag {} exist in the metadata manager?",
        name, tag
    );

    let namespace_specific_id = format!("DOCKER::MANIFEST::{}::{}", name, tag);
    let manifest_artifact_id = transparency_log
        .get_artifact(&namespace_specific_id)
        .map_err(RegistryError::from)?;
    let manifest_content = match manifest_artifact_id {
        Some(manifest_artifact_id) => {
            let decoded_hash = hex::decode(&manifest_artifact_id.get(7..).unwrap()).unwrap();
            get_artifact(
                p2p_client,
                &artifact_storage,
                &decoded_hash,
                HashAlgorithm::SHA512,
            )
            .await
            .map_err(|_| {
                warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::ManifestUnknown,
                })
            })?
        }
        None => {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::ManifestUnknown,
            }));
        }
    };

    Ok(warp::http::response::Builder::new()
        .header(
            "Content-Type",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .header("Content-Length", manifest_content.len())
        .status(StatusCode::OK)
        .body(manifest_content)
        .unwrap())
}
