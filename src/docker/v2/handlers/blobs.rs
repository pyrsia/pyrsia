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

use crate::artifact_service::model::PackageType;
use crate::artifact_service::service::ArtifactService;
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use log::debug;
use std::result::Result;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_blobs(
    _name: String,
    hash: String,
    artifact_service: Arc<Mutex<ArtifactService>>,
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
    use crate::artifact_service::storage::ArtifactStorage;
    use crate::build_service::event::BuildEventClient;
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

    #[tokio::test]
    async fn test_handle_get_blobs_unknown_in_artifact_service() {
        let tmp_dir = test_util::tests::setup();

        let name = "alpine";
        let hash = "7300a197d7deb39371d4683d60f60f2fbbfd7541837ceb2278c12014e94e657b";

        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        let result = handle_get_blobs(
            name.to_owned(),
            hash.to_owned(),
            Arc::new(Mutex::new(artifact_service)),
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

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_handle_get_blobs() {
        let tmp_dir = test_util::tests::setup();

        let name = "alpine";
        let hash = "865c8d988be4669f3e48f73b98f9bc2507be0246ea35e0098cf6054d3644c14f";
        let package_type = PackageType::Docker;
        let package_specific_id = hash;
        let package_specific_artifact_id = hash;

        let (add_artifact_sender, add_artifact_receiver) = oneshot::channel();
        let (command_sender, _command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let mut artifact_service = ArtifactService::new(&tmp_dir, build_event_client, p2p_client)
            .expect("Creating ArtifactService failed");

        artifact_service
            .transparency_log_service
            .add_artifact(
                AddArtifactRequest {
                    package_type,
                    package_specific_id: package_specific_id.to_owned(),
                    num_artifacts: 8,
                    package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                    artifact_hash: hash.to_owned(),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();

        let transparency_log = add_artifact_receiver.await.unwrap().unwrap();

        create_artifact(
            &artifact_service.artifact_storage,
            &transparency_log.artifact_id,
        )
        .unwrap();

        let result = handle_get_blobs(
            name.to_owned(),
            hash.to_owned(),
            Arc::new(Mutex::new(artifact_service)),
        )
        .await;

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

    fn create_artifact(
        artifact_storage: &ArtifactStorage,
        artifact_id: &str,
    ) -> Result<(), anyhow::Error> {
        artifact_storage
            .push_artifact(&mut get_file_reader()?, artifact_id)
            .context("Error while pushing artifact")
    }
}
