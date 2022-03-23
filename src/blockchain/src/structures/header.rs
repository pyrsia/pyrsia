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

use std::time::{SystemTime, UNIX_EPOCH};
use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::crypto::hash_algorithm::HashDigest;

pub type Address = HashDigest;

/// struct Header define the header of a block
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Copy)]
pub struct Header {
    /// 256bit Keccak Hash of the Parent Block(previous Block hash)
    pub parent_hash: HashDigest,
    /// the committer node's PeerID
    pub committer: Address,
    /// Unix timestamp in seconds, see https://en.wikipedia.org/wiki/Unix_time
    pub timestamp: u64,
    /// block sequence number, the current block number should be the parent(previous) block number plus 1
    pub ordinal: u128,
    /// Adds a salt to harden
    nonce: u128,
    /// block id, 256bit Keccak Hash of the Current Block Header, excluding itself
    pub hash: HashDigest,
}

// this struct exists only for generating a hash
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq)]
struct PartialHeader {
    parent_hash: HashDigest, //256bit Keccak Hash of the Parent Block
    committer: Address,      //the committer node's PeerID
    timestamp: u64,
    ordinal: u128,
    nonce: u128,
}
impl Header {
    pub fn new(parent_hash: HashDigest,
               committer: Address,
               ordinal: u128) -> Self {
        let partial = PartialHeader{
            parent_hash,
            committer,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ordinal,
            nonce: rand::thread_rng().gen::<u128>(),
        };
        Self {
            parent_hash : partial.parent_hash,
            committer : partial.committer,
            timestamp: partial.timestamp,
            ordinal : partial.ordinal,
            nonce: partial.nonce,
            hash: HashDigest::new(&(bincode::serialize(&partial).unwrap())),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use libp2p::identity;

    #[test]
    fn test_build_block_header() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = HashDigest::new(&keypair.public().encode());

        let header = Header::new(
            HashDigest::new(b""),
            local_id,
            5,
        );

        assert_eq!(5, header.ordinal);
        Ok(())
    }
}
