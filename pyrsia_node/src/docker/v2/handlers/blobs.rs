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

use crate::docker::docker_hub_util::get_docker_hub_auth_token;
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use bytes::{Buf, Bytes};
use log::{debug, error, info};
use reqwest::{header, Client};
use std::collections::HashMap;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

pub async fn handle_get_blobs(
    tx: tokio::sync::mpsc::Sender<String>,
    _name: String,
    hash: String,
) -> Result<impl Reply, Rejection> {
    let blob = format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data",
        hash.get(7..9).unwrap(),
        hash.get(7..).unwrap()
    );
    debug!("Searching for blob: {}", blob);
    let blob_path = Path::new(&blob);
    if !blob_path.exists() {
        if let Err(_) = get_blob_from_docker_hub(&_name, &hash).await {
            let mut send_message: String = "get_blobs | ".to_owned();
            let hash_clone: String = hash.clone();
            send_message.push_str(&hash_clone);

            match tx.send(send_message).await {
                Ok(_) => debug!("hash sent"),
                Err(_) => error!("failed to send stdin input"),
            }
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::BlobDoesNotExist(hash),
            }));
        }
    }

    if !blob_path.is_file() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown("ITS_NOT_A_FILE".to_string()),
        }));
    }

    info!("Reading blob from local Pyrsia storage: {}", blob);
    let content = fs::read(blob_path).map_err(|_| {
        warp::reject::custom(RegistryError {
            code: RegistryErrorCode::BlobUnknown,
        })
    })?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(content)
        .unwrap())
}

pub async fn handle_post_blob(name: String) -> Result<impl Reply, Rejection> {
    let id = Uuid::new_v4();

    create_upload_directory(&name, &id.to_string()).map_err(RegistryError::from)?;

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
    let mut blob_upload_dest = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}/data",
        name, id
    );

    let append = append_to_blob(&mut blob_upload_dest, bytes).map_err(RegistryError::from)?;

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

    store_blob_in_filesystem(&name, &id, digest, bytes).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header(
            "Location",
            format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, digest),
        )
        .status(StatusCode::CREATED)
        .body("")
        .unwrap())
}

pub fn append_to_blob(blob: &str, mut bytes: Bytes) -> std::io::Result<(u64, u64)> {
    debug!("Patching blob: {}", blob);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(blob)?;
    let mut total_bytes_read: u64 = 0;
    let initial_file_length: u64;
    initial_file_length = file.metadata()?.len();
    while bytes.has_remaining() {
        let bytes_remaining = bytes.remaining();
        let bytes_to_read = if bytes_remaining <= 4096 {
            bytes_remaining
        } else {
            4096
        };
        total_bytes_read += bytes_to_read as u64;
        let mut b = vec![0; bytes_to_read];
        bytes.copy_to_slice(&mut b);
        file.write_all(&b)?;
    }

    Ok((initial_file_length, total_bytes_read))
}

fn create_upload_directory(name: &str, id: &str) -> std::io::Result<()> {
    fs::create_dir_all(format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    ))
}

fn store_blob_in_filesystem(
    name: &str,
    id: &str,
    digest: &str,
    bytes: Bytes,
) -> std::io::Result<String> {
    let blob_upload_dest_dir = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    );
    let mut blob_upload_dest_data = blob_upload_dest_dir.clone();
    blob_upload_dest_data.push_str("/data");
    append_to_blob(&blob_upload_dest_data, bytes)?;

    let mut blob_dest = String::from(format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}",
        digest.get(7..9).unwrap(),
        digest.get(7..).unwrap()
    ));
    fs::create_dir_all(&blob_dest)?;

    blob_dest.push_str("/data");
    fs::copy(&blob_upload_dest_data, &blob_dest)?;

    fs::remove_dir_all(&blob_upload_dest_dir)?;

    Ok(blob_dest)
}

async fn get_blob_from_docker_hub(name: &str, hash: &str) -> Result<String, Rejection> {
    let token = get_docker_hub_auth_token(name).await?;

    get_blob_from_docker_hub_with_token(name, hash, token).await
}

async fn get_blob_from_docker_hub_with_token(
    name: &str,
    hash: &str,
    token: String,
) -> Result<String, Rejection> {
    let url = format!(
        "https://registry-1.docker.io/v2/library/{}/blobs/{}",
        name, hash
    );
    debug!("Reading blob from docker.io with url: {}", url);
    let response = Client::new()
        .get(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token))
        .send()
        .await
        .map_err(RegistryError::from)?;

    debug!("Got blob from docker.io with status {}", response.status());
    let bytes = response.bytes().await.map_err(RegistryError::from)?;

    let id = Uuid::new_v4();

    create_upload_directory(&name, &id.to_string()).map_err(RegistryError::from)?;

    let blob_dest = store_blob_in_filesystem(&name, &id.to_string(), &hash, bytes)
        .map_err(RegistryError::from)?;

    Ok(blob_dest)
}
