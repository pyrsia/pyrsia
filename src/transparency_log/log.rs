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

use crate::util::env_util::read_var;
use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;

#[derive(Debug, Clone, Error)]
enum TransparencyLogError {
    #[error("Duplicate ID {id:?} in transparency log")]
    DuplicateId { id: String },
}

#[derive(Debug, strum_macros::Display, Deserialize, Serialize, Clone)]
enum Operation {
    AddArtifact,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
struct Payload {
    id: String,
    hash: String,
    timestamp: u64,
    operation: Operation,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignatureEnvelope {
    /// The data that is integrity protected
    payload: Payload,
    /// The time at which the signature was generated. This is a part of signed attributes
    signing_timestamp: u64,
    /// The digital signature computed on payload and signed attributes
    signature: Vec<u8>,
    /// the public key of the signer
    sign_identifier: [u8; 32], //this is identity::ed25519::PublicKey(a byte array in compressed form
}

#[derive(Clone)]
pub struct TransparencyLog {
    payloads: HashMap<String, String>,
}

impl TransparencyLog {
    pub fn new() -> Self {
        TransparencyLog {
            payloads: HashMap::new(),
        }
    }

    pub fn add_artifact(&mut self, id: &str, hash: &str) -> anyhow::Result<()> {
        let payload = Payload {
            id: id.to_string(),
            hash: hash.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operation: Operation::AddArtifact,
        };

        let json_payload = write_payload(&payload)?;
        self.payloads.insert(id.to_string(), json_payload);

        Ok(())
    }
}

fn write_payload(payload: &Payload) -> anyhow::Result<String> {
    let payload_storage_path = get_payload_storage_path();
    fs::create_dir_all(&payload_storage_path)?;
    let payload_filename = format!(
        "{}/{}.log",
        payload_storage_path,
        str::replace(&payload.id, "/", "_")
    );
    debug!(
        "Storing transparency log payload at: {:?}",
        payload_filename
    );
    match fs::File::options()
        .write(true)
        .create_new(true)
        .open(&payload_filename)
    {
        Ok(mut payload_file) => {
            let json_payload = serde_json::to_string(payload)?;
            payload_file.write_all(json_payload.as_bytes())?;
            Ok(json_payload)
        }
        Err(e) => match e.kind() {
            io::ErrorKind::AlreadyExists => Err(TransparencyLogError::DuplicateId {
                id: payload.id.clone(),
            }
            .into()),
            _ => Err(e.into()),
        },
    }
}

fn get_payload_storage_path() -> String {
    format!(
        "{}/{}",
        read_var("PYRSIA_ARTIFACT_PATH", "pyrsia"),
        "transparency_log"
    )
}

impl Default for TransparencyLog {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay::assay;
    use std::env;
    use std::path::Path;

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
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = tear_down()
    )]
    fn test_new_transparency_log_has_empty_payload() {
        let log = TransparencyLog::new();

        assert_eq!(log.payloads.len(), 0);
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = tear_down()
    )]
    fn test_with_default() {
        let log: TransparencyLog = Default::default();

        assert_eq!(log.payloads.len(), 0);
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = tear_down()
    )]
    fn test_add_artifact() {
        let mut log = TransparencyLog::new();

        let result = log.add_artifact("id", "hash");
        assert!(result.is_ok());

        assert!(log.payloads.contains_key("id"));
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = tear_down()
    )]
    fn test_add_artifact_with_id_containing_forward_slash() {
        let mut log = TransparencyLog::new();

        let result = log.add_artifact("id/with/slash", "hash");
        assert!(result.is_ok());

        assert!(log.payloads.contains_key("id/with/slash"));
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = tear_down()
    )]
    fn test_add_duplicate_artifact() {
        let mut log = TransparencyLog::new();

        let result = log.add_artifact("id", "hash");
        assert!(result.is_ok());

        let result = log.add_artifact("id", "hash2");
        assert!(result.is_err());
    }
}
