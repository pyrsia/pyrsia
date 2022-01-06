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
use libp2p::{identity, Multiaddr, PeerId};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub id: u64,
    pub hash: String,
    pub previous_hash: String,
    pub timestamp: u128,
    pub data: String,
    pub nonce: u64,
}

pub struct H256(pub [u8; 32]); //Unformatted binary data of fixed length
pub struct Address(pub [u8; 20]); //Unformatted binary data of fixed length
pub struct U256(pub [u64; 4]); //Little-endian large integer type

pub struct Header {
    pub parent_hash: H256,       //256bit Keccak Hash of the Parent Block
    pub committer: Address,      //commit node PeerID
    pub transactions_root: H256, //256bit Keccak Hash of the root node of Transaction Tries
    //    pub state_root: H256,   //256bit Keccak Hash of the root node of state Tries
    pub timestamp: u64,
    pub number: U256,
    pub nonce: U256,
    pub signature: BlockSignature,
}

impl Header {
    pub fn new(
        parent_hash: H256,
        committer: Address,
        transactions_root: H256,
        number: U256,
        nonce: U256,
        signature: BlockSignature,
    ) -> Self {
        Self {
            parent_hash: parent_hash,
            committer: committer,
            transactions_root: transactions_root,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            number: number,
            nonce: nonce,
            signature: signature,
        }
    }
}

pub struct Block_v1 {
    pub header: Header,
    pub transactions: Vec<Transaction>,
}

pub struct Transaction {
    pub nonce: U256,
    pub trans_type: TransactionType,
    pub submmitter: Address,
    pub signature: TransactionSignature,
    pub payload: Vec<u8>,
}

pub struct Signature {
    signature: Vec<u8>,
    pubkey: identity::ed25519::PublicKey,
}

type TransactionSignature = Signature;
type BlockSignature = Signature;

pub enum TransactionType {
    Create,
}

pub fn generate_ed25519() -> identity::ed25519::Keypair {
    //RFC8032
    identity::ed25519::Keypair::generate()
}

pub fn signature(keypair: &identity::ed25519::Keypair, msg: &[u8]) -> Vec<u8> {
    (*keypair).sign(msg)
}

pub fn get_publickey_from_keypair(
    keypair: &identity::ed25519::Keypair,
) -> identity::ed25519::PublicKey {
    (*keypair).public()
}

pub fn verify(pubkey: &identity::ed25519::PublicKey, msg: &[u8], sig: &[u8]) -> bool {
    (*pubkey).verify(msg, sig)
}
