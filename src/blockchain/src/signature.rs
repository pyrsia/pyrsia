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

use aleph_bft::SignatureSet;
use codec::{Decode, Encode};
use libp2p::core::identity::ed25519::Keypair;
use serde::{Deserialize, Serialize};
use std::hash::{Hash, Hasher};

pub type Error = ed25519_dalek::SignatureError;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq, Decode)]
pub struct Signature {
    #[codec(encoded_as = "[u8; ed25519_dalek::Signature::BYTE_SIZE]")]
    signature: ed25519_dalek::Signature,
}

#[allow(clippy::derive_hash_xor_eq)] // https://github.com/rust-lang/rust-clippy/issues/7666
impl Hash for Signature {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.signature.to_bytes().hash(state);
    }
}

impl Signature {
    pub fn from_bytes(msg: &[u8]) -> Result<Self, Error> {
        let sig = ed25519_dalek::Signature::from_bytes(msg)?;
        Ok(Self { signature: sig })
    }
    pub fn to_bytes(self) -> [u8; ed25519_dalek::Signature::BYTE_SIZE] {
        self.signature.to_bytes()
    }
    pub fn new(msg: &[u8], keypair: &Keypair) -> Self {
        let signed: Vec<u8> = keypair.sign(msg);
        Signature::from_bytes(&signed).expect("signed data should always be valid")
    }
}

impl Encode for Signature {
    fn using_encoded<R, F: FnOnce(&[u8]) -> R>(&self, f: F) -> R {
        self.signature.to_bytes().using_encoded(f)
    }

    fn size_hint(&self) -> usize {
        ed25519_dalek::Signature::BYTE_SIZE
    }
}

pub type MultiSignature = SignatureSet<Signature>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_signature_encode() {
        let bytes: [u8; ed25519_dalek::Signature::BYTE_SIZE] = [
            0x6d, 0xd3, 0x55, 0x66, 0x7f, 0xae, 0x4e, 0xb4, 0x3c, 0x6e, 0x0a, 0xb9, 0x2e, 0x87,
            0x0e, 0xdb, 0x2d, 0xe0, 0xa8, 0x8c, 0xae, 0x12, 0xdb, 0xd8, 0x59, 0x15, 0x07, 0xf5,
            0x84, 0xfe, 0x49, 0x12, 0xba, 0xbf, 0xf4, 0x97, 0xf1, 0xb8, 0xed, 0xf9, 0x56, 0x7d,
            0x24, 0x83, 0xd5, 0x4d, 0xdc, 0x64, 0x59, 0xbe, 0xa7, 0x85, 0x52, 0x81, 0xb7, 0xa2,
            0x46, 0xa6, 0x09, 0xe3, 0x00, 0x1a, 0x4e, 0x08,
        ];
        let sign = Signature::from_bytes(&bytes).unwrap();
        println!("{:?}", sign.encode());

        assert_eq!(
            sign.encode(),
            vec![
                109, 211, 85, 102, 127, 174, 78, 180, 60, 110, 10, 185, 46, 135, 14, 219, 45, 224,
                168, 140, 174, 18, 219, 216, 89, 21, 7, 245, 132, 254, 73, 18, 186, 191, 244, 151,
                241, 184, 237, 249, 86, 125, 36, 131, 213, 77, 220, 100, 89, 190, 167, 133, 82,
                129, 183, 162, 70, 166, 9, 227, 0, 26, 78, 8
            ]
        );
    }

    #[test]
    fn test_signature_decode() {
        let bytes: [u8; ed25519_dalek::Signature::BYTE_SIZE] = [
            0x6d, 0xd3, 0x55, 0x66, 0x7f, 0xae, 0x4e, 0xb4, 0x3c, 0x6e, 0x0a, 0xb9, 0x2e, 0x87,
            0x0e, 0xdb, 0x2d, 0xe0, 0xa8, 0x8c, 0xae, 0x12, 0xdb, 0xd8, 0x59, 0x15, 0x07, 0xf5,
            0x84, 0xfe, 0x49, 0x12, 0xba, 0xbf, 0xf4, 0x97, 0xf1, 0xb8, 0xed, 0xf9, 0x56, 0x7d,
            0x24, 0x83, 0xd5, 0x4d, 0xdc, 0x64, 0x59, 0xbe, 0xa7, 0x85, 0x52, 0x81, 0xb7, 0xa2,
            0x46, 0xa6, 0x09, 0xe3, 0x00, 0x1a, 0x4e, 0x08,
        ];
        let expected = Signature::from_bytes(&bytes).unwrap();

        let mut da: &[u8] = &bytes;
        let sign = Signature::decode(&mut da);

        assert_eq!(sign.ok(), Some(expected));
    }
}
