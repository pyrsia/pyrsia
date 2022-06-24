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

use crate::artifact_service::service::{ArtifactService, PackageType};
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use log::debug;
use std::result::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_blobs(
    artifact_service: Arc<Mutex<ArtifactService>>,
    hash: String,
) -> Result<impl Reply, Rejection> {
    debug!("Getting blob with hash : {:?}", hash);

    let blob_content = artifact_service
        .lock()
        .await
        .get_artifact(PackageType::Docker, &hash)
        .await
        .map_err(|_| {
            warp::reject::custom(RegistryError {
                code: RegistryErrorCode::BlobUnknown,
            })
        })?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(blob_content.to_vec())
        .unwrap())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_service::hashing::{Hash, HashAlgorithm};
    use crate::artifact_service::storage::ArtifactStorage;
    use crate::network::client::Client;
    use crate::transparency_log::log::AddArtifactRequest;
    use crate::util::test_util;
    use anyhow::Context;
    use hyper::header::HeaderValue;
    use libp2p::identity::Keypair;
    use std::borrow::Borrow;
    use std::fs::File;
    use std::path::PathBuf;
    use tokio::sync::{mpsc, oneshot};

    const VALID_ARTIFACT_HASH: [u8; 32] = [
        0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9, 0xbc,
        0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36, 0x44, 0xc1,
        0x4f,
    ];

    #[tokio::test]
    async fn test_handle_get_blobs_unknown_in_artifact_service() {
        let tmp_dir = test_util::tests::setup();

        let hash = "7300a197d7deb39371d4683d60f60f2fbbfd7541837ceb2278c12014e94e657b";
        let package_type = PackageType::Docker;
        let package_type_id = hash;

        let (add_artifact_sender, _) = oneshot::channel();
        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let mut artifact_service =
            ArtifactService::new(&tmp_dir, p2p_client).expect("Creating ArtifactService failed");

        artifact_service
            .transparency_log
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_type_id: package_type_id.to_string(),
                    hash: hash.to_string(),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();

        let result =
            handle_get_blobs(Arc::new(Mutex::new(artifact_service)), hash.to_string()).await;

        assert!(result.is_err());
        let rejection = result.err().unwrap();
        let registry_error = rejection.find::<RegistryError>().unwrap().borrow();
        assert_eq!(
            *registry_error,
            RegistryError {
                code: RegistryErrorCode::BlobUnknown,
            }
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_handle_get_blobs() {
        let tmp_dir = test_util::tests::setup();

        let hash = "865c8d988be4669f3e48f73b98f9bc2507be0246ea35e0098cf6054d3644c14f";
        let package_type = PackageType::Docker;
        let package_type_id = hash;

        let (add_artifact_sender, _) = oneshot::channel();
        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let mut artifact_service =
            ArtifactService::new(&tmp_dir, p2p_client).expect("Creating ArtifactService failed");
        artifact_service
            .transparency_log
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_type_id: package_type_id.to_string(),
                    hash: hash.to_string(),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();
        create_artifact(&artifact_service.artifact_storage).unwrap();

        let result =
            handle_get_blobs(Arc::new(Mutex::new(artifact_service)), hash.to_string()).await;

        assert!(result.is_ok());

        let response = result.unwrap().into_response();
        assert_eq!(response.status(), StatusCode::OK);
        assert_eq!(
            response.headers().get("Content-Type"),
            Some(&HeaderValue::from_static("application/octet-stream"))
        );

        test_util::tests::teardown(tmp_dir);
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
