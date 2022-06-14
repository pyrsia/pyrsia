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
use crate::network::client::{ArtifactType, Client};
use crate::transparency_log::log::TransparencyLog;
use log::debug;
use std::result::Result;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_blobs(
    mut transparency_log: TransparencyLog,
    mut p2p_client: Client,
    artifact_storage: ArtifactStorage,
    hash: String,
) -> Result<impl Reply, Rejection> {
    debug!("Getting blob with hash : {:?}", hash);

    let namespace_specific_id = format!("DOCKER::BLOB::{}", hash);
    let blob_artifact_id = transparency_log
        .get_artifact(&namespace_specific_id)
        .map_err(RegistryError::from)?;
    let blob_content = match blob_artifact_id {
        Some(blob_artifact_id) => {
            let decoded_hash = hex::decode(&blob_artifact_id.get(7..).unwrap()).unwrap();
            get_artifact(
                p2p_client.clone(),
                &artifact_storage,
                &decoded_hash,
                HashAlgorithm::SHA256,
            )
            .await
            .map_err(|_| {
                warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::BlobUnknown,
                })
            })?
        }
        None => {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::BlobUnknown,
            }));
        }
    };

    p2p_client
        .provide(ArtifactType::Artifact, hash.clone().into())
        .await
        .map_err(RegistryError::from)?;

    debug!("Final Step: {:?} successfully retrieved!", hash);
    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(blob_content)
        .unwrap())
}
