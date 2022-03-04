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

extern crate aleph_bft;

use multihash::{Code, Multihash, MultihashDigest};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct HashDigest {
    multihash: Multihash,
}

impl HashDigest {
    pub fn new(msg: &[u8]) -> Self {
        Self {
            multihash: Code::Keccak256.digest(msg),
        }
    }

    pub fn to_slice(&self) -> [u8; 32] {
        self.multihash
            .digest()
            .try_into()
            .expect("a valid Keccak256 to be 32 bytes")
    }
}

impl aleph_bft::Hasher for HashDigest {
    type Hash = [u8; 32];

    fn hash(x: &[u8]) -> Self::Hash {
        HashDigest::new(x).to_slice()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use aleph_bft::Hasher;

    #[test]
    fn test_hash_digest() {
        let message = b"hello world";
        let expected_digest = [
            0x47, 0x17, 0x32, 0x85, 0xa8, 0xd7, 0x34, 0x1e, 0x5e, 0x97, 0x2f, 0xc6, 0x77, 0x28,
            0x63, 0x84, 0xf8, 0x02, 0xf8, 0xef, 0x42, 0xa5, 0xec, 0x5f, 0x03, 0xbb, 0xfa, 0x25,
            0x4c, 0xb0, 0x1f, 0xad,
        ];

        let hash = HashDigest::new(message);

        assert_eq!(hash.to_slice(), expected_digest);
        assert_eq!(HashDigest::hash(message), expected_digest);
    }
}
