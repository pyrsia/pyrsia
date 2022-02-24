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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
pub struct HashDigest {
    multihash: Multihash,
}

impl HashDigest {
    // TODO(prince-chrismc): Define Hash size constant

    pub fn new(msg: &[u8]) -> Self {
        Self {
            multihash: Code::Keccak256.digest(msg),
        }
    }

    // TODO(prince-chrismc): Define `to_bytes` for fixed sized array
}

impl aleph_bft::Hasher for HashDigest {
    type Hash = [u8; 32];

    fn hash(x: &[u8]) -> Self::Hash {
        Code::Keccak256.digest(x).to_bytes().try_into().unwrap()
    }
}
