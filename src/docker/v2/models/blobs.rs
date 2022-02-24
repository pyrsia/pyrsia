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

use super::*;
use crate::docker::docker_hub_util::get_docker_hub_auth_token;
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use bytes::{Buf, Bytes};
use futures::stream::{FusedStream, Stream};
use futures::task::{Context, Poll};
use log::{debug, error, info, trace};
use reqwest::{header, Client};
use std::collections::HashMap;
use std::fs;
use std::fs::File;
use std::io::prelude::*;
use std::pin::Pin;
use std::result::Result;
use std::str;
use uuid::Uuid;
use warp::{http::StatusCode, Rejection, Reply};

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

pub fn store_blob_in_filesystem(
    name: &str,
    id: &str,
    digest: &str,
    bytes: Bytes,
) -> Result<bool, Box<dyn std::error::Error>> {
    let blob_upload_dest_dir = format!(
        "/tmp/registry/docker/registry/v2/repositories/{}/_uploads/{}",
        name, id
    );
    let mut blob_upload_dest_data = blob_upload_dest_dir.clone();
    blob_upload_dest_data.push_str("/data");
    append_to_blob(&blob_upload_dest_data, bytes)?;

    //put blob in artifact manager
    let reader = File::open(blob_upload_dest_data.as_str()).unwrap();

    let push_result = put_artifact(
        hex::decode(&digest.get(7..).unwrap()).unwrap().as_ref(),
        Box::new(reader),
    )?;

    fs::remove_dir_all(&blob_upload_dest_dir)?;

    Ok(push_result)
}

#[cfg(test)]

mod tests {
    use super::*;
    use bytes::{BufMut, BytesMut};
    use expectest::expect;
    use expectest::prelude::*;

    #[test]
    fn append_to_blob_writes_to_filesystem() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("append_to_blob_writes_to_filesystem");
        let blob_upload_dest_dir = path.to_str().unwrap();

        let mut buf = BytesMut::with_capacity(20);
        buf.put(&b"sample_blob"[..]);
        let bytes = buf.freeze();
        let length_of_hello_world: u64 = bytes.len().try_into().unwrap();

        let result = append_to_blob(&blob_upload_dest_dir, bytes);
        let (_, total_bytes_appended) = result.unwrap();

        expect!(total_bytes_appended).to(be_equal_to(length_of_hello_world));
    }
}
