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

use bytes::{Buf, Bytes};
use log::{debug, error, info};
use reqwest::{get, header};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

use super::{RegistryError, RegistryErrorCode};

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
        match get_blob_from_docker_hub(&_name, &hash).await {
            Ok(_) => {
                info!("Downloaded content from docker.io for {}", _name);
            }
            Err(_) => {
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
    }

    if !blob_path.is_file() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown("ITS_NOT_A_FILE".to_string()),
        }));
    }

    info!("Reading blob from local Pyrsia storage: {}", blob);
    match fs::read(blob_path) {
        Ok(content) => {
            return Ok(warp::http::response::Builder::new()
                .header("Content-Type", "application/octet-stream")
                .status(StatusCode::OK)
                .body(content)
                .unwrap());
        }
        Err(_) => {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::BlobUnknown,
            }));
        }
    }
}

pub async fn handle_post_blob(name: String) -> Result<impl Reply, Rejection> {
    let id = Uuid::new_v4();

    if let Err(e) = fs::create_dir_all(format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    )) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

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
    let append = append_to_blob(&mut blob_upload_dest, bytes);
    if let Err(e) = append {
        Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }))
    } else {
        let append_result = append.ok().unwrap();
        let range = format!(
            "{}-{}",
            append_result.0,
            append_result.0 + append_result.1 - 1
        );
        debug!("Patch blob range: {}", range);
        return Ok(warp::http::response::Builder::new()
            .header(
                "Location",
                format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, id),
            )
            .header("Range", &range)
            .status(StatusCode::ACCEPTED)
            .body("")
            .unwrap());
    }
}

pub async fn handle_put_blob(
    name: String,
    id: String,
    params: HashMap<String, String>,
    bytes: Bytes,
) -> Result<impl Reply, Rejection> {
    let blob_upload_dest_dir = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    );
    let mut blob_upload_dest_data = blob_upload_dest_dir.clone();
    blob_upload_dest_data.push_str("/data");
    if let Err(e) = append_to_blob(&blob_upload_dest_data, bytes) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    let digest = match params.get("digest") {
        Some(v) => v,
        None => {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(String::from("missing digest")),
            }))
        }
    };

    let mut blob_dest = String::from(format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}",
        digest.get(7..9).unwrap(),
        digest.get(7..).unwrap()
    ));
    if let Err(e) = fs::create_dir_all(&blob_dest) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    blob_dest.push_str("/data");
    if let Err(e) = fs::copy(&blob_upload_dest_data, &blob_dest) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    if let Err(e) = fs::remove_dir_all(&blob_upload_dest_dir) {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(e.to_string()),
        }));
    }

    Ok(warp::http::response::Builder::new()
        .header(
            "Location",
            format!("http://localhost:7878/v2/{}/blobs/uploads/{}", name, digest),
        )
        .status(StatusCode::CREATED)
        .body("")
        .unwrap())
}

pub fn append_to_blob(blob: &str, mut bytes: Bytes) -> Result<(u64, u64), std::io::Error> {
    debug!("Patching blob: {}", blob);
    let file = fs::OpenOptions::new().create(true).append(true).open(blob);
    let mut total_bytes_read: u64 = 0;
    let initial_file_length: u64;
    if let Ok(mut f) = file {
        initial_file_length = f.metadata().unwrap().len();
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
            if let Err(e) = f.write_all(&b) {
                error!("{}", e);
                return Err(e);
            }
        }
    } else {
        let e = file.err().unwrap();
        error!("{}", e);
        return Err(e);
    }

    Ok((initial_file_length, total_bytes_read))
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
struct Bearer {
    token: String,
    expires_in: u64,
}

async fn get_blob_from_docker_hub(name: &str, hash: &str) -> Result<String, Rejection> {
    let token = get_docker_hub_auth_token(name).await?;

    get_blob_from_docker_hub_with_token(name, hash, token).await
}

async fn get_docker_hub_auth_token(name: &str) -> Result<Bearer, Rejection> {
    let auth_url = format!("https://auth.docker.io/token?client_id=Pyrsia&service=registry.docker.io&scope=repository:library/{}:pull", name);

    match reqwest::get(auth_url).await {
        Ok(response) => match response.json().await {
            Ok(token) => Ok(token),
            Err(e) => {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }
        },
        Err(e) => {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }
    }
}

async fn get_blob_from_docker_hub_with_token(
    name: &str,
    hash: &str,
    token: Bearer,
) -> Result<String, Rejection> {
    let url = format!(
        "https://registry-1.docker.io/v2/library/{}/blobs/{}",
        name, hash
    );
    debug!("Reading blob from docker.io with url: {}", url);
    match reqwest::Client::new()
        .get(url)
        .header(header::AUTHORIZATION, format!("Bearer {}", token.token))
        .send()
        .await
    {
        Ok(response) => {
            debug!("Got blob from docker.io with status {}", response.status());
            let contents = response.bytes().await.unwrap();

            let id = Uuid::new_v4();

            if let Err(e) = fs::create_dir_all(format!(
                "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
                name, id
            )) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }

            let blob_upload_dest_dir = format!(
                "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
                name, id
            );

            let mut blob_upload_dest_data = blob_upload_dest_dir.clone();
            blob_upload_dest_data.push_str("/data");
            if let Err(e) = append_to_blob(&blob_upload_dest_data, contents) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }

            let mut blob_dest = String::from(format!(
                "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}",
                hash.get(7..9).unwrap(),
                hash.get(7..).unwrap()
            ));
            if let Err(e) = fs::create_dir_all(&blob_dest) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }

            blob_dest.push_str("/data");
            if let Err(e) = fs::copy(&blob_upload_dest_data, &blob_dest) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }

            if let Err(e) = fs::remove_dir_all(&blob_upload_dest_dir) {
                return Err(warp::reject::custom(RegistryError {
                    code: RegistryErrorCode::Unknown(e.to_string()),
                }));
            }

            return Ok(blob_dest);
        }
        Err(e) => {
            return Err(warp::reject::custom(RegistryError {
                code: RegistryErrorCode::Unknown(e.to_string()),
            }));
        }
    }
}
