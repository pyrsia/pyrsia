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

use aleph_bft::{NodeCount, NodeIndex};
use libp2p::core::identity::ed25519::PublicKey;
use log::{trace, warn};
use std::collections::HashMap;

use super::signature::{MultiSignature, Signature};

#[derive(Clone, Default)]
pub struct AuthorityVerifier {
    authorities: HashMap<NodeIndex, PublicKey>,
    // TODO(prince-chrismc): Re-introduce `NodeIndex` to associate with `PeerId` when adding `network`
    // peers_by_index: HashMap<NodeIndex, PeerId>,
}

impl AuthorityVerifier {
    pub fn new() -> AuthorityVerifier {
        Default::default()
    }
    pub fn save(&mut self, node_ix: NodeIndex, public_key: PublicKey) {
        trace!(
            "Recording new authority {:?} with {:?}",
            node_ix,
            public_key
        );
        self.authorities.insert(node_ix, public_key);
    }
    /// Verifies whether the message is correctly signed with the signature assumed to be made by a
    /// node of the given index.
    pub fn verify(&self, msg: &[u8], sgn: &Signature, index: NodeIndex) -> bool {
        let sig = sgn.clone().to_bytes();
        match self.authorities.get(&index) {
            Some(public_key) => public_key.verify(msg, &sig),
            None => {
                warn!("No public key for {:?}", index);
                false
            }
        }
    }

    pub fn node_count(&self) -> NodeCount {
        self.authorities.len().into()
    }

    fn threshold(&self) -> usize {
        2 * self.node_count().0 / 3 + 1
    }

    /// Verifies whether the given signature set is a correct and complete multisignature of the
    /// message. Completeness requires more than 2/3 of all authorities.
    pub fn is_complete(&self, msg: &[u8], partial: &MultiSignature) -> bool {
        let signature_count = partial.iter().count();
        if signature_count < self.threshold() {
            return false;
        }
        partial.iter().all(|(i, sgn)| self.verify(msg, sgn, i))
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use aleph_bft::PartialMultisignature;
    use libp2p::core::identity::ed25519::Keypair;

    #[test]
    fn test_auth_verifier_nothing() {
        let multi_signs = MultiSignature::with_size(0.into());
        let verifier = AuthorityVerifier::new();

        assert_eq!(verifier.node_count(), 0.into());
        assert_eq!(verifier.threshold(), 1);
        assert!(!verifier.is_complete(b"hello world", &multi_signs));
    }

    #[test]
    fn test_auth_verifier_empty() {
        let keypair = Keypair::generate();
        let signed = keypair.sign(b"hello world!");
        let sign = Signature::from_bytes(&signed).expect("signature to be valid");
        let multi_signs =
            MultiSignature::add_signature(MultiSignature::with_size(1.into()), &sign, 0.into());
        let verifier = AuthorityVerifier::new();

        assert_eq!(verifier.node_count(), 0.into());
        assert_eq!(verifier.threshold(), 1);
        assert!(!verifier.is_complete(b"hello world", &multi_signs));
    }

    #[test]
    fn test_auth_verifier_one_signature_valid() {
        let keypair = Keypair::generate();
        let signed = keypair.sign(b"hello world!");
        let sign = Signature::from_bytes(&signed).expect("signature to be valid");
        let node_index: NodeIndex = 0.into();
        let multi_signs =
            MultiSignature::add_signature(MultiSignature::with_size(1.into()), &sign, node_index);

        let mut verifier = AuthorityVerifier::new();
        verifier.save(node_index, keypair.public());

        assert_eq!(verifier.node_count(), 1.into());
        assert_eq!(verifier.threshold(), 1);
        assert!(!verifier.is_complete(b"hello world", &multi_signs));
    }
}
