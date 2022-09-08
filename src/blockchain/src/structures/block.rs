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
use log::warn;
use serde::{Deserialize, Serialize};
use std::cmp::Ordering;
use std::fmt::{Display, Formatter};

use super::header::{Address, Header, Ordinal};
use super::transaction::Transaction;
use crate::crypto::hash_algorithm::HashDigest;
use crate::signature::Signature;

pub type PublicKey = [u8; 32];
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Decode, Encode, Hash)]
pub struct BlockSignature {
    signature: Signature,
    #[codec(skip)]
    public_key: PublicKey,
}

#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq, Decode, Encode, Hash)]
pub struct Block {
    pub header: Header,
    // TODO(fishseabowl): Should be a Merkle Tree to speed up validation with root hash
    pub transactions: Vec<Transaction>,
    block_signature: BlockSignature,
}

impl Block {
    pub fn new(
        parent_hash: HashDigest,
        ordinal: Ordinal,
        transactions: Vec<Transaction>,
        signing_key: &identity::ed25519::Keypair,
    ) -> Self {
        let transaction_root = HashDigest::new(&bincode::serialize(&transactions).unwrap());
        let header = Header::new(
            parent_hash,
            transaction_root,
            Address::from(identity::PublicKey::Ed25519(signing_key.public())),
            ordinal,
        );
        Self {
            header,
            transactions,
            block_signature: BlockSignature {
                signature: Signature::new(
                    &bincode::serialize(&header.hash()).unwrap(),
                    signing_key,
                ),
                public_key: signing_key.public().encode(),
            },
        }
    }

    pub fn signature(&self) -> BlockSignature {
        self.block_signature.clone()
    }

    pub fn verify(&self) -> bool {
        let public_key = identity::ed25519::PublicKey::decode(&self.signature().public_key);
        match public_key {
            Ok(pub_key) => pub_key.verify(
                &bincode::serialize(&self.header.hash()).unwrap(),
                &self.signature().signature.to_bytes(),
            ),
            Err(e) => {
                warn!("Blockchain: Couldn't decode public key! Error is {:?}", e);
                false
            }
        }
    }

    pub fn fetch_payload(&self) -> Vec<Vec<u8>> {
        let mut result = vec![];

        for trans in self.transactions.clone() {
            result.push(trans.payload());
        }

        result
    }
}

impl PartialOrd for Block {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        self.header.ordinal.partial_cmp(&other.header.ordinal)
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string_pretty(&self).expect("json format error");
        write!(f, "{}", json)
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {

    use super::super::transaction::TransactionType;
    use super::*;

    #[test]
    fn test_build_block() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let transactions = vec![Transaction::new(
            TransactionType::Create,
            local_id,
            b"Hello First Transaction".to_vec(),
            &keypair,
        )];
        let block = Block::new(HashDigest::new(b""), 1, transactions.to_vec(), &keypair);
        let expected_signature =
            Signature::new(&bincode::serialize(&block.header.hash()).unwrap(), &keypair);

        assert_eq!(1, block.header.ordinal);
        assert_eq!(expected_signature, block.signature().signature);
        Ok(())
    }

    #[test]
    fn test_fetch_payload() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let transactions = vec![Transaction::new(
            TransactionType::Create,
            local_id,
            b"Hello First Transaction".to_vec(),
            &keypair,
        )];
        let block = Block::new(HashDigest::new(b""), 1, transactions.to_vec(), &keypair);

        assert_eq!(
            b"Hello First Transaction".to_vec(),
            block.fetch_payload()[0]
        );
        Ok(())
    }

    #[test]
    fn test_signature() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let transactions = vec![Transaction::new(
            TransactionType::Create,
            local_id,
            b"Hello First Transaction".to_vec(),
            &keypair,
        )];
        let block = Block::new(HashDigest::new(b""), 1, transactions.to_vec(), &keypair);

        assert_eq!(block.signature(), block.block_signature);
        Ok(())
    }

    #[test]
    fn test_block_verify() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let transactions = vec![Transaction::new(
            TransactionType::Create,
            local_id,
            b"Hello First Transaction".to_vec(),
            &keypair,
        )];
        let block = Block::new(HashDigest::new(b""), 1, transactions.to_vec(), &keypair);

        assert!(block.verify());
        Ok(())
    }

    #[test]
    fn test_display() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let transactions = vec![Transaction::new(
            TransactionType::Create,
            local_id,
            b"Hello First Transaction".to_vec(),
            &keypair,
        )];
        let block = Block::new(HashDigest::new(b""), 1, transactions.to_vec(), &keypair);

        assert_ne!(format!("The block is: {block}"), "The block is: ");
        Ok(())
    }
}
