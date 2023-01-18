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
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_blobs(
    name: String,
    digest: String,
    mut artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    debug!(
        "Getting blob with digest: {:?}. If not found, a build will be requested",
        &get_package_specific_artifact_id(&name, &digest)
    );

    let blob_content = artifact_service
        .get_artifact_or_build(
            PackageType::Docker,
            &get_package_specific_artifact_id(&name, &digest),
            &get_package_specific_artifact_id(&name, &digest),
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
        .body(blob_content.to_vec())
        .unwrap())
}

fn get_package_specific_artifact_id(name: &str, digest: &str) -> String {
    let combined_tag = format!("{}@{}", name, digest);
    if combined_tag.contains('/') {
        combined_tag
    } else {
        format!("library/{}", combined_tag)
    }
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
    use std::borrow::Borrow;
    use std::collections::HashSet;
    use std::fs::File;
    use std::path::PathBuf;

    #[test]
    fn test_get_package_specific_artifact_id_from_digest() {
        let name = "library/alpine";
        let tag = "sha256:1e014f84205d569a5cc3be4e108ca614055f7e21d11928946113ab3f36054801";

        assert_eq!(
            get_package_specific_artifact_id(name, tag),
            format!("{}@{}", name, tag)
        );
    }

    #[test]
    fn test_get_package_specific_artifact_id_as_official_image_name_from_digest() {
        let name = "alpine";
        let official_image_name = "library/alpine";
        let tag = "sha256:1e014f84205d569a5cc3be4e108ca614055f7e21d11928946113ab3f36054801";

        assert_eq!(
            get_package_specific_artifact_id(name, tag),
            format!("{}@{}", official_image_name, tag)
        );
    }

    #[tokio::test]
    async fn test_handle_get_blobs_unknown_in_artifact_service() {
        let tmp_dir = test_util::tests::setup();

        let name = "alpine";
        let hash = "7300a197d7deb39371d4683d60f60f2fbbfd7541837ceb2278c12014e94e657b";

        let (artifact_service, ..) = test_util::tests::create_artifact_service(&tmp_dir);

        let result = handle_get_blobs(name.to_owned(), hash.to_owned(), artifact_service).await;

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
        let digest = format!("sha256:{}", hash);
        let package_type = PackageType::Docker;
        let package_specific_id = format!("{}:latest", name);
        let package_specific_artifact_id = get_package_specific_artifact_id(name, &digest);

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
                package_type,
                package_specific_id,
                num_artifacts: 8,
                package_specific_artifact_id,
                artifact_hash: hash.to_owned(),
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

        let result = handle_get_blobs(name.to_owned(), digest, artifact_service).await;

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
