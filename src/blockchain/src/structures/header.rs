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
use libp2p::{identity, PeerId};
use multihash::Multihash;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

use crate::crypto::hash_algorithm::HashDigest;

#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Copy, Decode, Encode)]
pub struct Address {
    // This can not be libp2p's PeerId as it is missing the SCALE codec support for Aleph,
    // internally it's a https://github.com/libp2p/rust-libp2p/blob/6cc3b4ec52c922bfcf562a29b5805c3150e37c75/core/src/peer_id.rs#L40
    // So we will stick with that.
    peer_id: Multihash,
}

impl From<identity::PublicKey> for Address {
    fn from(key: identity::PublicKey) -> Address {
        Self {
            peer_id: PeerId::from_public_key(&key).into(),
        }
    }
}

impl From<PeerId> for Address {
    fn from(peer_id: PeerId) -> Address {
        Self {
            peer_id: peer_id.into(),
        }
    }
}

pub type Ordinal = u128;

// this struct exists only for generating a hash
#[derive(Serialize)]
struct PartialHeader {
    parent_hash: HashDigest,
    transactions_hash: HashDigest,
    committer: Address,
    timestamp: u64,
    ordinal: Ordinal,
    nonce: u128,
}

impl From<Header> for PartialHeader {
    fn from(header: Header) -> Self {
        PartialHeader {
            parent_hash: header.parent_hash,
            transactions_hash: header.transactions_hash,
            committer: header.committer,
            timestamp: header.timestamp,
            ordinal: header.ordinal,
            nonce: header.nonce,
        }
    }
}

fn calculate_hash(incomplete_header: &PartialHeader) -> Result<HashDigest, bincode::Error> {
    let bytes = bincode::serialize(incomplete_header)?;
    Ok(HashDigest::new(&bytes))
}

/// struct Header define the header of a block
#[derive(Serialize, Deserialize, Debug, Clone, Hash, PartialEq, Eq, Copy, Decode, Encode)]
pub struct Header {
    /// 256-bit Keccak Hash of the parent block (previous [`Block`][block]'s [`hash`][hash])
    ///
    /// [block]: crate::structures::block::Block
    /// [hash]: crate::structures::header::Header::hash
    pub parent_hash: HashDigest,
    /// 256-bit Keccak Hash of the [`Block`][block]'s [`transactions`][transactions]
    ///
    /// [block]: crate::structures::block::Block
    /// [transactions]: crate::structures::block::Block::transactions
    pub transactions_hash: HashDigest,
    /// the committer node's PeerId
    pub committer: Address,
    /// Unix timestamp in seconds, see <https://en.wikipedia.org/wiki/Unix_time> for more.
    pub timestamp: u64,
    /// block sequence number, the current block number should be the parent (previous) block number plus 1
    pub ordinal: Ordinal,
    /// Adds a salt to harden
    nonce: u128,
    /// The block id, 256-bit Keccak Hash of the Current Block Header, excluding itself
    hash: HashDigest,
}

impl Header {
    pub fn new(
        parent_hash: HashDigest,
        transactions_hash: HashDigest,
        committer: Address,
        ordinal: Ordinal,
    ) -> Self {
        let partial = PartialHeader {
            parent_hash,
            transactions_hash,
            committer,
            timestamp: SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            ordinal,
            nonce: rand::thread_rng().gen::<u128>(),
        };
        Self {
            parent_hash: partial.parent_hash,
            transactions_hash: partial.transactions_hash,
            committer: partial.committer,
            timestamp: partial.timestamp,
            ordinal: partial.ordinal,
            nonce: partial.nonce,
            hash: calculate_hash(&partial).unwrap(),
        }
    }

    pub fn hash(&self) -> HashDigest {
        self.hash
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use libp2p::identity;

    #[test]
    fn test_build_block_header() {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let header = Header::new(HashDigest::new(b""), HashDigest::new(b""), local_id, 5);

        let partial: PartialHeader = header.into();
        let expected_hash = calculate_hash(&partial).unwrap();

        assert_eq!(5, header.ordinal);
        assert_eq!(expected_hash, header.hash());
    }
}
