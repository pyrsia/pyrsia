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

use codec::{Decode, Encode};
use libp2p::identity;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::header::Address;
use crate::crypto::hash_algorithm::HashDigest;
use crate::signature::Signature;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Copy, Decode, Encode)]
pub enum TransactionType {
    Create,
}

// Temporary structure to be able to calculate the hash of a transaction
#[derive(Serialize)]
struct PartialTransaction {
    type_id: TransactionType,
    submitter: Address,
    timestamp: u64,
    payload: Vec<u8>,
    nonce: u128,
}

impl PartialTransaction {
    fn convert_to_transaction(
        self,
        ed25519_keypair: &identity::ed25519::Keypair,
    ) -> Result<Transaction, bincode::Error> {
        let hash = calculate_hash(&self)?;
        Ok(Transaction {
            type_id: self.type_id,
            submitter: self.submitter,
            timestamp: self.timestamp,
            payload: self.payload,
            nonce: self.nonce,
            hash,
            signature: Signature::new(&bincode::serialize(&hash)?, ed25519_keypair),
        })
    }
}

impl From<Transaction> for PartialTransaction {
    fn from(transaction: Transaction) -> Self {
        PartialTransaction {
            type_id: transaction.type_id,
            submitter: transaction.submitter,
            timestamp: transaction.timestamp,
            payload: transaction.payload,
            nonce: transaction.nonce,
        }
    }
}

fn calculate_hash(
    incomplete_transaction: &PartialTransaction,
) -> Result<HashDigest, bincode::Error> {
    let bytes = bincode::serialize(incomplete_transaction)?;
    Ok(HashDigest::new(&bytes))
}

pub type TransactionSignature = Signature;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Decode, Encode)]
pub struct Transaction {
    type_id: TransactionType,
    submitter: Address,
    timestamp: u64,
    payload: Vec<u8>,
    nonce: u128, // Adds a salt to harden
    hash: HashDigest,
    signature: TransactionSignature,
}
impl Transaction {
    pub fn new(
        type_id: TransactionType,
        submitter: Address,
        payload: Vec<u8>,
        ed25519_keypair: &identity::ed25519::Keypair,
    ) -> Self {
        let partial_transaction = PartialTransaction {
            type_id,
            submitter,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            payload,
            nonce: rand::thread_rng().gen::<u128>(),
        };
        partial_transaction
            .convert_to_transaction(ed25519_keypair)
            .unwrap()
    }

    pub fn hash(&self) -> HashDigest {
        self.hash
    }

    pub fn signature(&self) -> TransactionSignature {
        self.signature.clone()
    }

    pub fn payload(&self) -> Vec<u8> {
        self.payload.clone()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_new() {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            b"Hello First Transaction".to_vec(),
            &keypair,
        );
        let partial: PartialTransaction = transaction.clone().into();
        let expected_hash = calculate_hash(&partial).unwrap();
        let expected_signature =
            Signature::new(&bincode::serialize(&expected_hash).unwrap(), &keypair);

        assert_eq!(expected_hash, transaction.hash());
        assert_eq!(expected_signature, transaction.signature());
    }

    #[test]
    fn test_payload() {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            b"Hello First Transaction".to_vec(),
            &keypair,
        );

        assert_eq!(b"Hello First Transaction".to_vec(), transaction.payload());
    }
}
