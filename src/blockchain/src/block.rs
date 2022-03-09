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

use libp2p::identity;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};
use std::time::{SystemTime, UNIX_EPOCH};

use super::crypto::hash_algorithm::HashDigest;
use super::header::*;
use super::signature::Signature;

// TransactionType define the type of transaction, currently only create
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub enum TransactionType {
    Create,
    AddAuthority,
    RevokeAuthority,
}

// ToDo
pub type TransactionSignature = Signature;
pub type BlockSignature = Signature;

//ToDo
pub fn sign(msg: &[u8], keypair: &identity::ed25519::Keypair) -> Vec<u8> {
    (*keypair).sign(msg)
}

//ToDo
pub fn get_publickey_from_keypair(
    keypair: &identity::ed25519::Keypair,
) -> identity::ed25519::PublicKey {
    keypair.public()
}

// struct Block define a block strcuture
#[derive(Serialize, Deserialize, Debug, Clone, Eq, PartialEq)]
pub struct Block {
    pub header: Header,
    pub transactions: Vec<Transaction>,
    pub signature: BlockSignature,
}

impl Block {
    pub fn new(
        header: Header,
        transactions: Vec<Transaction>,
        ed25519_keypair: &identity::ed25519::Keypair,
    ) -> Self {
        Self {
            header,
            transactions,
            signature: Signature::new(&bincode::serialize(&header.hash).unwrap(), ed25519_keypair),
        }
    }

    //After merging Aleph consensus algorithm, it would be implemented
    pub fn verify(&self) -> bool {
        true
    }
}

// struct Transaction define the details of a transaction in a block
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct Transaction {
    pub trans_type: TransactionType,
    pub submitter: Address,
    pub timestamp: u64,
    pub payload: Vec<u8>,
    nonce: u128, // Adds a salt to harden
    pub transaction_hash: HashDigest,
    pub signature: TransactionSignature,
}

impl Transaction {
    pub fn new(
        partial_transaction: PartialTransaction,
        ed25519_keypair: &identity::ed25519::Keypair,
    ) -> Self {
        let hash = HashDigest::new(&(bincode::serialize(&partial_transaction).unwrap()));
        Self {
            trans_type: partial_transaction.trans_type,
            submitter: partial_transaction.submitter,
            timestamp: partial_transaction.timestamp,
            payload: partial_transaction.payload,
            nonce: partial_transaction.nonce,
            transaction_hash: hash,
            signature: Signature::new(&bincode::serialize(&hash).unwrap(), ed25519_keypair),
        }
    }
}

// struct PartialTransaction is a part of Transaction for easily count the hash value of a transaction
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct PartialTransaction {
    pub trans_type: TransactionType,
    pub submitter: Address,
    pub timestamp: u64,
    pub payload: Vec<u8>,
    nonce: u128,
}

impl PartialTransaction {
    pub fn new(trans_type: TransactionType, submitter: Address, payload: Vec<u8>) -> Self {
        Self {
            trans_type,
            submitter,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            payload,
            nonce: rand::thread_rng().gen::<u128>(),
        }
    }
}

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string_pretty(&self).expect("json format error");
        write!(f, "{}", json)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_block() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = HashDigest::new(&get_publickey_from_keypair(&keypair).encode());

        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            PartialTransaction::new(TransactionType::Create, local_id, data.as_bytes().to_vec()),
            &keypair,
        );
        transactions.push(transaction);
        let block_header = Header::new(PartialHeader::new(
            HashDigest::new(b""),
            local_id,
            HashDigest::new(b""),
            1,
        ));
        let block = Block::new(block_header, transactions.to_vec(), &keypair);

        assert_eq!(1, block.header.number);
        Ok(())
    }
}
