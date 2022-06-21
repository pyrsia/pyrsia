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
use crate::artifact_service::storage::ArtifactStorage;
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use crate::network::client::Client;
use crate::transparency_log::log::TransparencyLog;
use log::debug;
use std::result::Result;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_blobs(
    transparency_log: TransparencyLog,
    p2p_client: Client,
    artifact_storage: ArtifactStorage,
    hash: String,
) -> Result<impl Reply, Rejection> {
    debug!("Getting blob with hash : {:?}", hash);

    let blob_content = get_artifact(
        transparency_log,
        p2p_client,
        &artifact_storage,
        &get_namespace_specific_id(&hash),
    )
    .await
    .map_err(|_| {
        warp::reject::custom(RegistryError {
            code: RegistryErrorCode::BlobUnknown,
        })
    })?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(blob_content)
        .unwrap())
}

fn get_namespace_specific_id(hash: &str) -> String {
    format!("DOCKER::BLOB::{}", hash)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_service::service::{Hash, HashAlgorithm};
    use crate::util::test_util;
    use anyhow::Context;
    use assay::assay;
    use futures::channel::mpsc;
    use hyper::header::HeaderValue;
    use libp2p::identity::Keypair;
    use std::borrow::Borrow;
    use std::fs::File;
    use std::path::PathBuf;

    const VALID_ARTIFACT_HASH: [u8; 32] = [
        0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9, 0xbc,
        0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36, 0x44, 0xc1,
        0x4f,
    ];

    #[test]
    fn test_get_namespace_specific_id() {
        let hash = "hash";

        assert_eq!(
            get_namespace_specific_id(hash),
            format!("DOCKER::BLOB::{}", hash)
        );
    }

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-node"),
          ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    #[tokio::test]
    async fn test_handle_get_blobs_unknown_in_artifact_service() {
        let hash = "7300a197d7deb39371d4683d60f60f2fbbfd7541837ceb2278c12014e94e657b";
        let namespace_specific_id = format!("DOCKER::BLOB::{}", hash);

        let mut transparency_log = TransparencyLog::new();
        transparency_log.add_artifact(&namespace_specific_id, hash)?;

        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let artifact_storage = ArtifactStorage::new()?;

        let result = handle_get_blobs(
            transparency_log,
            p2p_client,
            artifact_storage,
            hash.to_string(),
        )
        .await;

        assert!(result.is_err());
        let rejection = result.err().unwrap();
        let registry_error = rejection.find::<RegistryError>().unwrap().borrow();
        assert_eq!(
            *registry_error,
            RegistryError {
                code: RegistryErrorCode::BlobUnknown,
            }
        );
    }

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-node"),
          ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    #[tokio::test]
    async fn test_handle_get_blobs() {
        let hash = "865c8d988be4669f3e48f73b98f9bc2507be0246ea35e0098cf6054d3644c14f";
        let namespace_specific_id = format!("DOCKER::BLOB::{}", hash);

        let mut transparency_log = TransparencyLog::new();
        transparency_log.add_artifact(&namespace_specific_id, hash)?;

        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let artifact_storage = ArtifactStorage::new()?;
        create_artifact(&artifact_storage)?;

        let result = handle_get_blobs(
            transparency_log,
            p2p_client,
            artifact_storage,
            hash.to_string(),
        )
        .await;

        assert!(result.is_ok());

        let response = result.unwrap().into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("Content-Type"),
            Some(&HeaderValue::from_static("application/octet-stream"))
        );
    }

    fn get_file_reader() -> Result<File, anyhow::Error> {
        // test artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/artifact_test.json");

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }

    fn create_artifact(artifact_storage: &ArtifactStorage) -> Result<(), anyhow::Error> {
        let hash = Hash::new(HashAlgorithm::SHA256, &VALID_ARTIFACT_HASH)?;
        artifact_storage
            .push_artifact(&mut get_file_reader()?, &hash)
            .context("Error while pushing artifact")
    }
}
