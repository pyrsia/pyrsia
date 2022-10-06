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

use aleph_bft::{NodeCount, NodeIndex, PartialMultisignature};
use async_trait::async_trait;
use libp2p::core::identity::ed25519::PublicKey;
use log::trace;

use super::authority_pen::AuthorityPen;
use super::authority_verifier::AuthorityVerifier;
use super::signature::{MultiSignature, Signature};

#[derive(Clone)]
pub struct KeyBox {
    authority_pen: AuthorityPen,
    authority_verifier: AuthorityVerifier,
}

impl KeyBox {
    pub fn new(authority_pen: AuthorityPen, authority_verifier: AuthorityVerifier) -> Self {
        let mut kb = Self {
            authority_pen,
            authority_verifier,
        };
        // Record the pen as a known authority -- always trust yourself
        kb.record_authority(kb.authority_pen.index(), kb.authority_pen.public());
        kb
    }
    pub fn record_authority(&mut self, node_index: NodeIndex, public_key: PublicKey) {
        self.authority_verifier.save(node_index, public_key);
    }
}

#[async_trait]
impl aleph_bft::KeyBox for KeyBox {
    type Signature = Signature;

    fn node_count(&self) -> NodeCount {
        self.authority_verifier.node_count()
    }

    async fn sign(&self, msg: &[u8]) -> Self::Signature {
        trace!("🖋️ {:?} signing message", self.authority_pen.index());
        self.authority_pen.sign(msg)
    }

    fn verify(&self, msg: &[u8], sgn: &Self::Signature, index: NodeIndex) -> bool {
        trace!(
            "🔎 {:?} verifying message and signature from {:?}",
            self.authority_pen.index(),
            index
        );
        self.authority_verifier.verify(msg, sgn, index)
    }
}

impl aleph_bft::MultiKeychain for KeyBox {
    type PartialMultisignature = MultiSignature;

    fn from_signature(
        &self,
        signature: &Signature,
        index: NodeIndex,
    ) -> Self::PartialMultisignature {
        MultiSignature::add_signature(
            MultiSignature::with_size(self.authority_verifier.node_count()),
            signature,
            index,
        )
    }
    fn is_complete(&self, msg: &[u8], partial: &Self::PartialMultisignature) -> bool {
        self.authority_verifier.is_complete(msg, partial)
    }
}

impl aleph_bft::Index for KeyBox {
    fn index(&self) -> NodeIndex {
        self.authority_pen.index()
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use aleph_bft::KeyBox as AlephKeyBox;
    use libp2p::core::identity::ed25519::Keypair;

    #[tokio::test]
    async fn test_key_box_self_signed() {
        let key_box = KeyBox::new(
            AuthorityPen::new(0.into(), Keypair::generate()),
            AuthorityVerifier::new(),
        );
        let sign: Signature = key_box.sign(b"hello world!").await;

        assert!(!key_box.verify(b"hello world", &sign, 0.into()));
    }
}
