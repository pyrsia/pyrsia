use bytes::{Buf, Bytes};
use log::{debug, error};
use std::collections::HashMap;
use std::fs;
use std::io::prelude::*;
use std::path::Path;
use uuid::Uuid;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

use crate::utils::error_util::RegistryError as registry_err;
use crate::utils::error_util::RegistryErrorCode as registry_err_code;

pub async fn handle_get_blobs(_name: String, hash: String) -> Result<impl Reply, Rejection> {
    let blob = format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}/data",
        hash.get(7..9).unwrap(),
        hash.get(7..).unwrap()
    );
    debug!("Getting blob: {}", blob);
    if !Path::new(&blob).is_file() {
        return Err(warp::reject::custom(registry_err {
            code: registry_err_code::BlobUnknown,
        }));
    }

    let blob_content = fs::read(blob);
    if blob_content.is_err() {
        return Err(warp::reject::custom(registry_err {
            code: registry_err_code::BlobUnknown,
        }));
    }

    let content = blob_content.unwrap();
    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(content)
        .unwrap())
}

pub async fn handle_post_blob(name: String) -> Result<impl Reply, Rejection> {
    let id = Uuid::new_v4();

    if let Err(e) = fs::create_dir_all(format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    )) {
        return Err(warp::reject::custom(registry_err {
            code: registry_err_code::Unknown(e.to_string()),
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
        return Err(warp::reject::custom(registry_err {
            code: registry_err_code::Unknown(e.to_string()),
        }));
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
        return Err(warp::reject::custom(registry_err {
            code: registry_err_code::Unknown(e.to_string()),
        }));
    }

    let digest = match params.get("digest") {
        Some(v) => v,
        None => {
            return Err(warp::reject::custom(registry_err {
                code: registry_err_code::Unknown(String::from("missing digest")),
            }))
        }
    };

    let mut blob_dest = String::from(format!(
        "/tmp/registry/docker/registry/v2/blobs/sha256/{}/{}",
        digest.get(7..9).unwrap(),
        digest.get(7..).unwrap()
    ));
    if let Err(e) = fs::create_dir_all(&blob_dest) {
        return Err(warp::reject::custom(registry_err {
            code: registry_err_code::Unknown(e.to_string()),
        }));
    }

    blob_dest.push_str("/data");
    if let Err(e) = fs::copy(&blob_upload_dest_data, &blob_dest) {
        return Err(warp::reject::custom(registry_err {
            code: registry_err_code::Unknown(e.to_string()),
        }));
    }

    if let Err(e) = fs::remove_dir_all(&blob_upload_dest_dir) {
        return Err(warp::reject::custom(registry_err {
            code: registry_err_code::Unknown(e.to_string()),
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
