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

use anyhow::anyhow;
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use crate::network::client::Client;
use crate::artifact_service::storage::ArtifactStorage;
use crate::transparency_log::log::TransparencyLog;
use crate::artifact_service::handlers::get_artifact;

use warp::{http::StatusCode, Rejection, Reply};
use log::debug;

pub async fn handle_get_maven_artifact(
    transparency_log: TransparencyLog,
    p2p_client: Client,
    artifact_storage: ArtifactStorage,
    full_path: String
) -> Result<impl Reply, Rejection> {
    debug!("Requesting maven artifact: {}", full_path);
    let namespace_specific_id = get_namespace_specific_id(&full_path)
        .map_err(|_| {
            return warp::reject::custom(RegistryError {
                code: RegistryErrorCode::BlobUnknown,
            });
        })?;

    // request artifact
    debug!("Requesting artifact for id {}", &namespace_specific_id);
    let blob_content = get_artifact(
        transparency_log,
        p2p_client,
        &artifact_storage,
        &namespace_specific_id,
    )
    .await
    .map_err(|err| {
        debug!("Error retrieving artifact: {:?}", err);
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

fn get_namespace_specific_id(full_path:&str) -> Result<String, anyhow::Error> {
    // maven coordinates like "com.company:test:1.0" will produce a request
    // like: "GET /maven2/com/company/test/1.0/test-1.0.jar"

    // split, and remove first two strings: "" and "maven2":
    let mut pieces: Vec<&str> = full_path.split("/").skip(2).collect();
    if pieces.len() < 4 {
        return Err(anyhow!(format!("Error, invalid full path: {}", full_path)));
    }
    let file_name = pieces.pop().unwrap();
    let version = pieces.pop().unwrap();
    let artifact_id = pieces.pop().unwrap();
    let group_id = pieces.join(".");

    Ok(format!("MAVEN2/FILE/{}/{}/{}/{}", group_id, artifact_id, version, file_name))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_service::service::{Hash, HashAlgorithm};
    use crate::util::test_util;
    use anyhow::Context;
    use assay::assay;
    use futures::channel::mpsc;
    use libp2p::identity::Keypair;
    use hyper::header::HeaderValue;
    use std::fs::File;
    use std::path::PathBuf;

    const VALID_ARTIFACT_HASH: &str = "e11c16ff163ccc1efe01d2696c626891560fa82123601a5ff196d97b6ab156da";
    const VALID_FULL_PATH: &str = "/maven2/test/test/1.0/test-1.0.jar";
    const VALID_MAVEN_ID: &str = "MAVEN2/FILE/test/test/1.0/test-1.0.jar";

    #[test]
    fn test_get_namespace_specific_id() {
        assert_eq!(
            get_namespace_specific_id(VALID_FULL_PATH).unwrap(),
            VALID_MAVEN_ID
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
    async fn test_handle_test_maven_artifact() {
        let mut transparency_log = TransparencyLog::new();
        transparency_log.add_artifact(VALID_MAVEN_ID, VALID_ARTIFACT_HASH)?;

        let (sender, _) = mpsc::channel(1);
        let p2p_client = Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let artifact_storage = ArtifactStorage::new()?;
        create_artifact(&artifact_storage)?;

        let result = handle_get_maven_artifact(
            transparency_log.clone(),
            p2p_client.clone(),
            artifact_storage.clone(),
            VALID_FULL_PATH.to_string(),
        ).await;

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
