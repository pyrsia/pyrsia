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

use log::debug;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
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
    storage_path: PathBuf,
    payloads: HashMap<String, Payload>,
}

impl TransparencyLog {
    pub fn new<P: AsRef<Path>>(repository_path: P) -> Result<Self, anyhow::Error> {
        let mut absolute_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        absolute_path.push("transparency_log");
        Ok(TransparencyLog {
            storage_path: absolute_path,
            payloads: HashMap::new(),
        })
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

        self.write_payload(&payload)?;
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

    fn write_payload(&self, payload: &Payload) -> anyhow::Result<()> {
        fs::create_dir_all(&self.storage_path)?;
        let payload_filename = format!(
            "{}/{}.log",
            self.storage_path.to_str().unwrap(),
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_util;

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

    #[test]
    fn test_new_transparency_log_has_empty_payload() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        assert_eq!(log.payloads.len(), 0);

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_add_artifact() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.add_artifact("id", "hash");
        assert!(result.is_ok());

        assert!(log.payloads.contains_key("id"));

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_add_artifact_with_id_containing_forward_slash() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.add_artifact("id/with/slash", "hash");
        assert!(result.is_ok());

        assert!(log.payloads.contains_key("id/with/slash"));

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_add_duplicate_artifact() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.add_artifact("id", "hash");
        assert!(result.is_ok());

        let result = log.add_artifact("id", "hash2");
        assert!(result.is_err());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_verify_artifact() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        log.add_artifact("id", "hash")
            .expect("Adding artifact failed.");

        let result = log.verify_artifact("id", "hash");
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_verify_unknown_artifact() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.verify_artifact("id", "hash");
        assert!(result.is_err());
        assert_eq!(
            result,
            Err(TransparencyLogError::NotFound {
                id: String::from("id")
            })
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_verify_artifact_with_invalid_hash() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

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

        test_util::tests::teardown(tmp_dir);
    }
}
