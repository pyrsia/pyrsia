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

use thiserror::Error;
use tokio::sync::{mpsc, oneshot};

use pyrsia_blockchain_network::structures::transaction::Transaction;

use crate::build_service::event::BuildEvent;

#[derive(Debug, Error)]
pub enum VerificationError {}

pub struct VerificationResult {}

/// The verification service is a component used by authorized nodes only.
/// It implements all necessary logic to verify blockchain transactions.
pub struct VerificationService {
    _build_event_sender: mpsc::Sender<BuildEvent>,
}

impl VerificationService {
    pub fn new(build_event_sender: mpsc::Sender<BuildEvent>) -> Result<Self, anyhow::Error> {
        Ok(VerificationService {
            _build_event_sender: build_event_sender,
        })
    }

    /// Verify a build for the specified transaction. This method is
    /// used to be able to reach consensus about a transaction that
    /// is a candidate to be committed to the blockchain.
    pub async fn verify_transaction(
        &self,
        _transaction: Transaction,
        _sender: oneshot::Sender<Result<VerificationResult, VerificationError>>,
    ) -> Result<(), VerificationError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity;
    use pyrsia_blockchain_network::structures::header::Address;
    use pyrsia_blockchain_network::structures::transaction::TransactionType;

    #[tokio::test]
    async fn test_verify_transaction() {
        let keypair = identity::ed25519::Keypair::generate();
        let submitter = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let payload = vec![1, 2, 3];
        let transaction = Transaction::new(TransactionType::Create, submitter, payload, &keypair);

        let (sender, _) = oneshot::channel();

        let (build_command_sender, _build_command_receiver) = mpsc::channel(1);

        let verification_service = VerificationService::new(build_command_sender).unwrap();
        let verification_result = verification_service
            .verify_transaction(transaction, sender)
            .await;

        assert!(verification_result.is_ok());
    }
}
