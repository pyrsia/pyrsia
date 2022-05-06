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
use crate::docker::v2::storage::*;
use crate::network::client::{ArtifactType, Client};
use bytes::Bytes;
use libp2p::PeerId;
use log::{debug, info, trace};
use reqwest::header;
use std::collections::HashMap;
use std::result::Result;
use std::str;
use uuid::Uuid;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_blobs(
    mut p2p_client: Client,
    name: String,
    hash: String,
) -> Result<impl Reply, Rejection> {
    debug!("Getting blob with hash : {:?}", hash);
    let blob_content;

    debug!("Step 1: Does {:?} exist in the artifact manager?", hash);
    let decoded_hash = hex::decode(&hash.get(7..).unwrap()).unwrap();
    match get_artifact(&decoded_hash, HashAlgorithm::SHA256) {
        Ok(blob) => {
            debug!("Step 1: YES, {:?} exist in the artifact manager.", hash);
            blob_content = blob;
        }
        Err(_) => {
            debug!(
                "Step 1: NO, {:?} does not exist in the artifact manager.",
                hash
            );

            get_blob_from_network(p2p_client.clone(), &name, &hash).await?;
            blob_content = get_artifact(&decoded_hash, HashAlgorithm::SHA256).map_err(|_| {
                warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::BlobUnknown,
                })
            })?;
        }
    }

    p2p_client
        .provide(ArtifactType::Artifact, hash.clone().into())
        .await;

    debug!("Final Step: {:?} successfully retrieved!", hash);
    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(blob_content)
        .unwrap())
}

pub async fn handle_post_blob(name: String) -> Result<impl Reply, Rejection> {
    let id = Uuid::new_v4();

    trace!(
        "Getting ready to start new upload for {} - {}",
        name,
        id.to_string()
    );

    blobs::create_upload_directory(&name, &id.to_string()).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header(
            "Location",
            format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, id),
        )
        .header("Range", "0-0")
        .status(StatusCode::ACCEPTED)
        .body("")
        .unwrap())
}

pub async fn handle_patch_blob(
    name: String,
    id: String,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    let blob_upload_dest = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data",
        name, id
    );

    let append = blobs::append_to_blob(&blob_upload_dest, bytes).map_err(RegistryError::from)?;

    let range = format!("{}-{}", append.0, append.0 + append.1 - 1);
    debug!("Patch blob range: {}", range);

    Ok(warp::http::response::Builder::new()
        .header(
            "Location",
            format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, id),
        )
        .header("Range", &range)
        .status(StatusCode::ACCEPTED)
        .body("")
        .unwrap())
}

pub async fn handle_put_blob(
    name: String,
    id: String,
    params: HashMap<String, String>,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    let digest = params.get("digest").ok_or(RegistryError {
        code: RegistryErrorCode::Unknown(String::from("missing digest")),
    })?;

    blobs::store_blob_in_filesystem(&name, &id, digest, bytes).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header(
            "Location",
            format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, digest),
        )
        .status(StatusCode::CREATED)
        .body("")
        .unwrap())
}

// Request the content of the artifact from the pyrsia network
async fn get_blob_from_network(
    mut p2p_client: Client,
    name: &str,
    hash: &str,
) -> Result<(), RegistryError> {
    let providers = p2p_client
        .list_providers(ArtifactType::Artifact, hash.into())
        .await;
    debug!(
        "Step 2: Does {:?} exist in the Pyrsia network? Providers: {:?}",
        hash, providers
    );

    match p2p_client.get_idle_peer(providers).await {
        Some(peer) => {
            debug!(
                "Step 2: YES, {:?} exists in the Pyrsia network, fetching from peer {:?}.",
                hash, peer
            );
            if get_blob_from_other_peer(p2p_client.clone(), &peer, name, hash)
                .await
                .is_err()
            {
                get_blob_from_docker_hub(name, hash).await?
            }
        }
        None => {
            debug!(
                "Step 2: No, {:?} does not exist in the Pyrsia network, fetching from docker.io.",
                hash
            );
            get_blob_from_docker_hub(name, hash).await?
        }
    }

    Ok(())
}

// Request the content of the artifact from other peer
async fn get_blob_from_other_peer(
    mut p2p_client: Client,
    peer_id: &PeerId,
    name: &str,
    hash: &str,
) -> Result<(), RegistryError> {
    info!(
        "Reading blob from Pyrsia Node {}: {}",
        peer_id,
        hash.get(7..).unwrap()
    );
    match p2p_client
        .request_artifact(peer_id, ArtifactType::Artifact, hash.into())
        .await
    {
        Ok(artifact) => {
            let id = Uuid::new_v4();
            debug!("Step 2: YES, {:?} exists in the Pyrsia network.", hash);

            blobs::create_upload_directory(name, &id.to_string()).map_err(RegistryError::from)?;
            blobs::store_blob_in_filesystem(
                name,
                &id.to_string(),
                hash,
                bytes::Bytes::from(artifact),
            )
            .map_err(RegistryError::from)?;
            debug!(
                "Step 2: {:?} successfully stored locally from Pyrsia network.",
                hash
            );
            Ok(())
        }
        Err(error) => {
            debug!(
                "Step 2: Error while retrieving {:?} from the Pyrsia network from peer {}: {}",
                hash, peer_id, error
            );
            Err(RegistryError::from(error))
        }
    }
}

async fn get_blob_from_docker_hub(name: &str, hash: &str) -> Result<(), RegistryError> {
    debug!("Step 3: Retrieving {:?} from docker.io", hash);
    let token = get_docker_hub_auth_token(name).await?;

    get_blob_from_docker_hub_with_token(name, hash, token).await?;
    debug!(
        "Step 3: {:?} successfully stored locally from docker.io",
        hash
    );
    Ok(())
}

async fn get_blob_from_docker_hub_with_token(
    name: &str,
    hash: &str,
    token: String,
) -> Result<(), RegistryError> {
    let url = format!(
        "https://registry-1.docker.io/v2/library/{}/blobs/{}",
        name, hash
    );
    debug!("Reading blob from docker.io with url: {}", url);
    let response = reqwest::Client::new()
        .get(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .map_err(RegistryError::from)?;

    debug!("Got blob from docker.io with status {}", response.status());
    let bytes = response.bytes().await.map_err(RegistryError::from)?;

    let id = Uuid::new_v4();

    blobs::create_upload_directory(name, &id.to_string()).map_err(RegistryError::from)?;
    blobs::store_blob_in_filesystem(name, &id.to_string(), hash, bytes).map_err(RegistryError::from)
}
