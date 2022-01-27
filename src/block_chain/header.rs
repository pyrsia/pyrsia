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

use multihash::{Code, Multihash, MultihashDigest};
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct HashDigest {
    multihash: Multihash,
}

pub type Address = HashDigest;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
pub struct Header {
    pub parent_hash: HashDigest, //256bit Keccak Hash of the Parent Block
    pub committer: Address,      //the committer node's PeerID
    pub transactions_root: HashDigest, //256bit Keccak Hash of the root node of Transaction Tries
    pub timestamp: u64,
    pub number: u128,
    pub nonce: u128,
    pub current_hash: HashDigest, //256bit Keccak Hash of the Current Block Header, excluding itself
}

impl Header {
    pub fn new(partial_header: PartialHeader) -> Self {
        Self {
            parent_hash: partial_header.parent_hash,
            committer: partial_header.committer,
            transactions_root: partial_header.transactions_root,
            timestamp: partial_header.timestamp,
            number: partial_header.number,
            nonce: partial_header.nonce,
            current_hash: hash(&(bincode::serialize(&partial_header).unwrap())),
        }
    }
}

pub fn hash(msg: &[u8]) -> HashDigest {
    HashDigest {
        multihash: Code::Keccak256.digest(msg),
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct PartialHeader {
    pub parent_hash: HashDigest, //256bit Keccak Hash of the Parent Block
    pub committer: Address,      //the committer node's PeerID
    pub transactions_root: HashDigest, //256bit Keccak Hash of the root node of Transaction Tries
    pub timestamp: u64,
    pub number: u128,
    pub nonce: u128,
}

impl PartialHeader {
    pub fn new(
        parent_hash: HashDigest,
        committer: Address,
        transactions_root: HashDigest,
        number: u128,
        nonce: u128,
    ) -> Self {
        Self {
            parent_hash,
            committer,
            transactions_root,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            number,
            nonce,
        }
    }
}

impl From<Header> for PartialHeader {
    fn from(header: Header) -> PartialHeader {
        Self {
            parent_hash: header.parent_hash,
            committer: header.committer,
            transactions_root: header.transactions_root,
            timestamp: header.timestamp,
            number: header.number,
            nonce: header.nonce,
        }
    }
}
