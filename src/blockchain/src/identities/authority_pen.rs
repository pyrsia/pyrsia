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

use aleph_bft::NodeIndex;
use libp2p::core::identity::ed25519::{Keypair, PublicKey};

use super::signature::Signature;

#[derive(Clone)]
pub struct AuthorityPen {
    index: NodeIndex,
    keypair: Keypair,
}

impl AuthorityPen {
    pub fn new(index: NodeIndex, keypair: Keypair) -> Self {
        Self { index, keypair }
    }
    pub fn index(&self) -> NodeIndex {
        self.index
    }
    pub fn public(&self) -> PublicKey {
        self.keypair.public()
    }
    pub fn sign(&self, msg: &[u8]) -> Signature {
        Signature::new(msg, &self.keypair)
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;

    #[test]
    fn test_auth_pen_sign() {
        let keypair = Keypair::generate();
        let auth_pen = AuthorityPen::new(0.into(), keypair.clone());
        let signed = auth_pen.sign(b"hello world!");

        assert!(keypair.public().verify(b"hello world!", &signed.to_bytes()));
    }
}
