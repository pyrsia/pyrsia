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

use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use super::crypto::hash_algorithm::HashDigest;

pub type Address = HashDigest;

/// struct Header define the header of a block
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub struct Header {
    ///      parent_hash: 256bit Keccak Hash of the Parent Block(previous Block hash)
    pub parent_hash: HashDigest,
    ///      committer:  the committer node's PeerID
    pub committer: Address,
    ///      transactions_root: 256bit Keccak Hash of the root node of Transactions Merkle tree
    pub transactions_root: HashDigest,
    ///      timestamp: Unix tim, the number of seconds that have elapsed since the Unix epoch, excluding leap seconds
    pub timestamp: u64,
    ///      number: block sequence number, the current block number should be the parent(previous) block number plus 1
    pub number: u128,
    nonce: u128, // Adds a salt to harden
    ///      hash: block id, 256bit Keccak Hash of the Current Block Header, excluding itself
    pub hash: HashDigest,
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
            hash: HashDigest::new(&(bincode::serialize(&partial_header).unwrap())),
        }
    }
}

// struct PartialHeader is a part of struct header for easily count the hash value of block
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
pub struct PartialHeader {
    pub parent_hash: HashDigest, //256bit Keccak Hash of the Parent Block
    pub committer: Address,      //the committer node's PeerID
    pub transactions_root: HashDigest, //256bit Keccak Hash of the root node of Transaction Tries
    pub timestamp: u64,
    pub number: u128,
    nonce: u128,
}

impl PartialHeader {
    pub fn new(
        parent_hash: HashDigest,
        committer: Address,
        transactions_root: HashDigest,
        number: u128,
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
            nonce: rand::thread_rng().gen::<u128>(),
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

#[cfg(test)]
mod tests {
    use super::super::block;
    use super::*;
    use libp2p::identity;

    #[test]
    fn test_build_block_header() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = HashDigest::new(&block::get_publickey_from_keypair(&keypair).encode());

        let header = Header::new(PartialHeader::new(
            HashDigest::new(b""),
            local_id,
            HashDigest::new(b""),
            5,
        ));

        assert_eq!(5, header.number);
        Ok(())
    }
}
