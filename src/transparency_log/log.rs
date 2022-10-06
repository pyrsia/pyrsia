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

use crate::artifact_service::model::PackageType;
use crate::blockchain_service::service::BlockchainService;
use libp2p::PeerId;
use log::{debug, error};
use pyrsia_blockchain_network::error::BlockchainError;
use rusqlite::types::{ToSqlOutput, Value};
use rusqlite::{params, Connection, ToSql};
use serde::{Deserialize, Serialize};
use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::str::FromStr;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
use tokio::sync::Mutex;
use uuid::Uuid;

#[derive(Debug, Error)]
pub enum TransparencyLogError {
    #[error("TransparencyLog with ID {id} not found")]
    LogNotFound { id: String },
    #[error(
        "Artifact ID {package_specific_artifact_id} for type {package_type} not found in transparency log"
    )]
    ArtifactNotFound {
        package_type: PackageType,
        package_specific_artifact_id: String,
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
    DatabaseFailure(#[from] rusqlite::Error),
    #[error("Failure while accessing underlying storage: {0}")]
    StorageFailure(#[from] io::Error),
    #[error("Failure while adding block to the blockchain: {0}")]
    BlockchainFailure(#[from] BlockchainError),
}

#[derive(
    Debug,
    Clone,
    strum_macros::Display,
    strum_macros::EnumString,
    Deserialize,
    Serialize,
    Eq,
    PartialEq,
)]
pub enum Operation {
    AddArtifact,
    RemoveArtifact,
    AddNode,
    RemoveNode,
}

impl ToSql for Operation {
    fn to_sql(&self) -> rusqlite::Result<ToSqlOutput<'_>> {
        Ok(ToSqlOutput::from(self.to_string()))
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransparencyLog {
    pub id: String,
    pub package_type: Option<PackageType>,
    pub package_specific_id: String,
    pub num_artifacts: u32,
    pub package_specific_artifact_id: String,
    pub artifact_hash: String,
    source_hash: String,
    pub artifact_id: String,
    source_id: String,
    timestamp: u64,
    pub operation: Operation,
    pub node_id: String,
    node_public_key: String,
}

#[derive(Debug)]
pub struct AddArtifactRequest {
    pub package_type: PackageType,
    pub package_specific_id: String,
    pub num_artifacts: u32,
    pub package_specific_artifact_id: String,
    pub artifact_hash: String,
}

pub struct AuthorizedNode {
    pub id: String,
    pub public_key: String,
}

/// The transparency log service is used by the artifact service to store and retrieve
/// transparency log information about artifacts.
///
/// The transparency log itself depends on the blockchain component to retrieve
/// transactions and to reach consensus on the publication of new transactions.
///
/// It uses a local database to store and index transparency log information to simplify
/// access.
#[derive(Clone)]
pub struct TransparencyLogService {
    storage_path: PathBuf,
    blockchain_service: Arc<Mutex<BlockchainService>>,
}

impl TransparencyLogService {
    pub fn new<P: AsRef<Path>>(
        repository_path: P,
        blockchain_service: Arc<Mutex<BlockchainService>>,
    ) -> Result<Self, TransparencyLogError> {
        let mut absolute_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        absolute_path.push("transparency_log");
        Ok(TransparencyLogService {
            storage_path: absolute_path,
            blockchain_service,
        })
    }

    /// Add a new authorized node to the p2p network.
    pub async fn add_authorized_node(&self, peer_id: PeerId) -> Result<(), TransparencyLogError> {
        let transparency_log = TransparencyLog {
            id: Uuid::new_v4().to_string(),
            package_type: None,
            package_specific_id: String::from(""),
            num_artifacts: 0,
            package_specific_artifact_id: String::from(""),
            artifact_hash: String::from(""),
            source_hash: String::from(""),
            artifact_id: String::from(""),
            source_id: String::from(""),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operation: Operation::AddNode,
            node_id: peer_id.to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let payload = serde_json::to_string(&transparency_log).unwrap();
        self.blockchain_service
            .lock()
            .await
            .add_payload(payload.into_bytes())
            .await?;

        self.write_transparency_log(&transparency_log)
    }

    /// Remove a known authorized node from the p2p network.
    pub fn remove_authorized_node(&self, _peer_id: PeerId) -> Result<(), TransparencyLogError> {
        Ok(())
    }

    /// Adds a transparency log with the AddArtifact operation.
    pub async fn add_artifact(
        &mut self,
        add_artifact_request: AddArtifactRequest,
    ) -> Result<TransparencyLog, TransparencyLogError> {
        let transparency_log = TransparencyLog {
            id: Uuid::new_v4().to_string(),
            package_type: Some(add_artifact_request.package_type),
            package_specific_id: add_artifact_request.package_specific_id.clone(),
            num_artifacts: add_artifact_request.num_artifacts,
            package_specific_artifact_id: add_artifact_request.package_specific_artifact_id.clone(),
            artifact_hash: add_artifact_request.artifact_hash,
            source_hash: "".to_owned(),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let payload = serde_json::to_string(&transparency_log).unwrap();
        self.blockchain_service
            .lock()
            .await
            .add_payload(payload.into_bytes())
            .await?;

        Ok(transparency_log)
    }

    /// Adds a transparency log with the RemoveArtifact operation.
    pub fn remove_artifact(
        &mut self,
        _package_type: &PackageType,
        _package_specific_id: &str,
    ) -> Result<(), TransparencyLogError> {
        Ok(())
    }

    /// Gets the latest transparency log for the specified package of which the
    /// operation is either AddArtifact or RemoveArtifact. Returns an error
    /// when no transparency log could be found.
    pub fn get_artifact(
        &mut self,
        package_type: &PackageType,
        package_specific_artifact_id: &str,
    ) -> Result<TransparencyLog, TransparencyLogError> {
        self.read_transparency_log(package_type, package_specific_artifact_id)
    }

    /// Search the transparency log database for a list of transparency logs using the
    /// specified filter.
    pub fn search_transparency_logs(
        &self,
        package_type: &PackageType,
        package_specific_id: &str,
    ) -> Result<Vec<TransparencyLog>, TransparencyLogError> {
        self.read_transparency_logs(package_type, package_specific_id)
    }

    /// Gets a list of transparency logs of which the operation is AddNode. Returns an error
    /// when no transparency log could be found.
    pub fn get_authorized_nodes(&self) -> Result<Vec<TransparencyLog>, TransparencyLogError> {
        self.find_added_nodes()
    }

    fn open_db(&self) -> Result<Connection, TransparencyLogError> {
        let mut db_path = self.storage_path.to_owned();
        fs::create_dir_all(db_path.clone())?;
        db_path.push("transparency_log.db");
        let conn = Connection::open(db_path)?;
        match conn.execute(
            "CREATE TABLE IF NOT EXISTS TRANSPARENCYLOG (
                id TEXT PRIMARY KEY,
                package_type TEXT,
                package_specific_id TEXT,
                num_artifacts INTEGER,
                package_specific_artifact_id TEXT,
                artifact_hash TEXT,
                source_hash TEXT,
                artifact_id TEXT,
                source_id TEXT,
                timestamp INTEGER,
                operation TEXT NOT NULL,
                node_id TEXT,
                node_public_key TEXT
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

    pub fn find_transparency_log(&self, id: &str) -> Result<TransparencyLog, TransparencyLogError> {
        let query = ["SELECT * FROM TRANSPARENCYLOG WHERE id = '", id, "';"];

        let results = self.process_query(query.join("").as_str())?;

        if results.len() == 1 {
            Ok(results.into_iter().next().unwrap())
        } else {
            Err(TransparencyLogError::LogNotFound { id: id.to_owned() })
        }
    }

    pub fn write_transparency_log(
        &self,
        transparency_log: &TransparencyLog,
    ) -> Result<(), TransparencyLogError> {
        let conn = self.open_db()?;

        match conn.execute(
            "INSERT INTO TRANSPARENCYLOG (id, package_type, package_specific_id, num_artifacts, package_specific_artifact_id, artifact_hash, source_hash, artifact_id, source_id, timestamp, operation, node_id, node_public_key) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13)",
            params![
                transparency_log.id,
                transparency_log.package_type,
                transparency_log.package_specific_id,
                transparency_log.num_artifacts,
                transparency_log.package_specific_artifact_id,
                transparency_log.artifact_hash,
                transparency_log.source_hash,
                transparency_log.artifact_id,
                transparency_log.source_id,
                transparency_log.timestamp,
                transparency_log.operation,
                transparency_log.node_id,
                transparency_log.node_public_key,
            ],
        ) {
            Ok(_) => {
                debug!(
                    "Transparency log inserted into database with id: {}",
                    transparency_log.id
                );
                Ok(())
            }
            Err(err) => Err(err.into()),
        }
    }

    fn read_transparency_log(
        &self,
        package_type: &PackageType,
        package_specific_artifact_id: &str,
    ) -> Result<TransparencyLog, TransparencyLogError> {
        let query = [
            "SELECT * FROM TRANSPARENCYLOG WHERE package_type = '",
            &*package_type.to_string(),
            "' AND package_specific_artifact_id = '",
            package_specific_artifact_id,
            "';",
        ];
        let results = self.process_query(query.join("").as_str())?;

        let mut vector: Vec<TransparencyLog> = Vec::new();
        for record in results {
            if record.operation == Operation::AddArtifact
                || record.operation == Operation::RemoveArtifact
            {
                vector.push(record);
            }
        }

        vector.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        let latest_record =
            vector
                .into_iter()
                .next()
                .ok_or(TransparencyLogError::ArtifactNotFound {
                    package_type: *package_type,
                    package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                })?;

        if latest_record.operation == Operation::RemoveArtifact {
            return Err(TransparencyLogError::InvalidOperation {
                id: latest_record.id,
                invalid_operation: latest_record.operation,
            });
        }
        Ok(latest_record)
    }

    fn read_transparency_logs(
        &self,
        package_type: &PackageType,
        package_specific_id: &str,
    ) -> Result<Vec<TransparencyLog>, TransparencyLogError> {
        let query = [
            "SELECT * FROM TRANSPARENCYLOG WHERE package_type = '",
            &*package_type.to_string(),
            "' AND package_specific_id = '",
            package_specific_id,
            "';",
        ];
        let results = self.process_query(query.join("").as_str())?;

        let mut vector: Vec<TransparencyLog> = Vec::new();
        for record in results {
            if record.operation == Operation::AddArtifact
                || record.operation == Operation::RemoveArtifact
            {
                vector.push(record);
            }
        }

        vector.sort_by(|a, b| a.timestamp.cmp(&b.timestamp));

        Ok(vector)
    }

    fn find_added_nodes(&self) -> Result<Vec<TransparencyLog>, TransparencyLogError> {
        let query = [
            "SELECT * FROM TRANSPARENCYLOG WHERE operation = '",
            &Operation::AddNode.to_string(),
            "' OR operation = '",
            &Operation::RemoveNode.to_string(),
            "';",
        ];
        let results = self.process_query(query.join("").as_str())?;

        let mut vector_added: Vec<TransparencyLog> = Vec::new();
        let mut vector_removed: Vec<TransparencyLog> = Vec::new();
        for record in results {
            if record.operation == Operation::AddNode {
                vector_added.push(record);
            } else if record.operation == Operation::RemoveNode {
                vector_removed.push(record);
            }
        }
        for removed_record in vector_removed {
            vector_added.retain(|x| x.node_id != removed_record.node_id)
        }

        Ok(vector_added)
    }

    fn process_query(&self, query: &str) -> Result<Vec<TransparencyLog>, TransparencyLogError> {
        let conn = self.open_db()?;
        let mut stmt = conn.prepare(query)?;

        let transparency_log_records = stmt.query_map([], |row| {
            Ok(TransparencyLog {
                id: row.get(0)?,
                package_type: {
                    let value: Value = row.get(1)?;
                    match value {
                        Value::Text(pt) => Ok(Some(PackageType::from_str(&pt).unwrap())),
                        Value::Null => Ok(None),
                        _ => Err(rusqlite::Error::InvalidColumnType(
                            1,
                            "package_type".to_owned(),
                            value.data_type(),
                        )),
                    }?
                },
                package_specific_id: row.get(2)?,
                num_artifacts: row.get(3)?,
                package_specific_artifact_id: row.get(4)?,
                artifact_hash: row.get(5)?,
                source_hash: row.get(6)?,
                artifact_id: row.get(7)?,
                source_id: row.get(8)?,
                timestamp: row.get(9)?,
                operation: {
                    let op: String = row.get(10)?;
                    Operation::from_str(&op).unwrap()
                },
                node_id: row.get(11)?,
                node_public_key: row.get(12)?,
            })
        })?;

        let mut vector: Vec<TransparencyLog> = Vec::new();
        for transparency_log_record in transparency_log_records {
            vector.push(transparency_log_record?);
        }

        Ok(vector)
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::{network::client::Client, util::test_util};
    use libp2p::identity;
    use libp2p::identity::Keypair;
    use tokio::sync::mpsc;

    fn create_p2p_client(keypair: identity::Keypair) -> Client {
        let (command_sender, _command_receiver) = mpsc::channel(1);
        Client {
            sender: command_sender,
            local_peer_id: keypair.public().to_peer_id(),
        }
    }

    async fn create_transparency_log_service(
        artifact_path: impl AsRef<Path>,
    ) -> TransparencyLogService {
        let ed25519_keypair = identity::ed25519::Keypair::generate();
        let p2p_client = create_p2p_client(identity::Keypair::Ed25519(ed25519_keypair.clone()));

        let blockchain_service = BlockchainService::init_first_blockchain_node(
            &ed25519_keypair,
            &ed25519_keypair,
            p2p_client,
            &artifact_path,
        )
        .await
        .expect("Creating BlockchainService failed");

        TransparencyLogService::new(artifact_path, Arc::new(Mutex::new(blockchain_service)))
            .unwrap()
    }

    #[test]
    fn create_transparency_log() {
        let id = "id";
        let package_type = Some(PackageType::Docker);
        let package_specific_id = "package_specific_id";
        let num_artifacts = 10;
        let package_specific_artifact_id = "package_specific_artifact_id";
        let artifact_hash = "artifact_hash";
        let source_hash = "source_hash";
        let artifact_id = Uuid::new_v4().to_string();
        let source_id = Uuid::new_v4().to_string();
        let timestamp = 1234567890;
        let operation = Operation::AddArtifact;
        let node_id = Uuid::new_v4().to_string();
        let node_public_key = Uuid::new_v4().to_string();
        let transparency_log = TransparencyLog {
            id: id.to_string(),
            package_type,
            package_specific_id: package_specific_id.to_string(),
            num_artifacts,
            package_specific_artifact_id: package_specific_artifact_id.to_owned(),
            artifact_hash: artifact_hash.to_owned(),
            source_hash: source_hash.to_owned(),
            artifact_id: artifact_id.to_owned(),
            source_id: source_id.to_owned(),
            timestamp,
            operation: Operation::AddArtifact,
            node_id: node_id.to_owned(),
            node_public_key: node_public_key.to_owned(),
        };

        assert_eq!(transparency_log.id, id);
        assert_eq!(transparency_log.package_type, package_type);
        assert_eq!(transparency_log.package_specific_id, package_specific_id);
        assert_eq!(transparency_log.num_artifacts, num_artifacts);
        assert_eq!(
            transparency_log.package_specific_artifact_id,
            package_specific_artifact_id
        );
        assert_eq!(transparency_log.artifact_hash, artifact_hash);
        assert_eq!(transparency_log.source_hash, source_hash);
        assert_eq!(transparency_log.artifact_id, artifact_id);
        assert_eq!(transparency_log.source_id, source_id);
        assert_eq!(transparency_log.timestamp, timestamp);
        assert_eq!(transparency_log.operation, operation);
        assert_eq!(transparency_log.node_id, node_id);
        assert_eq!(transparency_log.node_public_key, node_public_key);
    }

    #[tokio::test]
    async fn test_open_db() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let result = log.open_db();
        assert!(result.is_ok());

        let conn = result.unwrap();
        let mut path = log.storage_path;
        path.push("transparency_log.db");
        assert_eq!(conn.path().unwrap(), path.as_path());

        let close_result = conn.close();
        assert!(close_result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_write_tranparency_log() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result = log.write_transparency_log(&transparency_log);
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_write_twice_transparency_log_error() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let mut result = log.write_transparency_log(&transparency_log);
        assert!(result.is_ok());
        result = log.write_transparency_log(&transparency_log);
        assert!(result.is_err());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_find_transparency_log() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write = log.write_transparency_log(&transparency_log);
        assert!(result_write.is_ok());

        let result_find = log.find_transparency_log("id");
        assert!(result_find.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_find_transparency_log_not_found() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write = log.write_transparency_log(&transparency_log);
        assert!(result_write.is_ok());

        let find_error = log
            .find_transparency_log("unknown_id")
            .expect_err("Find transparency log should have failed.");
        match find_error {
            TransparencyLogError::LogNotFound { id } => {
                assert_eq!("unknown_id".to_owned(), id);
            }
            e => {
                panic!("Invalid Error encountered: {:?}", e);
            }
        }

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_read_transparency_log() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write = log.write_transparency_log(&transparency_log);
        assert!(result_write.is_ok());

        let result_read =
            log.read_transparency_log(&PackageType::Maven2, "package_specific_artifact_id");
        assert!(result_read.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_read_transparency_log_invalid_id() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write = log.write_transparency_log(&transparency_log);
        assert!(result_write.is_ok());

        let result_read =
            log.read_transparency_log(&PackageType::Maven2, "invalid_package_specific_artifact_id");
        assert!(result_read.is_err());
        assert_eq!(
            result_read.err().unwrap().to_string(),
            TransparencyLogError::ArtifactNotFound {
                package_type: PackageType::Maven2,
                package_specific_artifact_id: String::from("invalid_package_specific_artifact_id"),
            }
            .to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_read_latest_transparency_log() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log1 = TransparencyLog {
            id: String::from("id1"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash1"),
            source_hash: String::from("source_hash1"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 10000000,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write1 = log.write_transparency_log(&transparency_log1);
        assert!(result_write1.is_ok());

        let transparency_log2 = TransparencyLog {
            id: String::from("id2"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id2"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id2"),
            artifact_hash: String::from("artifact_hash2"),
            source_hash: String::from("source_hash2"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 20000000,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write2 = log.write_transparency_log(&transparency_log2);
        assert!(result_write2.is_ok());

        let result_read =
            log.read_transparency_log(&PackageType::Maven2, "package_specific_artifact_id2");
        assert!(result_read.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_read_transparency_logs() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log1 = TransparencyLog {
            id: String::from("id1"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash1"),
            source_hash: String::from("source_hash1"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 10000000,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write1 = log.write_transparency_log(&transparency_log1);
        assert!(result_write1.is_ok());

        let transparency_log2 = TransparencyLog {
            id: String::from("id2"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id2"),
            artifact_hash: String::from("artifact_hash2"),
            source_hash: String::from("source_hash2"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 20000000,
            operation: Operation::AddArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write2 = log.write_transparency_log(&transparency_log2);
        assert!(result_write2.is_ok());

        let result_read1 = log.read_transparency_logs(&PackageType::Maven2, "package_specific_id");
        assert!(result_read1.is_ok());
        assert_eq!(result_read1.unwrap().len(), 2);

        let result_read2 =
            log.read_transparency_logs(&PackageType::Maven2, "other_package_specific_id");
        assert!(result_read2.is_ok());
        assert_eq!(result_read2.unwrap().len(), 0);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_read_remove_artifact_transparency_log() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: Some(PackageType::Maven2),
            package_specific_id: String::from("package_specific_id"),
            num_artifacts: 8,
            package_specific_artifact_id: String::from("package_specific_artifact_id"),
            artifact_hash: String::from("artifact_hash"),
            source_hash: String::from("source_hash"),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: 10000000,
            operation: Operation::RemoveArtifact,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write = log.write_transparency_log(&transparency_log);
        assert!(result_write.is_ok());

        let result_read =
            log.read_transparency_log(&PackageType::Maven2, "package_specific_artifact_id");
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

    #[tokio::test]
    async fn test_add_artifact() {
        let tmp_dir = test_util::tests::setup();

        let mut log = create_transparency_log_service(&tmp_dir).await;

        let result = log
            .add_artifact(AddArtifactRequest {
                package_type: PackageType::Docker,
                package_specific_id: "package_specific_id".to_owned(),
                num_artifacts: 8,
                package_specific_artifact_id: "package_specific_artifact_id".to_owned(),
                artifact_hash: "artifact_hash".to_owned(),
            })
            .await;
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_get_authorized_nodes_empty() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let result_read = log.get_authorized_nodes();
        assert!(result_read.is_ok());
        assert_eq!(result_read.unwrap().len(), 0);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_add_authorized_nodes() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let peer_id = Keypair::generate_ed25519().public().to_peer_id();

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: None,
            package_specific_id: String::from(""),
            num_artifacts: 0,
            package_specific_artifact_id: String::from(""),
            artifact_hash: String::from(""),
            source_hash: String::from(""),
            artifact_id: String::from(""),
            source_id: String::from(""),
            timestamp: 10000000,
            operation: Operation::AddNode,
            node_id: peer_id.clone().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_add = log.add_authorized_node(peer_id).await;
        assert!(result_add.is_ok());

        let result_read = log.get_authorized_nodes();
        assert!(result_read.is_ok());
        let vec = result_read.unwrap();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec.get(0).unwrap().node_id, transparency_log.node_id);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_get_authorized_nodes_add() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log = TransparencyLog {
            id: String::from("id"),
            package_type: None,
            package_specific_id: String::from(""),
            num_artifacts: 0,
            package_specific_artifact_id: String::from(""),
            artifact_hash: String::from(""),
            source_hash: String::from(""),
            artifact_id: String::from(""),
            source_id: String::from(""),
            timestamp: 10000000,
            operation: Operation::AddNode,
            node_id: String::from("node_id"),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write = log.write_transparency_log(&transparency_log);
        assert!(result_write.is_ok());

        let result_read = log.get_authorized_nodes();
        assert!(result_read.is_ok());
        let vec = result_read.unwrap();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec.get(0).unwrap().node_id, "node_id");

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_get_authorized_nodes_add_and_remove() {
        let tmp_dir = test_util::tests::setup();

        let log = create_transparency_log_service(&tmp_dir).await;

        let transparency_log1 = TransparencyLog {
            id: String::from("id1"),
            package_type: None,
            package_specific_id: String::from(""),
            num_artifacts: 8,
            package_specific_artifact_id: String::from(""),
            artifact_hash: String::from(""),
            source_hash: String::from(""),
            artifact_id: String::from(""),
            source_id: String::from(""),
            timestamp: 10000000,
            operation: Operation::AddNode,
            node_id: String::from("node_id1"),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write1 = log.write_transparency_log(&transparency_log1);
        assert!(result_write1.is_ok());

        let transparency_log2 = TransparencyLog {
            id: String::from("id2"),
            package_type: None,
            package_specific_id: String::from(""),
            num_artifacts: 8,
            package_specific_artifact_id: String::from(""),
            artifact_hash: String::from(""),
            source_hash: String::from(""),
            artifact_id: String::from(""),
            source_id: String::from(""),
            timestamp: 20000000,
            operation: Operation::AddNode,
            node_id: String::from("node_id2"),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write2 = log.write_transparency_log(&transparency_log2);
        assert!(result_write2.is_ok());

        let transparency_log3 = TransparencyLog {
            id: String::from("id3"),
            package_type: None,
            package_specific_id: String::from(""),
            num_artifacts: 8,
            package_specific_artifact_id: String::from(""),
            artifact_hash: String::from(""),
            source_hash: String::from(""),
            artifact_id: String::from(""),
            source_id: String::from(""),
            timestamp: 30000000,
            operation: Operation::RemoveNode,
            node_id: String::from("node_id1"),
            node_public_key: Uuid::new_v4().to_string(),
        };

        let result_write3 = log.write_transparency_log(&transparency_log3);
        assert!(result_write3.is_ok());

        let result_read = log.get_authorized_nodes();
        assert!(result_read.is_ok());
        let vec = result_read.unwrap();
        assert_eq!(vec.len(), 1);
        assert_eq!(vec.get(0).unwrap().node_id, "node_id2");

        test_util::tests::teardown(tmp_dir);
    }
}
