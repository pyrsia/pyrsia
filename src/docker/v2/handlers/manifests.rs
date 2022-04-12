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

use super::handlers::*;
use super::HashAlgorithm;
use crate::docker::docker_hub_util::get_docker_hub_auth_token;
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use crate::docker::v2::storage::manifests;
use crate::network;
use crate::node_manager::handlers::METADATA_MGR;
use bytes::Bytes;
use libp2p::PeerId;
use log::{debug, error, info};
use reqwest::{header, Client};
use std::fmt::Display;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

// Handles GET endpoint documented at https://docs.docker.com/registry/spec/api/#manifest
pub async fn fetch_manifest(
    mut p2p_client: network::client::Client,
    name: String,
    tag: String,
) -> Result<impl Reply, Rejection> {
    let manifest_content;
    debug!("Fetching manifest for {} with tag: {}", name, tag);

    //get package_version from metadata
    debug!(
        "Step 1: Does {}/{} exist in the metadata manager?",
        name, tag
    );
    match METADATA_MGR.get_package_version(
        crate::node_manager::model::DOCKER_NAMESPACE_ID,
        &name,
        &tag,
    ) {
        Ok(Some(package_version)) => {
            debug!(
                "Step 1: YES, {}/{} exist in the artifact manager.",
                name, tag
            );
            match manifests::get_artifact_manifest(&package_version.artifacts) {
                Some(artifact) => {
                    debug!("Getting manifest from artifact manager.");
                    manifest_content = get_artifact(artifact.hash(), HashAlgorithm::SHA512)
                        .map_err(|_| {
                            warp::reject::custom(RegistryError {
                                code: RegistryErrorCode::ManifestUnknown,
                            })
                        })?;
                }
                None => {
                    //TODO: neeed mechanism in metadata to delete the invalid metadata
                    error!("Bad metadata in local pyrsia. Getting manifest from pyrsia network.");

                    let hash = get_manifest_from_network(p2p_client.clone(), &name, &tag).await?;
                    manifest_content =
                        get_artifact(hex::decode(hash).unwrap().as_ref(), HashAlgorithm::SHA512)
                            .map_err(|_| {
                                warp::reject::custom(RegistryError {
                                    code: RegistryErrorCode::ManifestUnknown,
                                })
                            })?;
                }
            }
        }
        Ok(None) => {
            debug!(
                "Step 1: NO, {}/{} does not exist in the metadata manager.",
                name, tag
            );
            debug!("No package found in local pyrsia. Getting manifest from pyrsia network.");

            let hash = get_manifest_from_network(p2p_client.clone(), &name, &tag).await?;
            manifest_content =
                get_artifact(hex::decode(hash).unwrap().as_ref(), HashAlgorithm::SHA512).map_err(
                    |_| {
                        warp::reject::custom(RegistryError {
                            code: RegistryErrorCode::ManifestUnknown,
                        })
                    },
                )?;
        }
        Err(error) => {
            error!("Error getting manifest from local pyrsia: {}", error);
            debug!(
                "Error getting manifest from local pyrsia. Getting manifest from pyrsia network."
            );

            let hash = get_manifest_from_network(p2p_client.clone(), &name, &tag).await?;
            manifest_content =
                get_artifact(hex::decode(hash).unwrap().as_ref(), HashAlgorithm::SHA512).map_err(
                    |_| {
                        warp::reject::custom(RegistryError {
                            code: RegistryErrorCode::ManifestUnknown,
                        })
                    },
                )?;
        }
    };

    p2p_client.provide(&format!("{}/{}", name, tag)).await;

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

const LOCATION: &str = "Location";

// Handles PUT endpoint documented at https://docs.docker.com/registry/spec/api/#manifest
pub async fn put_manifest(
    name: String,
    reference: String,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    debug!("Storing pushed manifest in artifact manager.");
    let hash = manifests::store_manifest_in_artifact_manager(&name, &reference, &bytes)?;
    put_manifest_response(name, hash)
}

fn put_manifest_response(
    name: String,
    hash: String,
) -> Result<warp::http::Response<&'static str>, Rejection> {
    Ok(
        match warp::http::response::Builder::new()
            .header(
                LOCATION,
                format!(
                    "http://localhost:7878/v2/{}/manifests/sha256:{}",
                    name, hash
                ),
            )
            .header("Docker-Content-Digest", format!("sha256:{}", hash))
            .status(StatusCode::CREATED)
            .body("")
        {
            Ok(response) => response,
            Err(err) => internal_error_response("creating put_manifest response", &err),
        },
    )
}

fn internal_error_response(
    label: &str,
    err: &dyn Display,
) -> warp::http::response::Response<&'static str> {
    error!("Error {}: {}", label, err);
    warp::http::response::Builder::new()
        .status(StatusCode::INTERNAL_SERVER_ERROR)
        .body("Internal server error")
        .unwrap()
    // I couldn't find a way to return an internal server error that does not use unwrap or something else that can panic
}

async fn get_manifest_from_network(
    mut p2p_client: network::client::Client,
    name: &str,
    tag: &str,
) -> Result<String, Rejection> {
    let providers = p2p_client
        .list_providers(&format!("{}/{}", name, tag))
        .await;
    debug!(
        "Step 2: Does {}/{} exist in the Pyrsia network? Providers: {:?}",
        name, tag, providers
    );

    Ok(match providers.iter().next() {
        Some(peer) => {
            debug!(
                "Step 2: YES, {}/{} exists in the Pyrsia network, fetching from peer {}.",
                name, tag, peer
            );
            get_manifest_from_other_peer(p2p_client.clone(), peer, name, tag).await?
        }
        None => {
            debug!(
                "Step 2: No, {}/{} does not exist in the Pyrsia network, fetching from docker.io.",
                name, tag
            );
            get_manifest_from_docker_hub(name, tag).await?
        }
    })
}

// Request the content of the artifact from other peer
async fn get_manifest_from_other_peer(
    mut p2p_client: network::client::Client,
    peer_id: &PeerId,
    name: &str,
    tag: &str,
) -> Result<String, Rejection> {
    info!(
        "Reading manifest from Pyrsia Node {}: {}/{}",
        peer_id, name, tag
    );
    match p2p_client
        .request_artifact(peer_id, &format!("{}/{}", name, tag))
        .await
    {
        Ok(manifest) => {
            debug!("Step 2: YES, {}/{} exists in the Pyrsia network.", name, tag);
            match manifests::store_manifest_in_artifact_manager(name, tag, &bytes::Bytes::from(manifest)) {
                Ok(hash) => {
                    debug!(
                        "Step 2: {}/{} successfully stored locally from Pyrsia network.",
                        name, tag
                    );
                    Ok(hash)
                }
                Err(error) => {
                    debug!("Error while storing manifest in artifact manager: {:?}", error);
                    Err(error)
                }
            }
        }
        Err(err) => {
            debug!(
                "Step 2: Error while retrieving {}/{} from the Pyrsia network from peer {}: {}",
                name, tag, peer_id, err
            );
            get_manifest_from_docker_hub(name, tag).await
        }
    }
}

async fn get_manifest_from_docker_hub(name: &str, tag: &str) -> Result<String, Rejection> {
    debug!("Step 3: Retrieving {}/{} from docker.io", name, tag);
    let token = get_docker_hub_auth_token(name).await?;

    match get_manifest_from_docker_hub_with_token(name, tag, token).await {
        Ok(hash) => {
            debug!(
                "Step 3: {}/{} successfully stored locally from docker.io",
                name, tag
            );
            Ok(hash)
        }
        Err(error) => Err(error),
    }
}

async fn get_manifest_from_docker_hub_with_token(
    name: &str,
    tag: &str,
    token: String,
) -> Result<String, Rejection> {
    let url = format!(
        "https://registry-1.docker.io/v2/library/{}/manifests/{}",
        name, tag
    );

    debug!("Reading manifest from docker.io with url: {}", url);
    let response = Client::new()
        .get(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .header(
            "Accept",
            "application/vnd.docker.distribution.manifest.v2+json",
        )
        .send()
        .await
        .map_err(RegistryError::from)?;

    debug!(
        "Got manifest from docker.io with status {}",
        response.status()
    );

    let bytes = response.bytes().await.map_err(RegistryError::from)?;
    manifests::store_manifest_in_artifact_manager(name, tag, &bytes)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::network::client::command::Command;
    use assay::assay;
    use easy_hasher::easy_hasher::raw_sha512;
    use futures::channel::mpsc;
    use futures::executor;
    use futures::prelude::*;
    use libp2p::identity::Keypair;
    use serde::de::StdError;
    use std::env;
    use std::fs::{self, File};
    use std::io::Read;
    use std::panic;
    use std::path::{Path, PathBuf};
    use warp::http::header::HeaderMap;

    macro_rules! test_async {
        ($e:expr) => {
            tokio_test::block_on($e)
        };
    }

    fn tear_down() {
        if Path::new(&env::var("PYRSIA_ARTIFACT_PATH").unwrap()).exists() {
            fs::remove_dir_all(env::var("PYRSIA_ARTIFACT_PATH").unwrap()).expect(&format!(
                "unable to remove test directory {}",
                env::var("PYRSIA_ARTIFACT_PATH").unwrap()
            ));
        }
    }

    #[assay(
    env = [
      ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-node"),
      ("DEV_MODE", "on")
    ],
    teardown = tear_down()
    )]
    fn test_put_manifest_expecting_success_response_with_manifest_stored_in_artifact_manager_and_package_version_in_metadata_manager(
    ) {
        let name = "httpbin";
        let reference = "v2.4";

        let future = async {
            put_manifest(
                name.to_string(),
                reference.to_string(),
                Bytes::from(manifests::tests::MANIFEST_V1_JSON.as_bytes()),
            )
            .await
        };
        let result = executor::block_on(future);
        verify_put_manifest_result(result);
        check_artifact_manager_side_effects()?;
        check_package_version_metadata()?;
    }

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-node"),
          ("DEV_MODE", "on")
        ],
        teardown = tear_down()
        )]
    #[tokio::test]
    async fn test_fetch_manifest() {
        let name = "httpbin";
        let reference = "v2.4";

        let future = async {
            put_manifest(
                name.to_string(),
                reference.to_string(),
                Bytes::from(manifests::tests::MANIFEST_V1_JSON.as_bytes()),
            )
            .await
        };
        let result = executor::block_on(future);
        verify_put_manifest_result(result);
        check_package_version_metadata()?;

        let (sender, mut receiver) = mpsc::channel(1);
        let client = network::client::Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let future =
            async { fetch_manifest(client, "hello-world".to_string(), "v3.1".to_string()).await };

        tokio::spawn(async move {
            futures::select! {
                command = receiver.next() => match command {
                    Some(Command::ListProviders { hash: _hash, sender }) => {
                        let _ = sender.send(Default::default());
                    },
                    Some(Command::Provide { hash: _hash, sender }) => {
                        let _ = sender.send(());
                    },
                    _ => panic!("Command must match Command::ListProviders")
                }
            }
        });

        let result = executor::block_on(future);
        verify_fetch_manifest_result(result);
    }

    #[test]
    #[ignore]
    fn test_fetch_manifest_if_not_in_pyrsia_expecting_fetch_from_dockerhub_success_and_store_in_pyrsia(
    ) {
        let name = "alpine";
        let reference = "sha256:e7d88de73db3d3fd9b2d63aa7f447a10fd0220b7cbf39803c803f2af9ba256b3";

        assert!(check_manifest_is_stored_in_pyrsia("alpine_manifest.json").is_err());

        let (sender, mut _receiver) = mpsc::channel(1);
        let client = network::client::Client {
            sender,
            local_peer_id: Keypair::generate_ed25519().public().to_peer_id(),
        };

        let result = test_async!(fetch_manifest(
            client,
            name.to_string(),
            reference.to_string()
        ));
        verify_fetch_manifest_result_if_not_in_pyrsia(result);
        assert!(!(check_manifest_is_stored_in_pyrsia("alpine_manifest.json").is_err()));
    }

    fn check_package_version_metadata() -> anyhow::Result<()> {
        let some_package_version = METADATA_MGR.get_package_version(
            crate::node_manager::model::DOCKER_NAMESPACE_ID,
            "hello-world",
            "v3.1",
        )?;
        assert!(some_package_version.is_some());
        assert_eq!("v3.1", some_package_version.unwrap().version);
        Ok(())
    }

    fn verify_fetch_manifest_result_if_not_in_pyrsia(result: Result<impl Reply, Rejection>) {
        match result {
            Ok(reply) => {
                let response = reply.into_response();
                assert_eq!(response.status(), 200);

                let mut headers = HeaderMap::new();
                headers.insert("content-length", "528".parse().unwrap());
                assert_eq!(
                    response.headers().get("content-length").unwrap(),
                    headers["content-length"]
                );
            }
            Err(_) => {
                assert!(false)
            }
        };
    }

    fn verify_fetch_manifest_result(result: Result<impl Reply, Rejection>) {
        match result {
            Ok(reply) => {
                let response = reply.into_response();
                assert_eq!(response.status(), 200);

                let mut headers = HeaderMap::new();
                headers.insert("content-length", "4980".parse().unwrap());
                assert_eq!(
                    response.headers().get("content-length").unwrap(),
                    headers["content-length"]
                );
            }
            Err(_) => {
                assert!(false)
            }
        };
    }

    fn verify_put_manifest_result(result: Result<impl Reply, Rejection>) {
        match result {
            Ok(reply) => {
                let response = reply.into_response();
                assert_eq!(response.status(), 201);
                assert!(response.headers().contains_key(LOCATION));
                assert_eq!("http://localhost:7878/v2/httpbin/manifests/sha256:b258508df30760725a1020d30d38a3e01684af2dde5bb6e942d2a7e0744d4f152a65a187f19cad8a02d125e45b634d962faa32998a18e401631f37a7deb82efd",
                response.headers().get(LOCATION).unwrap());
            }
            Err(_) => {
                assert!(false)
            }
        };
    }

    fn get_test_file_reader(file_name: &str) -> Result<File, anyhow::Error> {
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/");
        curr_dir.push(file_name);

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }

    fn check_artifact_manager_side_effects() -> Result<(), Box<dyn StdError>> {
        let manifest_sha512: Vec<u8> =
            raw_sha512(manifests::tests::MANIFEST_V1_JSON.as_bytes().to_vec()).to_vec();
        let manifest_content = get_artifact(manifest_sha512.as_ref(), HashAlgorithm::SHA512)?;
        assert!(!manifest_content.is_empty());
        assert_eq!(4980, manifest_content.len());
        Ok(())
    }

    fn check_manifest_is_stored_in_pyrsia(file_name: &str) -> Result<Vec<u8>, Box<dyn StdError>> {
        let mut file = get_test_file_reader(file_name)?;
        let mut data = Vec::new();
        file.read_to_end(&mut data).expect("Unable to read data");
        let manifest_sha512: Vec<u8> = raw_sha512(data).to_vec();
        Ok(get_artifact(
            manifest_sha512.as_ref(),
            HashAlgorithm::SHA512,
        )?)
    }
}
