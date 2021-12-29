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
use std::fmt::{Display, Formatter};

use libp2p::{identity, Multiaddr, PeerId};
use serde::{Deserialize, Serialize};

pub struct H256(pub [u8; 32]);

//Unformatted binary data of fixed length
pub struct Address(pub [u8; 20]);

//Unformatted binary data of fixed length
pub struct H64(pub [u8; 8]);

//Unformatted binary data of fixed length
pub struct U256(pub [u64; 4]); //Little-endian large integer type

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Header {
    pub parent_hash: H256,
    //256bit Keccak Hash of the Parent Block
    pub beneficiary: Address,
    //Beneficiary Address, currently this is the commit node PeerID
    pub transactions_root: H256,
    //256bit Keccak Hash of the root node of Transaction Trie
    pub state_root: H256,
    //256bit Keccak Hash of the root node of state Trie
    pub timestamp: u64,
    pub number: U256,
    pub nonce: H64,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Block {
    pub header: Header,
    pub transactions: Vec<Transaction>,
}

pub struct Transaction {
    pub nonce: U256,
    pub action: TransactionAction,
    pub value: U256,
    pub signature: TransactionSignature,
    pub input: Vec<u8>,
}

pub struct TransactionSignature {
    //RFC6979
    pub v: u64,
    pub r: H256,
    pub s: H256,
}

pub enum TransactionAction {
    Create,
}

//RFC8032
pub fn generate_ed25519() -> identity::ed25519::Keypair {
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

impl Display for Block {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string_pretty(&self).expect("json format error");
        write!(f, "{}", json)
    }
}

impl PartialEq<Self> for Block {
    fn eq(&self, other: &Self) -> bool {
        self.id == other.id
            && self.hash == other.hash
            && self.previous_hash == other.previous_hash
            && self.nonce == other.nonce
    }
}

impl Eq for Block {}
