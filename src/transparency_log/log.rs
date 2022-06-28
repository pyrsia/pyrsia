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
use rusqlite::{Connection, Error};
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::{Path, PathBuf};
use std::str::FromStr;
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

#[derive(
    Debug, Clone, strum_macros::Display, strum_macros::EnumString, Deserialize, Serialize, PartialEq,
)]
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
}

impl TransparencyLog {
    pub fn new<P: AsRef<Path>>(repository_path: P) -> Result<Self, anyhow::Error> {
        let mut absolute_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        absolute_path.push("transparency_log");
        Ok(TransparencyLog {
            storage_path: absolute_path,
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

        Ok(())
    }

    pub fn verify_artifact(&mut self, id: &str, hash: &str) -> Result<(), TransparencyLogError> {
        match self.read_payload_db(id) {
            Ok(payload) => {
                if payload.hash == hash {
                    Ok(())
                } else {
                    Err(TransparencyLogError::InvalidHash {
                        id: String::from(id),
                        invalid_hash: String::from(hash),
                        actual_hash: payload.hash,
                    })
                }
            }
            Err(err) => {
                debug!("Error verifying artifact {:?}", err);
                Err(TransparencyLogError::NotFound {
                    id: String::from(id),
                })
            }
        }
    }

    pub fn get_artifact(&mut self, namespace_specific_id: &str) -> anyhow::Result<String> {
        match self.read_payload_db(namespace_specific_id) {
            Ok(payload) => Ok(String::from(&payload.hash)),
            Err(_) => {
                anyhow::bail!("No payload found with specified ID")
            }
        }
    }

    fn open_db(&self) -> anyhow::Result<Connection> {
        fs::create_dir_all(&self.storage_path)?;
        let payload_storage_path = self.storage_path.to_str().unwrap();
        let conn = Connection::open(payload_storage_path.to_owned() + "/transparency_log.db")?;
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS payload (
                id TEXT PRIMARY KEY,
                hash TEXT NOT NULL,
                timestamp INTEGER,
                operation TEXT NOT NULL
            )",
            [],
        ) {
            Ok(_) => Ok(conn),
            Err(err) => {
                debug!("Error creating transparency log database table: {:?}", err);
                Err(err.into())
            }
        }
    }

    fn write_payload(&self, payload: &Payload) -> anyhow::Result<()> {
        let conn = self.open_db()?;

        let payload_to_db = payload.clone();
        match conn.execute(
            "INSERT INTO payload (id, hash, timestamp, operation) values (?1, ?2, ?3, ?4)",
            [
                payload_to_db.id,
                payload_to_db.hash,
                payload_to_db.timestamp.to_string(),
                payload_to_db.operation.to_string(),
            ],
        ) {
            Ok(_) => {
                debug!(
                    "Payload inserted into transparency log with id: {}",
                    payload.id
                );
                Ok(())
            }
            Err(Error::SqliteFailure(sqlite_error, ref _sqlite_options))
                if sqlite_error.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
            {
                Err(TransparencyLogError::DuplicateId {
                    id: payload.id.clone(),
                }
                .into())
            }
            Err(err) => Err(err.into()),
        }
    }

    fn read_payload_db(&self, id: &str) -> anyhow::Result<Payload> {
        let conn = self.open_db()?;

        let mut stmt = conn.prepare("SELECT * FROM payload WHERE id=:id;")?;
        let mut payload_records = stmt.query_map(&[(":id", id)], |row| {
            Ok(Payload {
                id: row.get(0)?,
                hash: row.get(1)?,
                timestamp: row.get(2)?,
                operation: {
                    let op: String = row.get(3)?;
                    Operation::from_str(&op).unwrap()
                },
            })
        })?;

        match payload_records.next() {
            Some(Ok(record)) => Ok(record),
            _ => Err(TransparencyLogError::NotFound { id: id.to_string() }.into()),
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
    fn test_open_db() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.open_db();
        assert!(result.is_ok());

        let conn = result.unwrap();
        let mut path = log.storage_path;
        path.push("transparency_log.db");
        assert_eq!(conn.path().unwrap(), path.as_path());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_write_payload() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let payload = Payload {
            id: String::from("id"),
            hash: String::from("hash"),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
        };

        let result = log.write_payload(&payload);
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_write_twice_payload_error() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let payload = Payload {
            id: String::from("id"),
            hash: String::from("hash"),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
        };

        let mut result = log.write_payload(&payload);
        assert!(result.is_ok());
        result = log.write_payload(&payload);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            TransparencyLogError::DuplicateId {
                id: String::from("id")
            }
            .to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_read_payload() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let payload = Payload {
            id: String::from("id"),
            hash: String::from("hash"),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
        };

        let result_write = log.write_payload(&payload);
        assert!(result_write.is_ok());

        let result_read = log.read_payload_db("id");
        assert!(result_read.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_read_payload_invalid_id() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let payload = Payload {
            id: String::from("id"),
            hash: String::from("hash"),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
        };

        let result_write = log.write_payload(&payload);
        assert!(result_write.is_ok());

        let result_read = log.read_payload_db("invalid_id");
        assert!(result_read.is_err());
        assert_eq!(
            result_read.err().unwrap().to_string(),
            TransparencyLogError::NotFound {
                id: String::from("invalid_id")
            }
            .to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_add_artifact() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.add_artifact("id", "hash");
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_add_artifact_with_id_containing_forward_slash() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.add_artifact("id/with/slash", "hash");
        assert!(result.is_ok());

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
