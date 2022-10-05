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
use crate::build_service::error::BuildError;
use crate::build_service::event::BuildEventClient;
use crate::build_service::model::BuildResult;
use crate::transparency_log::log::{Operation, TransparencyLog};
use log::{error, info};
use std::collections::HashMap;
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Clone, Debug, Error, Eq, PartialEq)]
pub enum VerificationError {
    #[error("Artifact with specific id {artifact_specific_id} was not found in build {build_id}.")]
    ArtifactNotFound {
        build_id: String,
        artifact_specific_id: String,
    },
    #[error("Artifact with specific id {artifact_specific_id} from build {build_id} has invalid hash. Expected {expected_hash} but got {hash_from_build}.")]
    NonMatchingHash {
        build_id: String,
        artifact_specific_id: String,
        expected_hash: String,
        hash_from_build: String,
    },
    #[error("Verification failed with error: {0}")]
    Failure(String),
    #[error("Verification service does not support transparency logs with operation {0}")]
    UnsupportedOperation(Operation),
    #[error("Failed to verify build: {0}")]
    VerifyBuildError(BuildError),
}

impl From<BuildError> for VerificationError {
    fn from(build_error: BuildError) -> Self {
        Self::VerifyBuildError(build_error)
    }
}

/// A utility struct that uniquely identifies a package
/// by combining the type and specific id.
#[derive(Debug, Eq, Hash, PartialEq)]
struct Package {
    package_type: PackageType,
    package_specific_id: String,
}

/// Holds information about a single artifact that is
/// pending to be built or is actively being built.
struct VerificationInfo {
    sender: oneshot::Sender<Result<(), VerificationError>>,
    artifact_specific_id: String,
    artifact_hash: String,
}

/// The verification service is a component used by authorized nodes only.
/// It implements all necessary logic to verify blockchain transactions.
pub struct VerificationService {
    build_event_client: BuildEventClient,
    /// A map that keeps track of pending builds of packages. For each
    /// package in the map, each entry in the vector maps with a single
    /// artifact of the associated package. When all verification info
    /// for a single package has been collected, a build will be started
    /// and the verification info will move to the verifying_info map.
    pending_info: HashMap<Package, Vec<VerificationInfo>>,
    /// A map that keeps track of active builds. Each active build contains
    /// verification info that will be used for verification after the
    /// associated build has completed.
    verifying_info: HashMap<String, Vec<VerificationInfo>>,
}

impl VerificationService {
    pub fn new(build_event_client: BuildEventClient) -> Result<Self, anyhow::Error> {
        Ok(VerificationService {
            build_event_client,
            pending_info: Default::default(),
            verifying_info: Default::default(),
        })
    }

    /// Verify a build for the specified transaction. This method is
    /// used to be able to reach consensus about a transaction that
    /// is a candidate to be committed to the blockchain.
    pub async fn verify_transaction(
        &mut self,
        transaction_payload: &[u8],
        sender: oneshot::Sender<Result<(), VerificationError>>,
    ) -> Result<Option<String>, VerificationError> {
        let transparency_log: TransparencyLog = serde_json::from_slice(transaction_payload)
            .map_err(|e| VerificationError::Failure(e.to_string()))?;

        match transparency_log.operation {
            Operation::AddArtifact => self.verify_add_artifact(transparency_log, sender).await,
            unsupported_operation => Err(VerificationError::UnsupportedOperation(
                unsupported_operation,
            )),
        }
    }

    async fn verify_add_artifact(
        &mut self,
        transparency_log: TransparencyLog,
        sender: oneshot::Sender<Result<(), VerificationError>>,
    ) -> Result<Option<String>, VerificationError> {
        let package = Package {
            package_type: transparency_log
                .package_type
                .expect("Package type should not be empty"),
            package_specific_id: transparency_log.package_specific_id.clone(),
        };
        let num_artifacts = match self.pending_info.get_mut(&package) {
            Some(verification_artifacts) => {
                verification_artifacts.push(VerificationInfo {
                    sender,
                    artifact_specific_id: transparency_log.package_specific_artifact_id.clone(),
                    artifact_hash: transparency_log.artifact_hash.clone(),
                });
                verification_artifacts.len() as u32
            }
            None => {
                let verification_artifacts = vec![VerificationInfo {
                    sender,
                    artifact_specific_id: transparency_log.package_specific_artifact_id.clone(),
                    artifact_hash: transparency_log.artifact_hash.clone(),
                }];
                self.pending_info.insert(package, verification_artifacts);
                1
            }
        };

        if num_artifacts == transparency_log.num_artifacts {
            let package = Package {
                package_type: transparency_log
                    .package_type
                    .expect("Package type should not be empty"),
                package_specific_id: transparency_log.package_specific_id.clone(),
            };
            if let Some(verification_artifacts) = self.pending_info.remove(&package) {
                let build_id = self
                    .build_event_client
                    .verify_build(
                        transparency_log
                            .package_type
                            .expect("Package type should not be empty"),
                        transparency_log.package_specific_id.clone(),
                        transparency_log.package_specific_artifact_id.clone(),
                        transparency_log.artifact_hash.clone(),
                    )
                    .await?;

                self.verifying_info
                    .insert(build_id.clone(), verification_artifacts);

                Ok(Some(build_id))
            } else {
                Err(VerificationError::Failure(format!(
                    "Could not find verification info for package {:?}",
                    package
                )))
            }
        } else {
            Ok(None)
        }
    }

    pub fn handle_build_failed(&mut self, build_id: &str, build_error: BuildError) {
        if let Some(verification_artifacts) = self.verifying_info.remove(build_id) {
            let verification_error = VerificationError::from(build_error);
            for verification_artifact in verification_artifacts {
                verification_artifact
                    .sender
                    .send(Err(verification_error.clone()))
                    .unwrap_or_else(|e| {
                        error!("Verification Artifact verification_error send. Verification error {:#?}", e);
                    });
            }
        }
    }

    pub async fn handle_build_result(
        &mut self,
        build_id: &str,
        build_result: BuildResult,
    ) -> Result<(), anyhow::Error> {
        let package_specific_id = build_result.package_specific_id.as_str();

        info!(
            "Build with ID {} completed successfully for package type {:?} and package specific ID {}",
            build_id, build_result.package_type, package_specific_id
        );

        if let Some(verification_artifacts) = self.verifying_info.remove(build_id) {
            for verification_artifact in verification_artifacts {
                match build_result.artifacts.iter().find(|artifact| {
                    artifact.artifact_specific_id == verification_artifact.artifact_specific_id
                }) {
                    Some(build_result_artifact) => {
                        if verification_artifact.artifact_hash
                            == build_result_artifact.artifact_hash
                        {
                            verification_artifact
                                .sender
                                .send(Ok(()))
                                .unwrap_or_else(|_e| {
                                    error!("Verification Artifact Hash match send Ok.");
                                });
                        } else {
                            verification_artifact
                                .sender
                                .send(Err(VerificationError::NonMatchingHash {
                                    build_id: build_id.to_owned(),
                                    artifact_specific_id: verification_artifact
                                        .artifact_specific_id,
                                    expected_hash: verification_artifact.artifact_hash,
                                    hash_from_build: build_result_artifact.artifact_hash.clone(),
                                }))
                                .unwrap_or_else(|e| {
                                    error!("Verification Artifact Hash not matched send VerificationError {:#?}", e);
                                });
                        }
                    }
                    None => {
                        verification_artifact
                            .sender
                            .send(Err(VerificationError::ArtifactNotFound {
                                build_id: build_id.to_owned(),
                                artifact_specific_id: verification_artifact.artifact_specific_id,
                            }))
                            .unwrap_or_else(|e| {
                                error!("build_result_artifact:None. VerificationError {:#?}", e);
                            });
                    }
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::artifact_service::model::PackageType;
    use crate::blockchain_service::service::BlockchainService;
    use crate::build_service::event::BuildEvent;
    use crate::build_service::model::BuildResultArtifact;
    use crate::network::client::Client;
    use crate::transparency_log::log::{AddArtifactRequest, TransparencyLogService};
    use crate::util::test_util;
    use libp2p::identity::Keypair;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use tokio::sync::{mpsc, Mutex};

    fn create_p2p_client(local_keypair: &Keypair) -> Client {
        let (command_sender, _command_receiver) = mpsc::channel(1);
        Client {
            sender: command_sender,
            local_peer_id: local_keypair.public().to_peer_id(),
        }
    }

    async fn create_blockchain_service(
        local_keypair: &Keypair,
        p2p_client: Client,
        blockchain_path: impl AsRef<Path>,
    ) -> BlockchainService {
        let ed25519_keypair = match local_keypair {
            libp2p::identity::Keypair::Ed25519(ref v) => v,
            _ => {
                panic!("Keypair Format Error");
            }
        };

        BlockchainService::init_first_blockchain_node(
            ed25519_keypair,
            ed25519_keypair,
            p2p_client,
            blockchain_path,
        )
        .await
        .expect("Creating BlockchainService failed")
    }

    async fn create_transparency_log_service(
        artifact_path: impl AsRef<Path>,
    ) -> TransparencyLogService {
        let local_keypair = Keypair::generate_ed25519();
        let p2p_client = create_p2p_client(&local_keypair);

        let blockchain_service =
            create_blockchain_service(&local_keypair, p2p_client, &artifact_path).await;

        TransparencyLogService::new(&artifact_path, Arc::new(Mutex::new(blockchain_service)))
            .unwrap()
    }

    #[tokio::test]
    async fn test_verify_add_artifact_transaction() {
        let tmp_dir = test_util::tests::setup();

        let mut transparency_log_service = create_transparency_log_service(&tmp_dir).await;

        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.1";
        let transparency_log = transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 1,
                package_specific_artifact_id: "".to_owned(),
                artifact_hash: uuid::Uuid::new_v4().to_string(),
            })
            .await
            .unwrap();
        let payload = serde_json::to_string(&transparency_log).unwrap();

        let (verification_result_sender, _verification_result_receiver) = oneshot::channel();

        let (build_event_sender, mut build_event_receiver) = mpsc::channel(1);

        let build_event_client = BuildEventClient::new(build_event_sender);

        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Verify {
                        package_type: sent_package_type,
                        package_specific_id: sent_package_specific_id,
                        sender,
                    }) => {
                        let build_id = uuid::Uuid::new_v4().to_string();
                        assert_eq!(sent_package_type, package_type);
                        assert_eq!(sent_package_specific_id, package_specific_id);
                        let _ = sender.send(Ok(build_id));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Verify"),
                }
            }
        });

        let mut verification_service = VerificationService::new(build_event_client).unwrap();
        let verification_result = verification_service
            .verify_transaction(payload.as_bytes(), verification_result_sender)
            .await;

        assert!(verification_result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_verify_add_artifact_transaction_starts_build_when_num_artifacts_reached() {
        let tmp_dir = test_util::tests::setup();

        let mut transparency_log_service = create_transparency_log_service(&tmp_dir).await;

        let mut payloads = vec![];
        for i in 1..=3 {
            let transparency_log = transparency_log_service
                .add_artifact(AddArtifactRequest {
                    package_type: PackageType::Docker,
                    package_specific_id: "alpine:3.15.1".to_owned(),
                    num_artifacts: 3,
                    package_specific_artifact_id: format!("psaid_{}", i),
                    artifact_hash: uuid::Uuid::new_v4().to_string(),
                })
                .await
                .unwrap();
            let payload = serde_json::to_string(&transparency_log).unwrap();
            payloads.push(payload);
        }

        let (build_event_sender, mut build_event_receiver) = mpsc::channel(1);

        let build_event_client = BuildEventClient::new(build_event_sender);

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Verify { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Verify"),
                }
            }
        });

        let mut verification_service = VerificationService::new(build_event_client).unwrap();

        let (verification_result_sender_1, _verification_result_receiver) = oneshot::channel();
        let verification_result_1 = verification_service
            .verify_transaction(payloads[0].as_bytes(), verification_result_sender_1)
            .await;
        assert!(verification_result_1.is_ok());
        assert!(verification_result_1.unwrap().is_none());

        let (verification_result_sender_2, _verification_result_receiver) = oneshot::channel();
        let verification_result_2 = verification_service
            .verify_transaction(payloads[1].as_bytes(), verification_result_sender_2)
            .await;
        assert!(verification_result_2.is_ok());
        assert!(verification_result_2.unwrap().is_none());

        let (verification_result_sender_3, _verification_result_receiver) = oneshot::channel();
        let verification_result_3 = verification_service
            .verify_transaction(payloads[2].as_bytes(), verification_result_sender_3)
            .await;
        assert!(verification_result_3.is_ok());
        assert_eq!(
            verification_result_3.unwrap().unwrap(),
            build_id.to_string()
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_handle_build_result_notifies_sender() {
        let tmp_dir = test_util::tests::setup();

        let mut transparency_log_service = create_transparency_log_service(&tmp_dir).await;

        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.1";
        let package_specific_artifact_id = "a/b/c.blob";
        let artifact_hash = uuid::Uuid::new_v4();
        let transparency_log = transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 1,
                package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                artifact_hash: artifact_hash.to_string(),
            })
            .await
            .unwrap();
        let payload = serde_json::to_string(&transparency_log).unwrap();

        let (verification_result_sender, verification_result_receiver) = oneshot::channel();

        let (build_event_sender, mut build_event_receiver) = mpsc::channel(1);

        let build_event_client = BuildEventClient::new(build_event_sender);

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Verify { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Verify"),
                }
            }
        });

        let mut verification_service = VerificationService::new(build_event_client).unwrap();
        let verification_result = verification_service
            .verify_transaction(payload.as_bytes(), verification_result_sender)
            .await;

        assert!(verification_result.is_ok());

        let build_result = BuildResult {
            package_type,
            package_specific_id: package_specific_id.to_owned(),
            artifacts: vec![BuildResultArtifact {
                artifact_specific_id: package_specific_artifact_id.to_owned(),
                artifact_hash: artifact_hash.to_string(),
                artifact_location: PathBuf::from("a/b/c.blob"),
            }],
        };
        let handle_build_result = verification_service
            .handle_build_result(build_id.to_string().as_str(), build_result)
            .await;
        assert!(handle_build_result.is_ok());

        let verification_result = verification_result_receiver.await.unwrap();
        assert!(verification_result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_handle_build_result_with_missing_artifact_notifies_sender() {
        let tmp_dir = test_util::tests::setup();

        let mut transparency_log_service = create_transparency_log_service(&tmp_dir).await;

        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.1";
        let package_specific_artifact_id = "a/b/c.blob";
        let artifact_hash = uuid::Uuid::new_v4();
        let transparency_log = transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 1,
                package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                artifact_hash: artifact_hash.to_string(),
            })
            .await
            .unwrap();
        let payload = serde_json::to_string(&transparency_log).unwrap();

        let (verification_result_sender, verification_result_receiver) = oneshot::channel();

        let (build_event_sender, mut build_event_receiver) = mpsc::channel(1);

        let build_event_client = BuildEventClient::new(build_event_sender);

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Verify { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Verify"),
                }
            }
        });

        let mut verification_service = VerificationService::new(build_event_client).unwrap();
        let verification_result = verification_service
            .verify_transaction(payload.as_bytes(), verification_result_sender)
            .await;

        assert!(verification_result.is_ok());

        let missing_package_specific_artifact_id = "d/e/f.blob";
        let build_result = BuildResult {
            package_type,
            package_specific_id: package_specific_id.to_owned(),
            artifacts: vec![BuildResultArtifact {
                artifact_specific_id: missing_package_specific_artifact_id.to_owned(),
                artifact_hash: artifact_hash.to_string(),
                artifact_location: PathBuf::from("a/b/c.blob"),
            }],
        };
        let handle_build_result = verification_service
            .handle_build_result(build_id.to_string().as_str(), build_result)
            .await;
        assert!(handle_build_result.is_ok());

        let verification_result = verification_result_receiver.await.unwrap();
        assert!(verification_result.is_err());
        assert_eq!(
            verification_result.unwrap_err(),
            VerificationError::ArtifactNotFound {
                build_id: build_id.to_string(),
                artifact_specific_id: package_specific_artifact_id.to_owned()
            }
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_handle_build_result_with_different_hash_notifies_sender() {
        let tmp_dir = test_util::tests::setup();

        let mut transparency_log_service = create_transparency_log_service(&tmp_dir).await;

        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.1";
        let package_specific_artifact_id = "a/b/c.blob";
        let artifact_hash = uuid::Uuid::new_v4();
        let transparency_log = transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 1,
                package_specific_artifact_id: package_specific_artifact_id.to_owned(),
                artifact_hash: artifact_hash.to_string(),
            })
            .await
            .unwrap();
        let payload = serde_json::to_string(&transparency_log).unwrap();

        let (verification_result_sender, verification_result_receiver) = oneshot::channel();

        let (build_event_sender, mut build_event_receiver) = mpsc::channel(1);

        let build_event_client = BuildEventClient::new(build_event_sender);

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Verify { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Verify"),
                }
            }
        });

        let mut verification_service = VerificationService::new(build_event_client).unwrap();
        let verification_result = verification_service
            .verify_transaction(payload.as_bytes(), verification_result_sender)
            .await;

        assert!(verification_result.is_ok());

        let different_artifact_hash = uuid::Uuid::new_v4();
        let build_result = BuildResult {
            package_type,
            package_specific_id: package_specific_id.to_owned(),
            artifacts: vec![BuildResultArtifact {
                artifact_specific_id: package_specific_artifact_id.to_owned(),
                artifact_hash: different_artifact_hash.to_string(),
                artifact_location: PathBuf::from("a/b/c.blob"),
            }],
        };
        let handle_build_result = verification_service
            .handle_build_result(build_id.to_string().as_str(), build_result)
            .await;
        assert!(handle_build_result.is_ok());

        let verification_result = verification_result_receiver.await.unwrap();
        assert!(verification_result.is_err());
        assert_eq!(
            verification_result.unwrap_err(),
            VerificationError::NonMatchingHash {
                build_id: build_id.to_string(),
                artifact_specific_id: package_specific_artifact_id.to_owned(),
                expected_hash: artifact_hash.to_string(),
                hash_from_build: different_artifact_hash.to_string()
            }
        );

        test_util::tests::teardown(tmp_dir);
    }

    #[tokio::test]
    async fn test_handle_failed_build_notifies_sender() {
        let tmp_dir = test_util::tests::setup();

        let mut transparency_log_service = create_transparency_log_service(&tmp_dir).await;

        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.1";
        let transparency_log = transparency_log_service
            .add_artifact(AddArtifactRequest {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                num_artifacts: 1,
                package_specific_artifact_id: "".to_owned(),
                artifact_hash: uuid::Uuid::new_v4().to_string(),
            })
            .await
            .unwrap();
        let payload = serde_json::to_string(&transparency_log).unwrap();

        let (verification_result_sender, verification_result_receiver) = oneshot::channel();

        let (build_event_sender, mut build_event_receiver) = mpsc::channel(1);

        let build_event_client = BuildEventClient::new(build_event_sender);

        let build_id = uuid::Uuid::new_v4();
        tokio::spawn(async move {
            loop {
                match build_event_receiver.recv().await {
                    Some(BuildEvent::Verify { sender, .. }) => {
                        let _ = sender.send(Ok(build_id.to_string()));
                    }
                    _ => panic!("BuildEvent must match BuildEvent::Verify"),
                }
            }
        });

        let mut verification_service = VerificationService::new(build_event_client).unwrap();
        let verify_transaction_result = verification_service
            .verify_transaction(payload.as_bytes(), verification_result_sender)
            .await;
        assert!(verify_transaction_result.is_ok());

        let build_error = BuildError::InitializationFailed("error".to_owned());
        verification_service
            .handle_build_failed(build_id.to_string().as_str(), build_error.clone());

        let verification_result = verification_result_receiver.await.unwrap();
        assert!(verification_result.is_err());
        assert_eq!(
            verification_result.unwrap_err(),
            VerificationError::from(build_error)
        );

        test_util::tests::teardown(tmp_dir);
    }
}
