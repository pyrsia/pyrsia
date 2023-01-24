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
use crate::blockchain_service::event::BlockchainEventClient;
use libp2p::core::ParseError;
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
use std::time::{SystemTime, UNIX_EPOCH};
use thiserror::Error;
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
    #[error(
        "Artifact ID {package_specific_id} for type {package_type} already exists in transparency log"
    )]
    ArtifactAlreadyExists {
        package_type: PackageType,
        package_specific_id: String,
    },
    #[error("Node with node ID {node_id} already exists in transparency log")]
    NodeAlreadyExists { node_id: String },
    #[error("Hash Verification failed for ID {id}: {invalid_hash} vs {actual_hash}")]
    InvalidHash {
        id: String,
        invalid_hash: String,
        actual_hash: String,
    },
    #[error("Invalid peerId format: {0}")]
    InvalidNodePeerIDFormat(#[from] ParseError),
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
    #[error("Failure while generating JSON from transparency log: {0}")]
    SerdeJsonFailure(#[from] serde_json::error::Error),
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
    blockchain_event_client: BlockchainEventClient,
}

impl TransparencyLogService {
    pub fn new<P: AsRef<Path>>(
        repository_path: P,
        blockchain_event_client: BlockchainEventClient,
    ) -> Result<Self, TransparencyLogError> {
        let mut absolute_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        absolute_path.push("transparency_log");
        Ok(TransparencyLogService {
            storage_path: absolute_path,
            blockchain_event_client,
        })
    }

    /// Add a new authorized node to the p2p network.
    pub async fn add_authorized_node(&self, peer_id: PeerId) -> Result<(), TransparencyLogError> {
        self.verify_node_can_be_added_to_transparency_logs(&peer_id.to_string())?;

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

        let payload = serde_json::to_string(&transparency_log)?;
        self.blockchain_event_client
            .add_block(payload.into_bytes())
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

        let payload = serde_json::to_string(&transparency_log)?;
        self.blockchain_event_client
            .add_block(payload.into_bytes())
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

    /// Verifies that a specified package can be added to the transparency log database.
    /// For that, the database should not contain the artifact yet, or if it does,
    /// its latest operation is not RemoveArtifact. If that is not the case,
    /// an ArtifactAlreadyExists error is returned
    pub fn verify_package_can_be_added_to_transparency_logs(
        &self,
        package_type: &PackageType,
        package_specific_id: &str,
    ) -> Result<(), TransparencyLogError> {
        let result = self.read_transparency_logs(package_type, package_specific_id);
        match result.as_ref().ok().and(result.as_ref().unwrap().last()) {
            None => {
                // no logs or error, can be added
                Ok(())
            }
            Some(t) => {
                if t.operation == Operation::RemoveArtifact {
                    // was removed, can be added
                    Ok(())
                } else {
                    // the artifact exists, can't be added again
                    Err(TransparencyLogError::ArtifactAlreadyExists {
                        package_type: package_type.to_owned(),
                        package_specific_id: package_specific_id.to_owned(),
                    })
                }
            }
        }
    }

    /// Get a list of auth node PeerID. Return an error when no PeerID could be found.
    pub fn get_authorized_nodes(&self) -> Result<Vec<PeerId>, TransparencyLogError> {
        Ok(self
            .find_added_nodes()?
            .iter()
            //Get PeerId in the correct format, ignoring parsing errors
            .flat_map(|node| PeerId::from_str(&node.node_id))
            .collect::<Vec<PeerId>>())
    }

    /// Verifies that a specified node can be added to the transparency log database.
    /// For that, the database should not contain the node yet. If that is not the case,
    /// an NodeAlreadyExists error is returned
    pub fn verify_node_can_be_added_to_transparency_logs(
        &self,
        peer_id: &str,
    ) -> Result<(), TransparencyLogError> {
        match self.get_authorized_nodes().ok() {
            None => {
                // error, can be added
                Ok(())
            }
            Some(nodes) => {
                for node in nodes {
                    if node.eq(&PeerId::from_str(peer_id)?) {
                        return Err(TransparencyLogError::NodeAlreadyExists {
                            node_id: peer_id.to_owned(),
                        });
                    }
                }
                // was removed, can be added
                Ok(())
            }
        }
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
    use crate::blockchain_service::event::BlockchainEvent;
    use crate::util::test_util;
    use libp2p::identity::Keypair;

    #[test]
    fn create_transparency_log() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(tmp_dir);

        let ps_art_id = "test_package_specific_artifact_id";

        let transparency_log = TransparencyLog {
            id: "test_id".to_string(),
            package_type: Some(PackageType::Docker),
            package_specific_id: "test_package_specific_id".to_string(),
            num_artifacts: 10,
            package_specific_artifact_id: ps_art_id.to_owned(),
            artifact_hash: "test_artifact_hash".to_owned(),
            source_hash: "test_source_hash".to_owned(),
            artifact_id: "test_artifact_id".to_owned(),
            source_id: "test_source_id".to_owned(),
            timestamp: 1234567890,
            operation: Operation::AddArtifact,
            node_id: "test_node_id".to_owned(),
            node_public_key: "test_node_public_key".to_owned(),
        };

        assert!(log.write_transparency_log(&transparency_log).is_ok());

        let res = log
            .read_transparency_log(&PackageType::Docker, ps_art_id)
            .unwrap();

        assert_eq!(transparency_log.id, res.id);
        assert_eq!(transparency_log.package_type, res.package_type);
        assert_eq!(
            transparency_log.package_specific_id,
            res.package_specific_id
        );
        assert_eq!(transparency_log.num_artifacts, res.num_artifacts);
        assert_eq!(
            transparency_log.package_specific_artifact_id,
            res.package_specific_artifact_id
        );
        assert_eq!(transparency_log.artifact_hash, res.artifact_hash);
        assert_eq!(transparency_log.source_hash, res.source_hash);
        assert_eq!(transparency_log.artifact_id, res.artifact_id);
        assert_eq!(transparency_log.source_id, res.source_id);
        assert_eq!(transparency_log.timestamp, res.timestamp);
        assert_eq!(transparency_log.operation, res.operation);
        assert_eq!(transparency_log.node_id, res.node_id);
        assert_eq!(transparency_log.node_public_key, res.node_public_key);
    }

    #[tokio::test]
    async fn test_open_db() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

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

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let transparency_log = new_artifact_transparency_log_default();

        let result = log.write_transparency_log(&transparency_log);
        assert!(result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_write_twice_transparency_log_error() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let transparency_log = new_artifact_transparency_log_default();

        let mut result = log.write_transparency_log(&transparency_log);
        assert!(result.is_ok());
        result = log.write_transparency_log(&transparency_log);
        assert!(result.is_err());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_find_transparency_log() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let id = "test_id";
        let transparency_log = new_artifact_transparency_log_with_id(id);

        let result_write = log.write_transparency_log(&transparency_log);
        assert!(result_write.is_ok());

        let result_find = log.find_transparency_log(id);
        assert!(result_find.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_find_transparency_log_not_found() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let transparency_log = new_artifact_transparency_log_default();

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

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let ps_art_id = "package_specific_artifact_id";
        let transparency_log = new_artifact_transparency_log(
            Some(PackageType::Maven2),
            Operation::AddArtifact,
            Some("package_specific_id"),
            Some(ps_art_id),
        );

        assert!(log.write_transparency_log(&transparency_log).is_ok());

        assert!(log
            .read_transparency_log(&PackageType::Maven2, ps_art_id)
            .is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_read_transparency_log_invalid_id() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let transparency_log = new_artifact_transparency_log(
            Some(PackageType::Maven2),
            Operation::AddArtifact,
            Some("package_specific_id"),
            Some("package_specific_artifact_id"),
        );

        assert!(log.write_transparency_log(&transparency_log).is_ok());

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

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let transparency_log1 = new_artifact_transparency_log(
            Some(PackageType::Maven2),
            Operation::AddArtifact,
            Some("package_specific_id"),
            Some("package_specific_artifact_id"),
        );

        let result_write1 = log.write_transparency_log(&transparency_log1);
        assert!(result_write1.is_ok());

        let ps_art_id = "package_specific_artifact_id2";
        let transparency_log2 = new_artifact_transparency_log(
            Some(PackageType::Maven2),
            Operation::AddArtifact,
            Some("package_specific_id2"),
            Some(ps_art_id),
        );

        assert!(log.write_transparency_log(&transparency_log2).is_ok());

        assert!(log
            .read_transparency_log(&PackageType::Maven2, ps_art_id)
            .is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_read_transparency_logs() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let ps_id = "package_specific_id";
        let transparency_log1 = new_artifact_transparency_log(
            Some(PackageType::Maven2),
            Operation::AddArtifact,
            Some(ps_id),
            Some("package_specific_artifact_id"),
        );

        let result_write1 = log.write_transparency_log(&transparency_log1);
        assert!(result_write1.is_ok());

        let transparency_log2 = new_artifact_transparency_log(
            Some(PackageType::Maven2),
            Operation::AddArtifact,
            Some(ps_id),
            Some("package_specific_artifact_id2"),
        );

        assert!(log.write_transparency_log(&transparency_log2).is_ok());

        let result_read1 = log.read_transparency_logs(&PackageType::Maven2, ps_id);
        assert!(result_read1.is_ok());
        assert_eq!(result_read1.unwrap().len(), 2);

        let result_read2 =
            log.read_transparency_logs(&PackageType::Maven2, "other_package_specific_id");
        assert!(result_read2.is_ok());
        assert_eq!(result_read2.unwrap().len(), 0);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_verify_artifact_can_be_added_to_transparency_logs() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);
        let result1 = log.verify_package_can_be_added_to_transparency_logs(
            &PackageType::Docker,
            "package_specific_id",
        );
        assert!(result1.is_ok());

        let ps_id = "package_specific_id";
        assert!(log
            .verify_package_can_be_added_to_transparency_logs(&PackageType::Docker, ps_id)
            .is_ok());

        let transparency_log1 = new_artifact_transparency_log(
            Some(PackageType::Docker),
            Operation::AddArtifact,
            Some(ps_id),
            Some("package_specific_artifact_id"),
        );
        assert!(log.write_transparency_log(&transparency_log1).is_ok());

        let result2 =
            log.verify_package_can_be_added_to_transparency_logs(&PackageType::Docker, ps_id);
        assert!(result2.is_err());
        assert_eq!(
            result2.err().unwrap().to_string(),
            TransparencyLogError::ArtifactAlreadyExists {
                package_type: PackageType::Docker,
                package_specific_id: ps_id.to_string(),
            }
            .to_string()
        );

        let transparency_log2 = new_artifact_transparency_log(
            Some(PackageType::Docker),
            Operation::RemoveArtifact,
            Some(ps_id),
            Some("package_specific_artifact_id"),
        );
        assert!(log.write_transparency_log(&transparency_log2).is_ok());
        assert!(log
            .verify_package_can_be_added_to_transparency_logs(&PackageType::Docker, ps_id)
            .is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_read_remove_artifact_transparency_log() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let ps_art_id = "package_specific_artifact_id";
        let transparency_log = new_artifact_transparency_log(
            Some(PackageType::Maven2),
            Operation::RemoveArtifact,
            Some("package_specific_id"),
            Some(ps_art_id),
        );

        assert!(log.write_transparency_log(&transparency_log).is_ok());

        let result_read = log.read_transparency_log(&PackageType::Maven2, ps_art_id);
        assert!(result_read.is_err());
        assert_eq!(
            result_read.err().unwrap().to_string(),
            TransparencyLogError::InvalidOperation {
                id: transparency_log.id,
                invalid_operation: Operation::RemoveArtifact,
            }
            .to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_add_artifact() {
        let tmp_dir = test_util::tests::setup();

        let (mut log, mut build_event_receiver) =
            test_util::tests::create_transparency_log_service(&tmp_dir);

        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BlockchainEvent::AddBlock { sender, .. }) => {
                        let _ = sender.send(Ok(()));
                    }
                    _ => panic!("BlockchainEvent must match BlockchainEvent::AddBlock"),
                }
            }
        });

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

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let result_read = log.get_authorized_nodes();
        assert!(result_read.is_ok());
        assert_eq!(result_read.unwrap().len(), 0);

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_add_authorized_nodes() {
        let tmp_dir = test_util::tests::setup();

        let (log, mut build_event_receiver) =
            test_util::tests::create_transparency_log_service(&tmp_dir);

        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BlockchainEvent::AddBlock { sender, .. }) => {
                        let _ = sender.send(Ok(()));
                    }
                    _ => panic!("BlockchainEvent must match BlockchainEvent::AddBlock"),
                }
            }
        });

        let peer_id = Keypair::generate_ed25519().public().to_peer_id();

        let transparency_log =
            new_auth_node_transparency_log(Operation::AddNode, &peer_id.to_string());

        let result_add = log.add_authorized_node(peer_id).await;
        assert!(result_add.is_ok());

        let result_read = log.get_authorized_nodes();
        assert!(result_read.is_ok());
        let vec = result_read.unwrap();
        assert_eq!(vec.len(), 1);
        assert!(vec
            .get(0)
            .unwrap()
            .eq(&PeerId::from_str(&transparency_log.node_id).unwrap()));
        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_get_authorized_nodes_add() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let node_id: &str = &PeerId::random().to_string();
        let transparency_log = new_auth_node_transparency_log(Operation::AddNode, node_id);

        assert!(log.write_transparency_log(&transparency_log).is_ok());

        let result_read = log.get_authorized_nodes();
        assert!(result_read.is_ok());
        let vec = result_read.unwrap();
        assert_eq!(vec.len(), 1);
        assert!(vec.get(0).unwrap().eq(&PeerId::from_str(node_id).unwrap()));
        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_get_authorized_nodes_add_and_remove() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let first_node_id: &str = &PeerId::random().to_string();
        let transparency_log1 = new_auth_node_transparency_log(Operation::AddNode, first_node_id);

        assert!(log.write_transparency_log(&transparency_log1).is_ok());
        let second_node_id: &str = &PeerId::random().to_string();
        let transparency_log2 = new_auth_node_transparency_log(Operation::AddNode, second_node_id);

        assert!(log.write_transparency_log(&transparency_log2).is_ok());

        let transparency_log3 =
            new_auth_node_transparency_log(Operation::RemoveNode, first_node_id);

        assert!(log.write_transparency_log(&transparency_log3).is_ok());

        let result_read = log.get_authorized_nodes();
        assert!(result_read.is_ok());
        let vec = result_read.unwrap();
        assert_eq!(vec.len(), 1);
        assert!(vec
            .get(0)
            .unwrap()
            .eq(&PeerId::from_str(second_node_id).unwrap()));

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_verify_authorized_node_can_be_added() {
        let tmp_dir = test_util::tests::setup();

        let (log, _) = test_util::tests::create_transparency_log_service(&tmp_dir);

        let node_id: &str = &PeerId::random().to_string();
        assert!(log
            .verify_node_can_be_added_to_transparency_logs(node_id)
            .is_ok());

        let transparency_log1 = new_auth_node_transparency_log(Operation::AddNode, node_id);
        assert!(log.write_transparency_log(&transparency_log1).is_ok());

        let result2 = log.verify_node_can_be_added_to_transparency_logs(node_id);
        assert!(result2.is_err());
        assert_eq!(
            result2.err().unwrap().to_string(),
            TransparencyLogError::NodeAlreadyExists {
                node_id: node_id.to_string(),
            }
            .to_string()
        );

        let transparency_log3 = new_auth_node_transparency_log(Operation::RemoveNode, node_id);

        assert!(log.write_transparency_log(&transparency_log3).is_ok());

        assert!(log
            .verify_node_can_be_added_to_transparency_logs(node_id)
            .is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    fn new_artifact_transparency_log_default() -> TransparencyLog {
        new_transparency_log(
            Uuid::new_v4().to_string(),
            Some(PackageType::Maven2),
            Operation::AddArtifact,
            None,
            None,
        )
    }

    fn new_artifact_transparency_log_with_id(id: &str) -> TransparencyLog {
        new_transparency_log(
            id.to_owned(),
            Some(PackageType::Maven2),
            Operation::AddArtifact,
            None,
            None,
        )
    }

    fn new_artifact_transparency_log(
        pack_type: Option<PackageType>,
        op: Operation,
        ps_id: Option<&str>,
        ps_artifact_id: Option<&str>,
    ) -> TransparencyLog {
        new_transparency_log(
            Uuid::new_v4().to_string(),
            pack_type,
            op,
            ps_id,
            ps_artifact_id,
        )
    }

    fn new_transparency_log(
        id: String,
        pack_type: Option<PackageType>,
        op: Operation,
        ps_id: Option<&str>,
        ps_artifact_id: Option<&str>,
    ) -> TransparencyLog {
        TransparencyLog {
            id,
            package_type: pack_type,
            package_specific_id: ps_id.unwrap_or("ps_id").to_owned(),
            num_artifacts: 8,
            package_specific_artifact_id: ps_artifact_id.unwrap_or("ps_artifact_id").to_owned(),
            artifact_hash: "artifact_hash".to_owned(),
            source_hash: "source_hash".to_owned(),
            artifact_id: Uuid::new_v4().to_string(),
            source_id: Uuid::new_v4().to_string(),
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            operation: op,
            node_id: Uuid::new_v4().to_string(),
            node_public_key: Uuid::new_v4().to_string(),
        }
    }

    fn new_auth_node_transparency_log(op: Operation, node_id: &str) -> TransparencyLog {
        TransparencyLog {
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
            operation: op,
            node_id: node_id.to_owned(),
            node_public_key: Uuid::new_v4().to_string(),
        }
    }
}
