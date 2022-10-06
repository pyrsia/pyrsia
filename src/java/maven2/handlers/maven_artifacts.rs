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
use anyhow::bail;
use log::debug;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_maven_artifact(
    full_path: String,
    mut artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    debug!("Requesting maven artifact: {}", full_path);
    let package_specific_artifact_id =
        get_package_specific_artifact_id(&full_path).map_err(|err| {
            debug!(
                "Error getting package specific artifact id for artifact: {:?}",
                err
            );
            warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(err.to_string()),
            })
        })?;

    // request artifact
    debug!(
        "Requesting artifact for id {}",
        package_specific_artifact_id
    );
    let artifact_content = artifact_service
        .get_artifact(PackageType::Maven2, &package_specific_artifact_id)
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

fn get_package_specific_artifact_id(full_path: &str) -> Result<String, anyhow::Error> {
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
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::artifact_service::storage::ArtifactStorage;
    use crate::blockchain_service::service::BlockchainService;
    use crate::build_service::event::BuildEventClient;
    use crate::network::client::command::Command;
    use crate::network::client::Client;
    use crate::transparency_log::log::{AddArtifactRequest, TransparencyLogService};
    use crate::util::test_util;
    use anyhow::Context;
    use hyper::header::HeaderValue;
    use libp2p::identity::Keypair;
    use std::collections::HashSet;
    use std::fs::File;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    const VALID_ARTIFACT_HASH: &str =
        "e11c16ff163ccc1efe01d2696c626891560fa82123601a5ff196d97b6ab156da";
    const VALID_FULL_PATH: &str = "/maven2/test/test/1.0/test-1.0.jar";
    const INVALID_FULL_PATH: &str = "/maven2/test/1.0/test-1.0.jar";
    const VALID_MAVEN_ID: &str = "test:test:1.0";
    const VALID_MAVEN_ARTIFACT_ID: &str = "test/test/1.0/test-1.0.jar";

    fn create_p2p_client(local_keypair: &Keypair) -> (mpsc::Receiver<Command>, Client) {
        let (command_sender, command_receiver) = mpsc::channel(1);
        let p2p_client = Client {
            sender: command_sender,
            local_peer_id: local_keypair.public().to_peer_id(),
        };

        (command_receiver, p2p_client)
    }

    async fn create_blockchain_service(
        local_keypair: &Keypair,
        p2p_client: Client,
        blockchain_path: impl AsRef<Path>,
    ) -> BlockchainService {
        let ed25519_keypair = match local_keypair {
            libp2p::identity::Keypair::Ed25519(ref v) => v,
            _ => {
                panic!("Keypair Format Error");
            }
        };

        BlockchainService::init_first_blockchain_node(
            ed25519_keypair,
            ed25519_keypair,
            p2p_client,
            blockchain_path,
        )
        .await
        .expect("Creating BlockchainService failed")
    }

    async fn create_transparency_log_service(
        artifact_path: impl AsRef<Path>,
        local_keypair: Keypair,
        p2p_client: Client,
    ) -> TransparencyLogService {
        let blockchain_service =
            create_blockchain_service(&local_keypair, p2p_client, &artifact_path).await;

        TransparencyLogService::new(artifact_path, Arc::new(Mutex::new(blockchain_service)))
            .expect("Creating TransparencyLogService failed")
    }

    #[test]
    fn get_package_specific_artifact_id_test() {
        assert_eq!(
            get_package_specific_artifact_id(VALID_FULL_PATH).unwrap(),
            VALID_MAVEN_ARTIFACT_ID
        );
    }

    #[test]
    fn get_package_specific_artifact_id_with_invalid_path_test() {
        assert!(get_package_specific_artifact_id(INVALID_FULL_PATH).is_err());
    }

    #[tokio::test]
    async fn handle_get_maven_artifact_test() {
        let tmp_dir = test_util::tests::setup();

        let local_keypair = Keypair::generate_ed25519();
        let (mut command_receiver, p2p_client) = create_p2p_client(&local_keypair);
        let transparency_log_service =
            create_transparency_log_service(&tmp_dir, local_keypair, p2p_client.clone()).await;

        tokio::spawn(async move {
            loop {
                match command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);
        let build_event_client = BuildEventClient::new(build_event_sender);
        let mut artifact_service = ArtifactService::new(
            &tmp_dir,
            transparency_log_service,
            build_event_client,
            p2p_client,
        )
        .expect("Creating ArtifactService failed");

        let transparency_log = artifact_service
            .transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type: PackageType::Maven2,
                package_specific_id: VALID_MAVEN_ID.to_owned(),
                num_artifacts: 8,
                package_specific_artifact_id: VALID_MAVEN_ARTIFACT_ID.to_owned(),
                artifact_hash: VALID_ARTIFACT_HASH.to_owned(),
            })
            .await
            .unwrap();
        artifact_service
            .transparency_log_service
            .write_transparency_log(&transparency_log)
            .unwrap();

        create_artifact(
            &artifact_service.artifact_storage,
            &transparency_log.artifact_id,
        )
        .unwrap();

        let result = handle_get_maven_artifact(VALID_FULL_PATH.to_string(), artifact_service).await;

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

    fn create_artifact(
        artifact_storage: &ArtifactStorage,
        artifact_id: &str,
    ) -> Result<(), anyhow::Error> {
        artifact_storage
            .push_artifact(&mut get_file_reader()?, artifact_id)
            .context("Error while pushing artifact")
    }
}
