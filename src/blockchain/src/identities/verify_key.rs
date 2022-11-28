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

use codec::{Decode, Encode, Error as CodecError, Input};
use libp2p::core::identity::ed25519::PublicKey;
use log::{debug, trace};

// Convenience wrapper around a ed25519::PublicKey to support the parity scale coded required by aleph_bft
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct VerifyKey {
    pub public: PublicKey,
}

impl VerifyKey {
    pub fn public(&self) -> PublicKey {
        self.public.clone()
    }
}

impl Encode for VerifyKey {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.public().encode().using_encoded(f)
    }

    fn size_hint(&self) -> usize {
        32
    }
}

impl Decode for VerifyKey {
    fn decode<I: Input>(value: &mut I) -> Result<Self, CodecError> {
        let mut buf = [0u8; 32];
        value.read(&mut buf)?;
        let public = PublicKey::decode(&buf).map_err(|e| {
            debug!("public key decode failed with: {:?}", e);
            trace!("Failed to decode public key: {}", hex::encode(buf));
            CodecError::from("public key decoded from_bytes")
        })?;
        Ok(Self { public })
    }
}
