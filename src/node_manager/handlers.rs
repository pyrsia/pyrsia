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

use super::ArtifactManager;
use super::HashAlgorithm;

use super::Hash;

use crate::metadata_manager::metadata::Metadata;
use anyhow::{Context, Result};
use byte_unit::Byte;
use fs_extra::dir::get_size;
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::fs::File;
use std::io::{BufReader, Read};
use std::panic::UnwindSafe;
use std::str;
use std::{fs, panic};

const ART_MGR_DIR: &str = "pyrsia";
//TODO: read from CLI config file
pub const ART_MGR_ALLOCATED_SIZE: &str = "10.84 GB";

lazy_static! {
    pub static ref ART_MGR: ArtifactManager = {
        log_static_initialization_failure(
            "Artifact Manager Directory",
            fs::create_dir_all(ART_MGR_DIR).with_context(|| {
                format!(
                    "Failed to create artifact manager directory {}",
                    ART_MGR_DIR
                )
            }),
        );
        log_static_initialization_failure("Artifact Manager", ArtifactManager::new(ART_MGR_DIR))
    };
    pub static ref METADATA_MGR: Metadata =
        log_static_initialization_failure("Metadata Manager", Metadata::new());
}

fn log_static_initialization_failure<T: UnwindSafe>(
    label: &str,
    result: Result<T, anyhow::Error>,
) -> T {
    let panic_wrapper = panic::catch_unwind(|| match result {
        Ok(unwrapped) => unwrapped,
        Err(error) => {
            let msg = format!("Error initializing {}, error is: {}", label, error);
            error!("{}", msg);
            panic!("{}", msg)
        }
    });
    match panic_wrapper {
        Ok(normal) => normal,
        Err(partially_unwound_panic) => {
            error!("Initialization of {} panicked!", label);
            panic::resume_unwind(partially_unwound_panic)
        }
    }
}

//get_artifact: given artifact_hash(artifactName) pulls artifact for  artifact_manager and
//              returns read object to read the bytes of artifact
pub fn get_artifact(
    art_hash: &[u8],
    art_algorithm: HashAlgorithm,
) -> Result<Vec<u8>, anyhow::Error> {
    let hash = Hash::new(art_algorithm, art_hash)?;
    let result = ART_MGR.pull_artifact(&hash)?;
    let mut buf_reader: BufReader<File> = BufReader::new(result);
    let mut blob_content = Vec::new();
    buf_reader.read_to_end(&mut blob_content)?;
    Ok(blob_content)
}

//put_artifact: given artifact_hash(artifactName) & artifact_path push artifact to artifact_manager
//              and returns the boolean as true or false if it was able to create or not
pub fn put_artifact(
    artifact_hash: &[u8],
    art_reader: Box<dyn Read>,
) -> Result<bool, anyhow::Error> {
    let hash = Hash::new(HashAlgorithm::SHA256, artifact_hash)?;
    info!("put_artifact hash: {}", hash);
    let mut buf_reader = BufReader::new(art_reader);
    ART_MGR
        .push_artifact(&mut buf_reader, &hash)
        .context("Error from put_artifact")
}

pub fn get_arts_count() -> Result<usize, anyhow::Error> {
    ART_MGR
        .artifacts_count(ART_MGR_DIR)
        .context("Error while getting artifacts count")
}

pub fn get_space_available() -> Result<u64, anyhow::Error> {
    let disk_used_bytes =
        get_size(ART_MGR_DIR).context("Error while calculating the size of artifact manager")?;
    let mut available_space: u64 = 0;
    let total_allocated_size: u64 = Byte::from_str(ART_MGR_ALLOCATED_SIZE).unwrap().get_bytes();

    if total_allocated_size > disk_used_bytes {
        available_space = total_allocated_size - disk_used_bytes;
    }
    Ok(available_space)
}

pub fn disk_usage() -> Result<f64, anyhow::Error> {
    let disk_used_bytes =
        get_size(ART_MGR_DIR).context("Error while calculating the size of artifact manager")?;
    let total_allocated_size: u64 = Byte::from_str(ART_MGR_ALLOCATED_SIZE).unwrap().get_bytes();
    let mut disk_usage: f64 = 0.0;
    debug!("disk_used: {}", disk_used_bytes);
    debug!("total_allocated_size: {}", total_allocated_size);

    if total_allocated_size > disk_used_bytes {
        disk_usage = (disk_used_bytes as f64 / total_allocated_size as f64) * 100_f64;
    }
    Ok(disk_usage)
}

#[cfg(test)]

mod tests {
    use super::*;
    use anyhow::Context;
    use std::env;
    use std::io::BufReader;
    use std::{fs, path::PathBuf};

    const GOOD_ART_HASH: [u8; 32] = [
        0x66, 0xdc, 0x9a, 0xc8, 0xb2, 0x77, 0x12, 0xbc, 0x2c, 0x5d, 0xa3, 0x61, 0xab, 0x41, 0x75,
        0x20, 0x6e, 0x27, 0x1a, 0x8a, 0x90, 0xd2, 0x1, 0xfb, 0xbe, 0x7, 0xb8, 0x81, 0xed, 0x8e,
        0xec, 0xa7,
    ];

    #[test]
    fn put_and_get_artifact_test() -> Result<(), anyhow::Error> {
        println!("put_and_get_artifact_test started !!");

        // test artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/artifact_test.txt");
        println!("curr_dir is: {}", curr_dir.display());

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        //put the artifact
        put_artifact(&GOOD_ART_HASH, Box::new(reader)).context("Error from put_artifact")?;

        //validate pushed artifact with actual data
        let mut push_art_path = PathBuf::from(ART_MGR_DIR);
        push_art_path.push("SHA256");
        push_art_path.push(hex::encode(GOOD_ART_HASH));
        push_art_path.set_extension("file");
        println!("reading artifact path is: {}", push_art_path.display());
        let content_vec = fs::read(push_art_path.as_path()).context("reading pushed file")?;

        let test_art_path = PathBuf::from(path.as_str());
        let actual_content_vec = fs::read(test_art_path.as_path()).context("reading test file")?;

        assert_eq!(content_vec.as_slice(), actual_content_vec.as_slice());

        // pull artifact
        let file = get_artifact(&GOOD_ART_HASH, HashAlgorithm::SHA256)
            .context("Error from get_artifact")?;

        //validate pulled artifact with the actual data
        let s = match str::from_utf8(actual_content_vec.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };

        let s1 = match str::from_utf8(file.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        assert_eq!(s, s1);

        println!("put_and_get_artifact_test ended !!");
        Ok(())
    }

    #[test]
    fn test_that_a_metadata_manager_is_created_and_accessible() {
        let untrusted_key_pair = METADATA_MGR.untrusted_key_pair();
        assert!(!untrusted_key_pair.public_key.is_empty());
        assert!(!untrusted_key_pair.private_key.is_empty());
    }
}
