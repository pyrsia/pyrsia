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
use lazy_static::lazy_static;
use log::{debug, error, info};
use std::fs::File;
use std::io::{BufReader, Read};
use std::panic::UnwindSafe;
use std::str;
use std::{fs, panic};

pub const ART_MGR_DIR: &str = "pyrsia";
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

pub fn get_space_available(repository_path: &str) -> Result<u64, anyhow::Error> {
    let disk_used_bytes = ART_MGR.space_used(repository_path)?;

    let mut available_space: u64 = 0;
    let total_allocated_size: u64 = Byte::from_str(ART_MGR_ALLOCATED_SIZE).unwrap().get_bytes();

    if total_allocated_size > disk_used_bytes {
        available_space = total_allocated_size - disk_used_bytes;
    }
    Ok(available_space)
}

pub fn disk_usage(repository_path: &str) -> Result<f64, anyhow::Error> {
    let disk_used_bytes = ART_MGR.space_used(repository_path)?;

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
    use super::ArtifactManager;
    use super::HashAlgorithm;
    use super::*;
    use anyhow::Context;
    use std::fs::File;
    use std::path::PathBuf;
    use tempfile::Builder;

    use super::Hash;

    const GOOD_ART_HASH: [u8; 32] = [
        0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9, 0xbc,
        0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36, 0x44, 0xc1,
        0x4f,
    ];

    #[test]
    fn put_and_get_artifact_test() -> Result<(), anyhow::Error> {
        debug!("put_and_get_artifact_test started !!");
        //put the artifact
        put_artifact(&GOOD_ART_HASH, Box::new(get_file_reader()?))
            .context("Error from put_artifact")?;

        // pull artiafct
        let file = get_artifact(&GOOD_ART_HASH, HashAlgorithm::SHA256)
            .context("Error from get_artifact")?;

        //validate pulled artifact with the actual data
        let mut s = String::new();
        get_file_reader()?.read_to_string(&mut s)?;

        let s1 = match str::from_utf8(file.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        assert_eq!(s, s1);

        debug!("put_and_get_artifact_test ended !!");
        Ok(())
    }

    #[test]
    fn test_that_a_metadata_manager_is_created_and_accessible() {
        let untrusted_key_pair = METADATA_MGR.untrusted_key_pair();
        assert!(!untrusted_key_pair.public_key.is_empty());
        assert!(!untrusted_key_pair.private_key.is_empty());
    }

    #[test]
    fn test_disk_usage() -> Result<(), anyhow::Error> {
        let tmp_dir = Builder::new().prefix("PyrisaTest").tempdir()?;
        let tmp_path = tmp_dir.path().to_owned();
        assert!(tmp_path.exists());

        let name = tmp_path.to_str().unwrap();

        let am: ArtifactManager = ArtifactManager::new(name)?;

        let usage_pct_before = disk_usage(name).context("Error from disk_usage")?;
        assert_eq!("0.0", format!("{:.1}", usage_pct_before));

        create_artifact(am).context("Error creating artifact")?;

        let usage_pct_after = disk_usage(name).context("Error from disk_usage")?;
        assert_eq!("0.000047", format!("{:.6}", usage_pct_after));

        Ok(())
    }

    #[test]
    fn test_get_space_available() -> Result<(), anyhow::Error> {
        let tmp_dir = Builder::new().prefix("PyrisaTest").tempdir()?;
        let tmp_path = tmp_dir.path().to_owned();
        assert!(tmp_path.exists());

        let name = tmp_path.to_str().unwrap();

        let am: ArtifactManager = ArtifactManager::new(name)?;

        let space_available_before =
            get_space_available(name).context("Error from get_space_available")?;
        assert_eq!(10840000000, space_available_before);

        create_artifact(am).context("Error creating artifact")?;

        let space_available =
            get_space_available(name).context("Error from get_space_available")?;
        assert_eq!(10839994889, space_available);

        Ok(())
    }

    fn get_file_reader() -> Result<File, anyhow::Error> {
        // test artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/artifact_test.json");
        println!("curr_dir is: {}", curr_dir.display());

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }

    fn create_artifact(am: ArtifactManager) -> Result<(), anyhow::Error> {
        let hash = Hash::new(HashAlgorithm::SHA256, &GOOD_ART_HASH)?;
        let push_result = am
            .push_artifact(&mut get_file_reader()?, &hash)
            .context("Error while pushing artifact")?;

        assert_eq!(push_result, true);
        Ok(())
    }
}
