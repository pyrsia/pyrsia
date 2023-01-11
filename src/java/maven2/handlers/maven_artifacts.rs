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
use anyhow::{anyhow, bail};
use log::debug;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_maven_artifact(
    full_path: String,
    mut artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    debug!("Requesting maven artifact: {}", full_path);
    let package_specific_id = get_package_specific_id(&full_path).map_err(|err| {
        debug!("Error getting package specific id for artifact: {:?}", err);
        warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(err.to_string()),
        })
    })?;

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
        "Requesting artifact with package specific id: {}, and package specific artifact id: {}. If not found a build will be requested",
        package_specific_id, package_specific_artifact_id
    );

    let artifact_content = artifact_service
        .get_artifact_or_build(
            PackageType::Maven2,
            &package_specific_id,
            &package_specific_artifact_id,
        )
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

fn get_package_specific_id(full_path: &str) -> Result<String, anyhow::Error> {
    let (group_id, version, artifact_id, _file_name) = parse_artifact_from_full_path(full_path)?;
    Ok(format!("{}:{}:{}", group_id, artifact_id, version))
}

fn get_package_specific_artifact_id(full_path: &str) -> Result<String, anyhow::Error> {
    let (group_id, version, artifact_id, file_name) = parse_artifact_from_full_path(full_path)?;
    Ok(format!(
        "{}/{}/{}/{}",
        group_id, artifact_id, version, file_name
    ))
}

fn parse_artifact_from_full_path(
    full_path: &str,
) -> Result<(String, String, String, String), anyhow::Error> {
    // maven coordinates like "com.company:test:1.0" will produce a request
    // like: "GET /maven2/com/company/test/1.0/test-1.0.jar"

    // split, and remove first two strings: "" and "maven2":
    let mut pieces: Vec<&str> = full_path.split('/').skip(2).collect();
    if pieces.len() < 4 {
        bail!(format!("Error, invalid full path: {}", full_path));
    }
    let file_name = pieces
        .pop()
        .ok_or_else(|| anyhow!("Error extracting the file name"))?
        .to_string();
    let version = pieces
        .pop()
        .ok_or_else(|| anyhow!("Error extracting the version"))?
        .to_string();
    let artifact_id = pieces
        .pop()
        .ok_or_else(|| anyhow!("Error extracting the artifact id"))?
        .to_string();
    let group_id = pieces.join(".");

    Ok((group_id, version, artifact_id, file_name))
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::artifact_service::storage::ArtifactStorage;
    use crate::blockchain_service::event::BlockchainEvent;
    use crate::network::client::command::Command;
    use crate::transparency_log::log::AddArtifactRequest;
    use crate::util::test_util;
    use anyhow::Context;
    use hyper::header::HeaderValue;
    use std::collections::HashSet;
    use std::fs::File;
    use std::path::PathBuf;

    const VALID_ARTIFACT_HASH: &str =
        "e11c16ff163ccc1efe01d2696c626891560fa82123601a5ff196d97b6ab156da";
    const VALID_FULL_PATH: &str = "/maven2/test/test/1.0/test-1.0.jar";
    const INVALID_FULL_PATH: &str = "/maven2/test/1.0/test-1.0.jar";
    const VALID_MAVEN_ID: &str = "test:test:1.0";
    const VALID_MAVEN_ARTIFACT_ID: &str = "test/test/1.0/test-1.0.jar";

    #[test]
    fn parse_full_path_test() {
        let (group_id, version, artifact_id, file_name) =
            parse_artifact_from_full_path(VALID_FULL_PATH).unwrap();
        assert_eq!(group_id, "test");
        assert_eq!(artifact_id, "test");
        assert_eq!(version, "1.0");
        assert_eq!(file_name, "test-1.0.jar");
    }

    #[test]
    fn get_package_specific_id_test() {
        assert_eq!(
            get_package_specific_id(VALID_FULL_PATH).unwrap(),
            VALID_MAVEN_ID
        );
    }

    #[test]
    fn get_package_specific_id_with_invalid_path_test() {
        assert!(get_package_specific_id(INVALID_FULL_PATH).is_err());
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

        let (mut artifact_service, mut blockchain_event_receiver, _, mut p2p_command_receiver) =
            test_util::tests::create_artifact_service(&tmp_dir);

        tokio::spawn(async move {
            loop {
                match blockchain_event_receiver.recv().await {
                    Some(BlockchainEvent::AddBlock { sender, .. }) => {
                        let _ = sender.send(Ok(()));
                    }
                    _ => panic!("BlockchainEvent must match BlockchainEvent::AddBlock"),
                }
            }
        });

        tokio::spawn(async move {
            loop {
                match p2p_command_receiver.recv().await {
                    Some(Command::ListPeers { sender, .. }) => {
                        let _ = sender.send(HashSet::new());
                    }
                    _ => panic!("Command must match Command::ListPeers"),
                }
            }
        });

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
