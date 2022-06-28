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
use libp2p::PeerId;
use log::debug;
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::oneshot;
use uuid::Uuid;

#[derive(Debug, Clone, Error, PartialEq)]
pub enum TransparencyLogError {
    #[error("Duplicate ID {package_type_id} for type {package_type} in transparency log")]
    DuplicateId {
        package_type: PackageType,
        package_type_id: String,
    },
    #[error("ID {package_type_id} for type {package_type} not found in transparency log")]
    NotFound {
        package_type: PackageType,
        package_type_id: String,
    },
    #[error("Hash Verification failed for ID {id}: {invalid_hash} vs {actual_hash}")]
    InvalidHash {
        id: String,
        invalid_hash: String,
        actual_hash: String,
    },
    #[error("Invalid operation for ID {id}: {invalid_operation}")]
    InvalidOperation {
        id: String,
        invalid_operation: Operation,
    },
    #[error("Failure while accessing underlying storage: {0}")]
    StorageFailure(String),
}

impl From<io::Error> for TransparencyLogError {
    fn from(err: io::Error) -> TransparencyLogError {
        TransparencyLogError::StorageFailure(err.to_string())
    }
}

impl From<rusqlite::Error> for TransparencyLogError {
    fn from(err: rusqlite::Error) -> TransparencyLogError {
        TransparencyLogError::StorageFailure(err.to_string())
    }
}

#[derive(
    Debug, Clone, strum_macros::Display, strum_macros::EnumString, Deserialize, Serialize, PartialEq,
)]
pub enum Operation {
    AddArtifact,
    RemoveArtifact,
    Unknown,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Transaction {
    id: String,
    package_type: PackageType,
    package_type_id: String,
    pub artifact_hash: String,
    source_hash: String,
    artifact_id: String,
    source_id: String,
    timestamp: u64,
    operation: Operation,
}

pub struct AddArtifactRequest {
    pub package_type: PackageType,
    pub package_type_id: String,
    pub artifact_hash: String,
    pub source_hash: String,
}

/// The transparency log is used by the artifact service to store and retrieve
/// transparency log information about artifacts.
///
/// The transparency log itself depends on the blockchain component to retrieve
/// transactions and to reach consensus on the publication of new transactions.
///
/// It uses a local database to store and index transaction information to simplify
/// access.
pub struct TransparencyLog {
    storage_path: PathBuf,
}

impl TransparencyLog {
    pub fn new<P: AsRef<Path>>(repository_path: P) -> Result<Self, TransparencyLogError> {
        let mut absolute_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        absolute_path.push("transparency_log");
        Ok(TransparencyLog {
            storage_path: absolute_path,
        })
    }

    /// Add a new authorized node to the p2p network.
    pub fn add_authorized_node(&self, _peer_id: PeerId) -> Result<(), TransparencyLogError> {
        Ok(())
    }

    /// Remove a known authorized node from the p2p network.
    pub fn remove_authorized_node(&self, _peer_id: PeerId) -> Result<(), TransparencyLogError> {
        Ok(())
    }

    /// Adds a transaction with the AddArtifact operation.
    pub async fn add_artifact(
        &mut self,
        add_artifact_request: AddArtifactRequest,
        _sender: oneshot::Sender<Result<Transaction, TransparencyLogError>>,
    ) -> Result<(), TransparencyLogError> {
        let transaction = Transaction {
            id: add_artifact_request.package_type_id.to_string(),
            package_type: add_artifact_request.package_type,
            package_type_id: add_artifact_request.package_type_id.to_string(),
            artifact_hash: add_artifact_request.artifact_hash,
            source_hash: add_artifact_request.source_hash,
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operation: Operation::AddArtifact,
        };

        // TODO: Blockchain::add_block(transaction)
        // Wait for callback and resume

        self.write_transaction(&transaction)?;

        Ok(())
    }

    /// Adds a transaction with the RemoveArtifact operation.
    pub fn remove_artifact(
        &mut self,
        _package_type: &PackageType,
        _package_type_id: &str,
    ) -> Result<(), TransparencyLogError> {
        Ok(())
    }

    /// Gets the latest transaction for the specified package of which the
    /// operation is either AddArtifact or RemoveArtifact. Returns an error
    /// when no transaction could be found.
    pub fn get_artifact(
        &mut self,
        package_type: &PackageType,
        package_type_id: &str,
    ) -> Result<Transaction, TransparencyLogError> {
        self.read_transaction(package_type, package_type_id)
    }

    /// Search the transparency log for a list of transactions using the
    /// specified filter.
    pub fn search_transactions(&self) -> Result<Vec<Transaction>, TransparencyLogError> {
        Ok(vec![])
    }

    fn open_db(&self) -> Result<Connection, TransparencyLogError> {
        fs::create_dir_all(&self.storage_path)?;
        let db_storage_path = self.storage_path.to_str().unwrap();
        let conn = Connection::open(db_storage_path.to_owned() + "/transparency_log.db")?;
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS tl_transaction (
                id TEXT PRIMARY KEY,
                package_type TEXT,
                package_type_id TEXT,
                artifact_hash TEXT NOT NULL,
                source_hash TEXT,
                artifact_id TEXT,
                source_id TEXT,
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

    fn write_transaction(&self, transaction: &Transaction) -> Result<(), TransparencyLogError> {
        let conn = self.open_db()?;

        match conn.execute(
            "INSERT INTO tl_transaction (id, package_type, package_type_id, artifact_hash, source_hash, artifact_id, source_id, timestamp, operation) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            [
                transaction.id.to_string(),
                transaction.package_type.to_string(),
                transaction.package_type_id.to_string(),
                transaction.artifact_hash.to_string(),
                transaction.source_hash.to_string(),
                transaction.artifact_id.to_string(),
                transaction.source_id.to_string(),
                transaction.timestamp.to_string(),
                transaction.operation.to_string(),
            ],
        ) {
            Ok(_) => {
                debug!(
                    "Transaction inserted into transparency log with id: {}",
                    transaction.id
                );
                Ok(())
            }
            Err(rusqlite::Error::SqliteFailure(sqlite_error, ref _sqlite_options))
                if sqlite_error.extended_code == rusqlite::ffi::SQLITE_CONSTRAINT_PRIMARYKEY =>
            {
                Err(TransparencyLogError::DuplicateId {
                    package_type: transaction.package_type,
                    package_type_id: transaction.package_type_id.clone(),
                })
            }
            Err(err) => Err(err.into()),
        }
    }

    fn read_transaction(
        &self,
        package_type: &PackageType,
        package_type_id: &str,
    ) -> Result<Transaction, TransparencyLogError> {
        let conn = self.open_db()?;

        let mut stmt = conn.prepare("SELECT * FROM tl_transaction WHERE package_type = :package_type AND package_type_id = :package_type_id;")?;
        let transaction_records = stmt.query_map(
            &[
                (":package_type", &*package_type.to_string()),
                (":package_type_id", package_type_id),
            ],
            |row| {
                Ok(Transaction {
                    id: row.get(0)?,
                    package_type: {
                        let pt: String = row.get(1)?;
                        PackageType::from_str(&pt).unwrap()
                    },
                    package_type_id: row.get(2)?,
                    artifact_hash: row.get(3)?,
                    source_hash: row.get(4)?,
                    artifact_id: row.get(5)?,
                    source_id: row.get(6)?,
                    timestamp: row.get(7)?,
                    operation: {
                        let op: String = row.get(8)?;
                        Operation::from_str(&op).unwrap()
                    },
                })
            },
        )?;

        let mut vector: Vec<Transaction> = Vec::new();
        for transaction_record in transaction_records {
            let record = transaction_record?;
            if record.operation == Operation::AddArtifact
                || record.operation == Operation::RemoveArtifact
            {
                vector.push(record);
            }
        }

        vector.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let latest_record = vector
            .into_iter()
            .next()
            .ok_or(TransparencyLogError::NotFound {
                package_type: *package_type,
                package_type_id: package_type_id.to_string(),
            })?;

        if latest_record.operation == Operation::RemoveArtifact {
            return Err(TransparencyLogError::InvalidOperation {
                id: latest_record.id,
                invalid_operation: latest_record.operation,
            });
        }
        Ok(latest_record)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_util;

    #[test]
    fn create_transaction_log() {
        let id = "id";
        let package_type = PackageType::Docker;
        let package_type_id = "package_type_id";
        let artifact_hash = "artifact_hash";
        let source_hash = "source_hash";
        let artifact_id = Uuid::new_v4().to_string();
        let source_id = Uuid::new_v4().to_string();
        let timestamp = 1234567890;
        let operation = Operation::AddArtifact;
        let transaction = Transaction {
            id: id.to_string(),
            package_type: package_type.clone(),
            package_type_id: package_type_id.to_string(),
            artifact_hash: artifact_hash.to_string(),
            source_hash: source_hash.to_string(),
            artifact_id: artifact_id.to_string(),
            source_id: source_id.to_string(),
            timestamp,
            operation: Operation::AddArtifact,
        };

        assert_eq!(transaction.id, id);
        assert_eq!(transaction.package_type, package_type);
        assert_eq!(transaction.package_type_id, package_type_id);
        assert_eq!(transaction.artifact_hash, artifact_hash);
        assert_eq!(transaction.source_hash, source_hash);
        assert_eq!(transaction.artifact_id, artifact_id);
        assert_eq!(transaction.source_id, source_id);
        assert_eq!(transaction.timestamp, timestamp);
        assert_eq!(transaction.operation, operation);
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
    fn test_write_transaction() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let transaction = Transaction {
            id: String::from("id"),
            package_type: PackageType::Maven2,
            package_type_id: String::from("package_type_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: String::from(Uuid::new_v4().to_string()),
            source_id: String::from(Uuid::new_v4().to_string()),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
        };

        let result = log.write_transaction(&transaction);
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_write_twice_transaction_error() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let transaction = Transaction {
            id: String::from("id"),
            package_type: PackageType::Maven2,
            package_type_id: String::from("package_type_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: String::from(Uuid::new_v4().to_string()),
            source_id: String::from(Uuid::new_v4().to_string()),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
        };

        let mut result = log.write_transaction(&transaction);
        assert!(result.is_ok());
        result = log.write_transaction(&transaction);
        assert!(result.is_err());
        assert_eq!(
            result.err().unwrap().to_string(),
            TransparencyLogError::DuplicateId {
                package_type: PackageType::Maven2,
                package_type_id: String::from("package_type_id"),
            }
            .to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_read_transaction() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let transaction = Transaction {
            id: String::from("id"),
            package_type: PackageType::Maven2,
            package_type_id: String::from("package_type_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: String::from(Uuid::new_v4().to_string()),
            source_id: String::from(Uuid::new_v4().to_string()),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
        };

        let result_write = log.write_transaction(&transaction);
        assert!(result_write.is_ok());

        let result_read = log.read_transaction(&PackageType::Maven2, "package_type_id");
        assert!(result_read.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_read_transaction_invalid_id() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let transaction = Transaction {
            id: String::from("id"),
            package_type: PackageType::Maven2,
            package_type_id: String::from("package_type_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: String::from(Uuid::new_v4().to_string()),
            source_id: String::from(Uuid::new_v4().to_string()),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
        };

        let result_write = log.write_transaction(&transaction);
        assert!(result_write.is_ok());

        let result_read = log.read_transaction(&PackageType::Maven2, "invalid_package_type_id");
        assert!(result_read.is_err());
        assert_eq!(
            result_read.err().unwrap().to_string(),
            TransparencyLogError::NotFound {
                package_type: PackageType::Maven2,
                package_type_id: String::from("invalid_package_type_id"),
            }
            .to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_read_latest_transaction() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let transaction1 = Transaction {
            id: String::from("id1"),
            package_type: PackageType::Maven2,
            package_type_id: String::from("package_type_id1"),
            artifact_hash: String::from("artifact_hash1"),
            source_hash: String::from("source_hash1"),
            artifact_id: String::from(Uuid::new_v4().to_string()),
            source_id: String::from(Uuid::new_v4().to_string()),
            timestamp: 10000000,
            operation: Operation::AddArtifact,
        };

        let result_write1 = log.write_transaction(&transaction1);
        assert!(result_write1.is_ok());

        let transaction2 = Transaction {
            id: String::from("id2"),
            package_type: PackageType::Maven2,
            package_type_id: String::from("package_type_id2"),
            artifact_hash: String::from("artifact_hash2"),
            source_hash: String::from("source_hash2"),
            artifact_id: String::from(Uuid::new_v4().to_string()),
            source_id: String::from(Uuid::new_v4().to_string()),
            timestamp: 20000000,
            operation: Operation::AddArtifact,
        };

        let result_write2 = log.write_transaction(&transaction2);
        assert!(result_write2.is_ok());

        let result_read = log.read_transaction(&PackageType::Maven2, "package_type_id2");
        assert!(result_read.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_read_remove_artifact_transaction() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let transaction = Transaction {
            id: String::from("id"),
            package_type: PackageType::Maven2,
            package_type_id: String::from("package_type_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: String::from(Uuid::new_v4().to_string()),
            source_id: String::from(Uuid::new_v4().to_string()),
            timestamp: 10000000,
            operation: Operation::RemoveArtifact,
        };

        let result_write = log.write_transaction(&transaction);
        assert!(result_write.is_ok());

        let result_read = log.read_transaction(&PackageType::Maven2, "package_type_id");
        assert!(result_read.is_err());
        assert_eq!(
            result_read.err().unwrap().to_string(),
            TransparencyLogError::InvalidOperation {
                id: String::from("id"),
                invalid_operation: Operation::RemoveArtifact,
            }
            .to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[test]
    fn test_read_unknown_transaction() {
        let tmp_dir = test_util::tests::setup();

        let log = TransparencyLog::new(&tmp_dir).unwrap();

        let transaction = Transaction {
            id: String::from("id"),
            package_type: PackageType::Maven2,
            package_type_id: String::from("package_type_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: String::from(Uuid::new_v4().to_string()),
            source_id: String::from(Uuid::new_v4().to_string()),
            timestamp: 10000000,
            operation: Operation::Unknown,
        };

        let result_write = log.write_transaction(&transaction);
        assert!(result_write.is_ok());

        let result_read = log.read_transaction(&PackageType::Maven2, "package_type_id");
        assert!(result_read.is_err());
        assert_eq!(
            result_read.err().unwrap().to_string(),
            TransparencyLogError::NotFound {
                package_type: PackageType::Maven2,
                package_type_id: String::from("package_type_id"),
            }
            .to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_add_artifact() {
        let tmp_dir = test_util::tests::setup();

        let (sender, _receiver) = oneshot::channel();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log
            .add_artifact(
                AddArtifactRequest {
                    package_type: PackageType::Docker,
                    package_type_id: "package_type_id".to_string(),
                    artifact_hash: "artifact_hash".to_string(),
                    source_hash: "source_hash".to_string(),
                },
                sender,
            )
            .await;
        println!("RESULT: {:?}", result);
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_add_duplicate_artifact() {
        let tmp_dir = test_util::tests::setup();

        let (sender1, _receiver) = oneshot::channel();
        let (sender2, _receiver) = oneshot::channel();

        let mut log = TransparencyLog::new(&tmp_dir).unwrap();

        let result = log
            .add_artifact(
                AddArtifactRequest {
                    package_type: PackageType::Docker,
                    package_type_id: "package_type_id".to_string(),
                    artifact_hash: "artifact_hash".to_string(),
                    source_hash: "source_hash".to_string(),
                },
                sender1,
            )
            .await;
        assert!(result.is_ok());

        let result = log
            .add_artifact(
                AddArtifactRequest {
                    package_type: PackageType::Docker,
                    package_type_id: "package_type_id".to_string(),
                    artifact_hash: "artifact_hash2".to_string(),
                    source_hash: "source_hash2".to_string(),
                },
                sender2,
            )
            .await;
        assert!(result.is_err());

        test_util::tests::teardown(tmp_dir);
    }
}
