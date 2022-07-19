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

use crate::build_service::error::BuildError;
use crate::build_service::event::BuildEventClient;
use crate::transparency_log::log::{Operation, TransparencyLog};
use pyrsia_blockchain_network::structures::transaction::Transaction;
use thiserror::Error;
use tokio::sync::oneshot;

#[derive(Debug, Error)]
pub enum VerificationError {
    #[error("Verification failed with error: {0}")]
    Failure(String),
    #[error("Failed to verify build: {0}")]
    VerifyBuildError(BuildError),
}

impl From<BuildError> for VerificationError {
    fn from(build_error: BuildError) -> Self {
        Self::VerifyBuildError(build_error)
    }
}

pub struct VerificationResult {}

/// The verification service is a component used by authorized nodes only.
/// It implements all necessary logic to verify blockchain transactions.
pub struct VerificationService {
    build_event_client: BuildEventClient,
}

impl VerificationService {
    pub fn new(build_event_client: BuildEventClient) -> Result<Self, anyhow::Error> {
        Ok(VerificationService { build_event_client })
    }

    /// Verify a build for the specified transaction. This method is
    /// used to be able to reach consensus about a transaction that
    /// is a candidate to be committed to the blockchain.
    pub async fn verify_transaction(
        &self,
        transaction: Transaction,
        _sender: oneshot::Sender<Result<VerificationResult, VerificationError>>,
    ) -> Result<(), VerificationError> {
        let transparency_log: TransparencyLog = serde_json::from_slice(&transaction.payload())
            .map_err(|e| VerificationError::Failure(e.to_string()))?;

        if transparency_log.operation == Operation::AddArtifact {
            self.build_event_client
                .verify_build(
                    transparency_log.package_type,
                    transparency_log.package_specific_id,
                    transparency_log.package_specific_artifact_id,
                    transparency_log.artifact_hash,
                )
                .await?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_service::model::PackageType;
    use crate::build_service::mapping::model::MappingInfo;
    use crate::build_service::model::{BuildInfo, BuildStatus};
    use crate::transparency_log::log::{AddArtifactRequest, TransparencyLogService};
    use crate::util::test_util;
    use httptest::{matchers, responders, Expectation, Server};
    use libp2p::identity;
    use pyrsia_blockchain_network::structures::header::Address;
    use pyrsia_blockchain_network::structures::transaction::TransactionType;
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_verify_add_artifact_transaction() {
        let tmp_dir = test_util::tests::setup();

        let (sender, receiver) = oneshot::channel();

        let mut transparency_log_service = TransparencyLogService::new(tmp_dir).unwrap();

        transparency_log_service
            .add_artifact(
                AddArtifactRequest {
                    package_type: PackageType::Docker,
                    package_specific_id: "alpine:3.15.1".to_owned(),
                    package_specific_artifact_id: "".to_owned(),
                    artifact_hash: uuid::Uuid::new_v4().to_string(),
                },
                sender,
            )
            .await
            .unwrap();

        let transparency_log = receiver.await.unwrap().unwrap();
        let payload = serde_json::to_string(&transparency_log).unwrap();

        let keypair = identity::ed25519::Keypair::generate();
        let submitter = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let transaction = Transaction::new(
            TransactionType::Create,
            submitter,
            payload.as_bytes().to_vec(),
            &keypair,
        );

        let (sender, _) = oneshot::channel();

        let (build_event_sender, _build_event_receiver) = mpsc::channel(1);

        let build_event_client = BuildEventClient::new(build_event_sender);

        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id: "alpine:3.15.2".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let build_info = BuildInfo {
            id: uuid::Uuid::new_v4().to_string(),
            status: BuildStatus::Running,
        };

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::all_of!(
                matchers::request::method_path("PUT", "/build"),
                matchers::request::body(matchers::json_decoded(matchers::eq(serde_json::json!(
                    &mapping_info
                ))))
            ))
            .respond_with(responders::json_encoded(&build_info)),
        );

        let verification_service = VerificationService::new(build_event_client).unwrap();
        let verification_result = verification_service
            .verify_transaction(transaction, sender)
            .await;

        assert!(verification_result.is_ok());
    }
}
