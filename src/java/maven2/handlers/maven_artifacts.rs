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
use anyhow::bail;
use log::debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_maven_artifact(
    artifact_service: Arc<Mutex<ArtifactService>>,
    full_path: String,
) -> Result<impl Reply, Rejection> {
    debug!("Requesting maven artifact: {}", full_path);
    let package_type_id = get_package_type_id(&full_path).map_err(|err| {
        debug!("Error getting package type id for artifact: {:?}", err);
        warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        })
    })?;

    // request artifact
    debug!("Requesting artifact for id {}", package_type_id);
    let artifact_content = artifact_service
        .lock()
        .await
        .get_artifact(PackageType::Maven2, &package_type_id)
        .await
        .map_err(|err| {
            debug!("Error retrieving artifact: {:?}", err);
            warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(err.to_string()),
            })
        })?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(artifact_content)
        .unwrap())
}

fn get_package_type_id(full_path: &str) -> Result<String, anyhow::Error> {
    // maven coordinates like "com.company:test:1.0" will produce a request
    // like: "GET /maven2/com/company/test/1.0/test-1.0.jar"

    // split, and remove first two strings: "" and "maven2":
    let mut pieces: Vec<&str> = full_path.split('/').skip(2).collect();
    if pieces.len() < 4 {
        bail!(format!("Error, invalid full path: {}", full_path));
    }
    let file_name = pieces.pop().unwrap();
    let version = pieces.pop().unwrap();
    let artifact_id = pieces.pop().unwrap();
    let group_id = pieces.join(".");

    Ok(format!(
        "{}/{}/{}/{}",
        group_id, artifact_id, version, file_name
    ))
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
    use std::fs::File;
    use std::path::PathBuf;
    use tokio::sync::{mpsc, oneshot};

    const VALID_ARTIFACT_HASH: &str =
        "e11c16ff163ccc1efe01d2696c626891560fa82123601a5ff196d97b6ab156da";
    const VALID_FULL_PATH: &str = "/maven2/test/test/1.0/test-1.0.jar";
    const INVALID_FULL_PATH: &str = "/maven2/test/1.0/test-1.0.jar";
    const VALID_MAVEN_ID: &str = "test/test/1.0/test-1.0.jar";

    #[test]
    fn get_package_type_id_test() {
        assert_eq!(
            get_package_type_id(VALID_FULL_PATH).unwrap(),
            VALID_MAVEN_ID
        );
    }

    #[test]
    fn get_package_type_id_with_invalid_path_test() {
        assert!(get_package_type_id(INVALID_FULL_PATH).is_err());
    }

    #[tokio::test]
    async fn handle_get_maven_artifact_test() {
        let tmp_dir = test_util::tests::setup();

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
                    package_type: PackageType::Maven2,
                    package_type_id: VALID_MAVEN_ID.to_string(),
                    hash: VALID_ARTIFACT_HASH.to_string(),
                },
                add_artifact_sender,
            )
            .await
            .unwrap();
        create_artifact(&artifact_service.artifact_storage).unwrap();

        let result = handle_get_maven_artifact(
            Arc::new(Mutex::new(artifact_service)),
            VALID_FULL_PATH.to_string(),
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
        curr_dir.push("tests/resources/test-1.0.jar");

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }

    fn create_artifact(artifact_storage: &ArtifactStorage) -> Result<(), anyhow::Error> {
        let artifact_hash = hex::decode(VALID_ARTIFACT_HASH)?;
        let hash = Hash::new(HashAlgorithm::SHA256, &artifact_hash)?;
        artifact_storage
            .push_artifact(&mut get_file_reader()?, &hash)
            .context("Error while pushing artifact")
    }
}
