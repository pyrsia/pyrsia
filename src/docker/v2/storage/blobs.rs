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

use crate::artifacts_repository::hash_util::HashAlgorithm;
use crate::node_manager::handlers::*;
use bytes::{Buf, Bytes};
use log::debug;
use std::fs;
use std::fs::File;
use std::io::prelude::*;

pub fn append_to_blob(blob: &str, mut bytes: Bytes) -> std::io::Result<(u64, u64)> {
    debug!("Patching blob: {}", blob);
    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(blob)?;
    let mut total_bytes_read: u64 = 0;
    let initial_file_length: u64 = file.metadata()?.len();
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

pub fn create_upload_directory(name: &str, id: &str) -> std::io::Result<()> {
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
    let append = append_to_blob(&blob_upload_dest_data, bytes)?;

    // check if there is enough local allocated disk space
    let available_space = get_space_available();
    if available_space.is_err() {
        return Err(available_space.err().unwrap().to_string().into());
    }
    if append.1 > available_space.unwrap() {
        return Err("Not enough space left to store artifact".into());
    }
    //put blob in artifact manager
    let reader = File::open(blob_upload_dest_data.as_str()).unwrap();

    let push_result = put_artifact(
        hex::decode(&digest.get(7..).unwrap()).unwrap().as_ref(),
        Box::new(reader),
        HashAlgorithm::SHA256,
    )?;

    fs::remove_dir_all(&blob_upload_dest_dir)?;

    Ok(push_result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::node_manager::Hash;
    use anyhow::{Context, Result};
    use assay::assay;
    use bytes::{BufMut, BytesMut};
    use std::path::PathBuf;

    #[assay]
    fn append_to_blob_writes_to_filesystem() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("append_to_blob_writes_to_filesystem");
        let blob_upload_dest_dir = path.to_str().unwrap();

        let mut buf = BytesMut::with_capacity(20);
        buf.put(&b"sample_blob"[..]);
        let bytes = buf.freeze();
        let length_of_sample_blob: u64 = bytes.len().try_into().unwrap();

        let result = append_to_blob(&blob_upload_dest_dir, bytes);
        let (_, total_bytes_appended) = result.unwrap();

        assert_eq!(total_bytes_appended, length_of_sample_blob);
    }

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
          ("DEV_MODE", "on")
        ]  )]
    fn test_get_space_available() {
        let space_available_before =
            get_space_available().context("Error from get_space_available")?;

        create_artifact().context("Error creating artifact")?;

        let space_available_after =
            get_space_available().context("Error from get_space_available")?;
        debug!(
            "Before: {}; After: {}",
            space_available_before, space_available_after
        );
        assert!(space_available_after < space_available_before);
    }

    fn create_artifact() -> Result<(), anyhow::Error> {
        const VALID_ARTIFACT_HASH: [u8; 32] = [
            0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9,
            0xbc, 0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36,
            0x44, 0xc1, 0x4f,
        ];
        let hash = Hash::new(HashAlgorithm::SHA256, &VALID_ARTIFACT_HASH)?;
        let push_result = ART_MGR
            .push_artifact(&mut get_file_reader()?, &hash)
            .context("Error while pushing artifact")?;

        assert_eq!(push_result, true);
        Ok(())
    }

    fn get_file_reader() -> Result<File, anyhow::Error> {
        // test artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/artifact_test.json");

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }
}
