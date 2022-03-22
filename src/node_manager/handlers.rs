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

use super::config::get_config;
use super::ArtifactManager;
use super::Hash;
use super::HashAlgorithm;

use crate::metadata_manager::metadata::Metadata;
use crate::network::kademlia_thread_safe_proxy::KademliaThreadSafeProxy;
use crate::util::env_util::*;
use anyhow::{Context, Result};
use byte_unit::Byte;
use lazy_static::lazy_static;
use libp2p::{identity, kad::record::store::MemoryStore, PeerId};
use log::{debug, error, info};
use std::fs::File;
use std::io::{BufReader, Read};
use std::panic::UnwindSafe;
use std::str;
use std::{fs, panic};

lazy_static! {
    pub static ref LOCAL_KEY: identity::Keypair = identity::Keypair::generate_ed25519();
    pub static ref LOCAL_PEER_ID: PeerId = PeerId::from(LOCAL_KEY.public());
    pub static ref MEMORY_STORE: MemoryStore = MemoryStore::new(*LOCAL_PEER_ID);
    pub static ref KADEMLIA_PROXY: KademliaThreadSafeProxy = KademliaThreadSafeProxy::default();
    pub static ref ARTIFACTS_DIR: String = log_static_initialization_failure(
        "Pyrsia Artifact directory",
        Ok(read_var("PYRSIA_ARTIFACT_PATH", "pyrsia"))
    );
    pub static ref ART_MGR: ArtifactManager = {
        let dev_mode = read_var("DEV_MODE", "off");
        if dev_mode.to_lowercase() == "on" {
            log_static_initialization_failure(
                "Artifact Manager Directory",
                fs::create_dir_all(ARTIFACTS_DIR.as_str())
                    .with_context(|| "Failed to create artifact manager directory in dev mode"),
            );
        }
        log_static_initialization_failure(
            "Artifact Manager",
            ArtifactManager::new(ARTIFACTS_DIR.as_str()),
        )
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
pub fn get_artifact(art_hash: &[u8], algorithm: HashAlgorithm) -> Result<Vec<u8>, anyhow::Error> {
    let hash = Hash::new(algorithm, art_hash)?;
    let result = ART_MGR.pull_artifact(&hash)?;
    let mut buf_reader: BufReader<File> = BufReader::new(result);
    let mut blob_content = Vec::new();
    buf_reader.read_to_end(&mut blob_content)?;
    Ok(blob_content)
}

//get_artifact_hashes: retrieve a list of hashes of all artifacts that are stored in
//                     the artifact_manager
pub fn get_artifact_hashes() -> Result<Vec<String>, anyhow::Error> {
    let artifacts = ART_MGR.list_artifacts()?;
    Ok(artifacts
        .into_iter()
        .map(|artifact| {
            let hash_type = artifact
                .parent()
                .unwrap()
                .file_name()
                .unwrap()
                .to_str()
                .unwrap();
            let hash_value = artifact.file_name().unwrap().to_str().unwrap();
            let extension_dot = hash_value.rfind('.').unwrap();
            format!(
                "{}:{}",
                hash_type.to_lowercase(),
                hash_value.get(0..extension_dot).unwrap()
            )
        })
        .collect())
}

//put_artifact: given artifact_hash(artifactName) & artifact_path push artifact to artifact_manager
//              and returns the boolean as true or false if it was able to create or not
pub fn put_artifact(
    artifact_hash: &[u8],
    art_reader: Box<dyn Read>,
    algorithm: HashAlgorithm,
) -> Result<bool, anyhow::Error> {
    let hash = Hash::new(algorithm, artifact_hash)?;
    info!("put_artifact hash: {}", hash);
    let mut buf_reader = BufReader::new(art_reader);
    ART_MGR
        .push_artifact(&mut buf_reader, &hash)
        .context("Error from put_artifact")
}

pub fn get_arts_count() -> Result<usize, anyhow::Error> {
    ART_MGR
        .artifacts_count()
        .context("Error while getting artifacts count")
}

pub fn get_space_available() -> Result<u64, anyhow::Error> {
    let disk_used_bytes = ART_MGR.space_used()?;

    let mut available_space: u64 = 0;
    let cli_config = get_config().context("Error getting cli config file")?;

    let total_allocated_size: u64 = Byte::from_str(cli_config.disk_allocated)
        .unwrap()
        .get_bytes();

    if total_allocated_size > disk_used_bytes {
        available_space = total_allocated_size - disk_used_bytes;
    }
    Ok(available_space)
}

pub fn disk_usage(repository_path: &str) -> Result<f64, anyhow::Error> {
    let disk_used_bytes = ART_MGR.space_used(repository_path)?;
    let cli_config = get_config().context("Error getting cli config file")?;
    let total_allocated_size: u64 = Byte::from_str(cli_config.disk_allocated)
        .unwrap()
        .get_bytes();
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
    use super::HashAlgorithm;
    use super::*;
    use anyhow::Context;
    use assay::assay;
    use std::env;
    use std::fs::File;
    use std::path::Path;
    use std::path::PathBuf;

    use super::Hash;

    const VALID_ARTIFACT_HASH: [u8; 32] = [
        0x86, 0x5c, 0x8d, 0x98, 0x8b, 0xe4, 0x66, 0x9f, 0x3e, 0x48, 0xf7, 0x3b, 0x98, 0xf9, 0xbc,
        0x25, 0x7, 0xbe, 0x2, 0x46, 0xea, 0x35, 0xe0, 0x9, 0x8c, 0xf6, 0x5, 0x4d, 0x36, 0x44, 0xc1,
        0x4f,
    ];
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
    fn test_put_and_get_artifact() {
        //put the artifact
        put_artifact(
            &VALID_ARTIFACT_HASH,
            Box::new(get_file_reader()?),
            HashAlgorithm::SHA256,
        )
        .context("Error from put_artifact")?;

        // pull artiafct
        let file = get_artifact(&VALID_ARTIFACT_HASH, HashAlgorithm::SHA256)
            .context("Error from get_artifact")?;

        //validate pulled artifact with the actual data
        let mut s = String::new();
        get_file_reader()?.read_to_string(&mut s)?;

        let s1 = match str::from_utf8(file.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };
        assert_eq!(s, s1);
    }

    #[test]
    fn test_that_a_metadata_manager_is_created_and_accessible() {
        let untrusted_key_pair = METADATA_MGR.untrusted_key_pair();
        assert!(!untrusted_key_pair.public_key.is_empty());
        assert!(!untrusted_key_pair.private_key.is_empty());
    }

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
          ("DEV_MODE", "on")
        ]  )]
    fn test_disk_usage() {
        let usage_pct_before = disk_usage().context("Error from disk_usage")?;

        create_artifact().context("Error creating artifact")?;

        let usage_pct_after = disk_usage().context("Error from disk_usage")?;
        assert!(usage_pct_before < usage_pct_after);
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

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
          ("DEV_MODE", "on")
        ]  )]
    fn test_get_artifact_hashes_is_empty() {
        let artifact_hashes = get_artifact_hashes().context("Error from get_artifact_hashes")?;
        assert!(artifact_hashes.is_empty());
    }

    #[assay(
        env = [
          ("PYRSIA_ARTIFACT_PATH", "PyrsiaTest"),
          ("DEV_MODE", "on")
        ]  )]
    fn test_get_artifact_hashes() {
        create_artifact().context("Error creating artifact")?;

        let artifact_hashes = get_artifact_hashes().context("Error from get_artifact_hashes")?;
        assert!(artifact_hashes.len() == 1);
    }

    fn get_file_reader() -> Result<File, anyhow::Error> {
        // test artifact file in resources/test dir
        let mut curr_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        curr_dir.push("tests/resources/artifact_test.json");

        let path = String::from(curr_dir.to_string_lossy());
        let reader = File::open(path.as_str()).unwrap();
        Ok(reader)
    }

    fn create_artifact() -> Result<(), anyhow::Error> {
        let hash = Hash::new(HashAlgorithm::SHA256, &VALID_ARTIFACT_HASH)?;
        let push_result = ART_MGR
            .push_artifact(&mut get_file_reader()?, &hash)
            .context("Error while pushing artifact")?;

        assert_eq!(push_result, true);
        Ok(())
    }
}
