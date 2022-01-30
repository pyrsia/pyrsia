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
    let disk_used_bytes = ART_MGR.space_used()?;

    let mut available_space: u64 = 0;
    let total_allocated_size: u64 = Byte::from_str(ART_MGR_ALLOCATED_SIZE).unwrap().get_bytes();

    if total_allocated_size > disk_used_bytes {
        available_space = total_allocated_size - disk_used_bytes;
    }
    Ok(available_space)
}

pub fn disk_usage() -> Result<f64, anyhow::Error> {
    let disk_used_bytes = ART_MGR.space_used()?;

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
    use std::fs;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::tempdir;

    const MANIFEST_V1_JSON: &str = r##"{
        "name": "hello-world",
        "tag": "v3.1",
        "architecture": "amd64",
        "fsLayers": [
           {
              "blobSum": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"
           },
           {
              "blobSum": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"
           },
           {
              "blobSum": "sha256:cc8567d70002e957612902a8e985ea129d831ebe04057d88fb644857caa45d11"
           },
           {
              "blobSum": "sha256:5f70bf18a086007016e948b04aed3b82103a36bea41755b6cddfaf10ace3c6ef"
           }
        ],
        "history": [
           {
              "v1Compatibility": "{\"id\":\"e45a5af57b00862e5ef5782a9925979a02ba2b12dff832fd0991335f4a11e5c5\",\"parent\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"created\":\"2014-12-31T22:57:59.178729048Z\",\"container\":\"27b45f8fb11795b52e9605b686159729b0d9ca92f76d40fb4f05a62e19c46b4f\",\"container_config\":{\"Hostname\":\"8ce6509d66e2\",\"Domainname\":\"\",\"User\":\"\",\"Memory\":0,\"MemorySwap\":0,\"CpuShares\":0,\"Cpuset\":\"\",\"AttachStdin\":false,\"AttachStdout\":false,\"AttachStderr\":false,\"PortSpecs\":null,\"ExposedPorts\":null,\"Tty\":false,\"OpenStdin\":false,\"StdinOnce\":false,\"Env\":[\"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"],\"Cmd\":[\"/bin/sh\",\"-c\",\"#(nop) CMD [/hello]\"],\"Image\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"Volumes\":null,\"WorkingDir\":\"\",\"Entrypoint\":null,\"NetworkDisabled\":false,\"MacAddress\":\"\",\"OnBuild\":[],\"SecurityOpt\":null,\"Labels\":null},\"docker_version\":\"1.4.1\",\"config\":{\"Hostname\":\"8ce6509d66e2\",\"Domainname\":\"\",\"User\":\"\",\"Memory\":0,\"MemorySwap\":0,\"CpuShares\":0,\"Cpuset\":\"\",\"AttachStdin\":false,\"AttachStdout\":false,\"AttachStderr\":false,\"PortSpecs\":null,\"ExposedPorts\":null,\"Tty\":false,\"OpenStdin\":false,\"StdinOnce\":false,\"Env\":[\"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"],\"Cmd\":[\"/hello\"],\"Image\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"Volumes\":null,\"WorkingDir\":\"\",\"Entrypoint\":null,\"NetworkDisabled\":false,\"MacAddress\":\"\",\"OnBuild\":[],\"SecurityOpt\":null,\"Labels\":null},\"architecture\":\"amd64\",\"os\":\"linux\",\"Size\":0}\n"
           },
           {
              "v1Compatibility": "{\"id\":\"e45a5af57b00862e5ef5782a9925979a02ba2b12dff832fd0991335f4a11e5c5\",\"parent\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"created\":\"2014-12-31T22:57:59.178729048Z\",\"container\":\"27b45f8fb11795b52e9605b686159729b0d9ca92f76d40fb4f05a62e19c46b4f\",\"container_config\":{\"Hostname\":\"8ce6509d66e2\",\"Domainname\":\"\",\"User\":\"\",\"Memory\":0,\"MemorySwap\":0,\"CpuShares\":0,\"Cpuset\":\"\",\"AttachStdin\":false,\"AttachStdout\":false,\"AttachStderr\":false,\"PortSpecs\":null,\"ExposedPorts\":null,\"Tty\":false,\"OpenStdin\":false,\"StdinOnce\":false,\"Env\":[\"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"],\"Cmd\":[\"/bin/sh\",\"-c\",\"#(nop) CMD [/hello]\"],\"Image\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"Volumes\":null,\"WorkingDir\":\"\",\"Entrypoint\":null,\"NetworkDisabled\":false,\"MacAddress\":\"\",\"OnBuild\":[],\"SecurityOpt\":null,\"Labels\":null},\"docker_version\":\"1.4.1\",\"config\":{\"Hostname\":\"8ce6509d66e2\",\"Domainname\":\"\",\"User\":\"\",\"Memory\":0,\"MemorySwap\":0,\"CpuShares\":0,\"Cpuset\":\"\",\"AttachStdin\":false,\"AttachStdout\":false,\"AttachStderr\":false,\"PortSpecs\":null,\"ExposedPorts\":null,\"Tty\":false,\"OpenStdin\":false,\"StdinOnce\":false,\"Env\":[\"PATH=/usr/local/sbin:/usr/local/bin:/usr/sbin:/usr/bin:/sbin:/bin\"],\"Cmd\":[\"/hello\"],\"Image\":\"31cbccb51277105ba3ae35ce33c22b69c9e3f1002e76e4c736a2e8ebff9d7b5d\",\"Volumes\":null,\"WorkingDir\":\"\",\"Entrypoint\":null,\"NetworkDisabled\":false,\"MacAddress\":\"\",\"OnBuild\":[],\"SecurityOpt\":null,\"Labels\":null},\"architecture\":\"amd64\",\"os\":\"linux\",\"Size\":0}\n"
           }
        ],
        "schemaVersion": 1,
        "signatures": [
           {
              "header": {
                 "jwk": {
                    "crv": "P-256",
                    "kid": "OD6I:6DRK:JXEJ:KBM4:255X:NSAA:MUSF:E4VM:ZI6W:CUN2:L4Z6:LSF4",
                    "kty": "EC",
                    "x": "3gAwX48IQ5oaYQAYSxor6rYYc_6yjuLCjtQ9LUakg4A",
                    "y": "t72ge6kIA1XOjqjVoEOiPPAURltJFBMGDSQvEGVB010"
                 },
                 "alg": "ES256"
              },
              "signature": "XREm0L8WNn27Ga_iE_vRnTxVMhhYY0Zst_FfkKopg6gWSoTOZTuW4rK0fg_IqnKkEKlbD83tD46LKEGi5aIVFg",
              "protected": "eyJmb3JtYXRMZW5ndGgiOjY2MjgsImZvcm1hdFRhaWwiOiJDbjAiLCJ0aW1lIjoiMjAxNS0wNC0wOFQxODo1Mjo1OVoifQ"
           }]}"##;

    const GOOD_ART_HASH: [u8; 32] = [
        0x9c, 0x9f, 0xe2, 0x3f, 0x63, 0xae, 0x3b, 0x6c, 0x50, 0xe7, 0x85, 0x8, 0xeb, 0x99, 0xd7,
        0x9a, 0xae, 0xd3, 0x31, 0x1c, 0xfc, 0x9f, 0x6f, 0xba, 0xce, 0xa4, 0x2d, 0x7b, 0x94, 0x31,
        0xa, 0x9f,
    ];

    #[test]
    fn put_and_get_artifact_test() -> Result<(), anyhow::Error> {
        debug!("put_and_get_artifact_test started !!");

        let dir = tempdir()?;
        let file_path = dir.path().join("artifact_test.txt");
        let mut file = File::create(file_path.clone())?;
        writeln!(file, "{}", MANIFEST_V1_JSON)?;

        let reader = File::open(file_path.clone()).unwrap();
        let push_result =
            put_artifact(&GOOD_ART_HASH, Box::new(reader)).context("Error from put_artifact")?;
        assert_eq!(push_result, true);

        // pull artifact
        let file = get_artifact(&GOOD_ART_HASH, HashAlgorithm::SHA256)
            .context("Error from get_artifact")?;

        //validate pulled artifact with the actual data
        let expected_content_vec = fs::read(file_path).context("reading pushed file")?;
        let s = match str::from_utf8(expected_content_vec.as_slice()) {
            Ok(v) => v,
            Err(e) => panic!("Invalid UTF-8 sequence: {}", e),
        };

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
        if Path::new(ART_MGR_DIR).exists() {
            fs::remove_dir_all(ART_MGR_DIR).expect(&format!(
                "unable to remove artifact manager directory {}",
                ART_MGR_DIR
            ));
        }

        let usage_pct_before = disk_usage().context("Error from disk_usage")?;
        assert_eq!("0.0", format!("{:.1}", usage_pct_before));

        create_artifact().context("Error creating artifact")?;

        let usage_pct_after = disk_usage().context("Error from disk_usage")?;
        assert_eq!("0.000047", format!("{:.6}", usage_pct_after));

        Ok(())
    }

    #[test]
    fn test_get_space_available() -> Result<(), anyhow::Error> {
        if Path::new(ART_MGR_DIR).exists() {
            fs::remove_dir_all(ART_MGR_DIR).expect(&format!(
                "unable to remove artifact manager directory {}",
                ART_MGR_DIR
            ));
        }
        let space_available_before =
            get_space_available().context("Error from get_space_available")?;
        assert_eq!(10840000000, space_available_before);

        create_artifact().context("Error creating artifact")?;

        let space_available = get_space_available().context("Error from get_space_available")?;
        assert_eq!(10839994888, space_available);

        Ok(())
    }
    fn create_artifact() -> Result<(), anyhow::Error> {
        let dir = tempdir()?;
        let file_path = dir.path().join("artifact_test.txt");
        let mut file = File::create(file_path.clone())?;
        writeln!(file, "{}", MANIFEST_V1_JSON)?;

        let reader = File::open(file_path).unwrap();
        let push_result =
            put_artifact(&GOOD_ART_HASH, Box::new(reader)).context("Error from put_artifact")?;

        assert_eq!(push_result, true);
        Ok(())
    }
}
