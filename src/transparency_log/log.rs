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

use crate::artifact_service::service::PackageType;
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
    #[error("Duplicate ID {package_type_id:?} for type {package_type:?} in transparency log")]
    DuplicateId {
        package_type: PackageType,
        package_type_id: String,
    },
    #[error("ID {package_type_id:?} for type {package_type:?} not found in transparency log")]
    NotFound {
        package_type: PackageType,
        package_type_id: String,
    },
    #[error("Hash Verification failed for ID {id:?}: {invalid_hash:?} vs {actual_hash:?}")]
    InvalidHash {
        id: String,
        invalid_hash: String,
        actual_hash: String,
    },
    #[error("Invalid JSON Payload: {json_error}")]
    InvalidPayload { json_error: String },
}

impl From<serde_json::Error> for TransparencyLogError {
    fn from(err: serde_json::Error) -> TransparencyLogError {
        TransparencyLogError::InvalidPayload {
            json_error: err.to_string(),
        }
    }
}

#[derive(Debug, Clone, strum_macros::Display, Deserialize, Serialize, PartialEq)]
pub enum Operation {
    AddArtifact,
}

#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct Transaction {
    package_type: PackageType,
    package_type_id: String,
    pub hash: String,
    timestamp: u64,
    operation: Operation,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct SignatureEnvelope {
    /// The data that is integrity protected
    transaction: Transaction,
    /// The time at which the signature was generated. This is a part of signed attributes
    signing_timestamp: u64,
    /// The digital signature computed on transaction and signed attributes
    signature: Vec<u8>,
    /// the public key of the signer
    sign_identifier: [u8; 32], //this is identity::ed25519::PublicKey(a byte array in compressed form
}

#[derive(Clone)]
pub struct TransparencyLog {
    storage_path: PathBuf,
    transactions: HashMap<String, String>,
}

impl TransparencyLog {
    pub fn new<P: AsRef<Path>>(repository_path: P) -> Result<Self, anyhow::Error> {
        let mut absolute_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        absolute_path.push("transparency_log");
        Ok(TransparencyLog {
            storage_path: absolute_path,
            transactions: HashMap::new(),
        })
    }

    pub fn add_authorized_node(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn remove_authorized_node(&self) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn request_build(
        &self,
        _package_type: &PackageType,
        _package_type_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn add_artifact(
        &mut self,
        package_type: &PackageType,
        package_type_id: &str,
        hash: &str,
    ) -> anyhow::Result<()> {
        let transaction = Transaction {
            package_type: package_type.clone(),
            package_type_id: package_type_id.to_string(),
            hash: hash.to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operation: Operation::AddArtifact,
        };

        let json_transaction = self.write_transaction(&transaction)?;
        self.transactions.insert(
            format!("{}::{}", package_type, package_type_id),
            json_transaction,
        );

        Ok(())
    }

    pub fn remove_artifact(
        &mut self,
        _package_type: &PackageType,
        _package_type_id: &str,
    ) -> anyhow::Result<()> {
        Ok(())
    }

    pub fn get_artifact(
        &mut self,
        package_type: &PackageType,
        package_type_id: &str,
    ) -> Result<Transaction, TransparencyLogError> {
        if let Some(json_transaction) = self
            .transactions
            .get(&format!("{}::{}", package_type, package_type_id))
        {
            let transaction: Transaction = serde_json::from_str(json_transaction)?;
            Ok(transaction)
        } else {
            Err(TransparencyLogError::NotFound {
                package_type: package_type.clone(),
                package_type_id: package_type_id.to_string(),
            })
        }
    }

    pub fn search_transactions(&self) -> anyhow::Result<Vec<Transaction>> {
        Ok(vec![])
    }

    fn write_transaction(&self, transaction: &Transaction) -> anyhow::Result<String> {
        fs::create_dir_all(&self.storage_path)?;
        let transaction_filename = format!(
            "{}/{}.log",
            self.storage_path.to_str().unwrap(),
            str::replace(&transaction.package_type_id, "/", "_")
        );
        debug!("Storing transaction at: {:?}", transaction_filename);
        match fs::File::options()
            .write(true)
            .create_new(true)
            .open(&transaction_filename)
        {
            Ok(mut transaction_file) => {
                let json_transaction = serde_json::to_string(transaction)?;
                transaction_file.write_all(json_transaction.as_bytes())?;
                Ok(json_transaction)
            }
            Err(e) => match e.kind() {
                io::ErrorKind::AlreadyExists => Err(TransparencyLogError::DuplicateId {
                    package_type: transaction.package_type.clone(),
                    package_type_id: transaction.package_type_id.clone(),
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
    fn create_transaction_log() {
        let package_type = PackageType::Docker;
        let package_type_id = "package_type_id";
        let hash = "hash";
        let timestamp = 1234567890;
        let operation = Operation::AddArtifact;
        let transaction = Transaction {
            package_type: package_type.clone(),
            package_type_id: package_type_id.to_string(),
            hash: hash.to_string(),
            timestamp,
            operation: Operation::AddArtifact,
        };

        assert_eq!(transaction.package_type, package_type);
        assert_eq!(transaction.package_type_id, package_type_id);
        assert_eq!(transaction.hash, hash);
        assert_eq!(transaction.timestamp, timestamp);
        assert_eq!(transaction.operation, operation);
    }

    #[test]
    fn test_new_transparency_log_has_empty_logs() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        assert_eq!(log.transactions.len(), 0);

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_add_artifact() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.add_artifact(&PackageType::Docker, "id", "hash");
        assert!(result.is_ok());

        assert!(log.transactions.contains_key("Docker::id"));

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_add_artifact_with_id_containing_forward_slash() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.add_artifact(&PackageType::Docker, "id/with/slash", "hash");
        assert!(result.is_ok());

        assert!(log.transactions.contains_key("Docker::id/with/slash"));

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_add_duplicate_artifact() {
        let tmp_dir = test_util::tests::setup();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log.add_artifact(&PackageType::Docker, "id", "hash");
        assert!(result.is_ok());

        let result = log.add_artifact(&PackageType::Docker, "id", "hash2");
        assert!(result.is_err());

        test_util::tests::teardown(tmp_dir);
    }
}
