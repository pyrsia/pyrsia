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

#[derive(Debug, Clone, Error, PartialEq)]
pub enum TransparencyLogError {
    #[error("Duplicate ID {id:?} in transparency log")]
    DuplicateId { id: String },
    #[error("ID {id:?} not found in transparency log")]
    NotFound { id: String },
    #[error("Hash Verification failed for ID {id:?}: {invalid_hash:?} vs {actual_hash:?}")]
    InvalidHash {
        id: String,
        invalid_hash: String,
        actual_hash: String,
    },
}

#[derive(Debug, Clone, strum_macros::Display, Deserialize, Serialize, PartialEq)]
pub enum Operation {
    AddArtifact,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct Payload {
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
    payloads: HashMap<String, Payload>,
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

        write_payload(&payload)?;
        self.payloads.insert(id.into(), payload);

        Ok(())
    }

    pub fn verify_artifact(&mut self, id: &str, hash: &str) -> Result<(), TransparencyLogError> {
        if let Some(payload) = self.payloads.get(id) {
            if payload.hash == hash {
                Ok(())
            } else {
                Err(TransparencyLogError::InvalidHash {
                    id: String::from(id),
                    invalid_hash: String::from(hash),
                    actual_hash: payload.hash.clone(),
                })
            }
        } else {
            Err(TransparencyLogError::NotFound {
                id: String::from(id),
            })
        }
    }

    pub fn get_artifact(&mut self, namespace_specific_id: &str) -> anyhow::Result<String> {
        if let Some(payload) = self.payloads.get(namespace_specific_id) {
            return Ok(String::from(&payload.hash));
        }

        anyhow::bail!("No payload found with specified ID");
    }
}

fn write_payload(payload: &Payload) -> anyhow::Result<()> {
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
            Ok(())
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
    use crate::util::test_util;
    use assay::assay;

    #[test]
    fn create_payload() {
        let id = "id";
        let hash = "hash";
        let timestamp = 1234567890;
        let operation = Operation::AddArtifact;
        let payload = Payload {
            id: id.to_string(),
            hash: hash.to_string(),
            timestamp,
            operation: Operation::AddArtifact,
        };

        assert_eq!(payload.id, id);
        assert_eq!(payload.hash, hash);
        assert_eq!(payload.timestamp, timestamp);
        assert_eq!(payload.operation, operation);
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
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
        teardown = test_util::tear_down()
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
        teardown = test_util::tear_down()
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
        teardown = test_util::tear_down()
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
        teardown = test_util::tear_down()
    )]
    fn test_add_duplicate_artifact() {
        let mut log = TransparencyLog::new();

        let result = log.add_artifact("id", "hash");
        assert!(result.is_ok());

        let result = log.add_artifact("id", "hash2");
        assert!(result.is_err());
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    fn test_verify_artifact() {
        let mut log = TransparencyLog::new();

        log.add_artifact("id", "hash")
            .expect("Adding artifact failed.");

        let result = log.verify_artifact("id", "hash");
        assert!(result.is_ok());
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    fn test_verify_unknown_artifact() {
        let mut log = TransparencyLog::new();

        let result = log.verify_artifact("id", "hash");
        assert!(result.is_err());
        assert_eq!(
            result,
            Err(TransparencyLogError::NotFound {
                id: String::from("id")
            })
        );
    }

    #[assay(
        env = [
            ("PYRSIA_ARTIFACT_PATH", "pyrsia-test-transparency-log"),
            ("DEV_MODE", "on")
        ],
        teardown = test_util::tear_down()
    )]
    fn test_verify_artifact_with_invalid_hash() {
        let mut log = TransparencyLog::new();

        log.add_artifact("id", "hash")
            .expect("Adding artifact failed.");

        let result = log.verify_artifact("id", "invalid_hash");
        assert!(result.is_err());
        assert_eq!(
            result,
            Err(TransparencyLogError::InvalidHash {
                id: String::from("id"),
                invalid_hash: String::from("invalid_hash"),
                actual_hash: String::from("hash"),
            })
        );
    }
}
