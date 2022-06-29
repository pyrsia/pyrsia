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

use anyhow::anyhow;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::fmt::{Display, Formatter};
use strum_macros::{EnumIter, EnumString};

// We will provide implementations of this trait for each hash algorithm that we support.
pub trait Digester {
    fn update_hash(&mut self, input: &[u8]);

    fn finalize_hash(&mut self, hash_buffer: &mut [u8]);

    fn hash_size_in_bytes(&self) -> usize;
}

impl Digester for Sha256 {
    fn update_hash(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn finalize_hash(&mut self, hash_buffer: &mut [u8]) {
        hash_buffer.clone_from_slice(self.clone().finalize().as_slice());
    }

    fn hash_size_in_bytes(&self) -> usize {
        256 / 8
    }
}

impl Digester for Sha512 {
    fn update_hash(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn finalize_hash(&mut self, hash_buffer: &mut [u8]) {
        hash_buffer.clone_from_slice(self.clone().finalize().as_slice());
    }

    fn hash_size_in_bytes(&self) -> usize {
        512 / 8
    }
}

/// The types of hash algorithms that the artifact service supports
#[derive(EnumIter, Clone, Debug, PartialEq, EnumString, Serialize, Deserialize)]
pub enum HashAlgorithm {
    SHA256,
    SHA512,
}

impl HashAlgorithm {
    pub fn digest_factory(&self) -> Box<dyn Digester> {
        match self {
            HashAlgorithm::SHA256 => Box::new(Sha256::new()),
            HashAlgorithm::SHA512 => Box::new(Sha512::new()),
        }
    }

    /// Translate a HashAlgorithm to a string.
    pub fn hash_algorithm_to_str(&self) -> &'static str {
        match self {
            HashAlgorithm::SHA256 => "SHA256",
            HashAlgorithm::SHA512 => "SHA512",
        }
    }

    fn hash_length_in_bytes(&self) -> usize {
        match self {
            HashAlgorithm::SHA256 => 256 / 8,
            HashAlgorithm::SHA512 => 512 / 8,
        }
    }
}

impl Display for HashAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(HashAlgorithm::hash_algorithm_to_str(self))
    }
}

#[derive(PartialEq, Debug)]
pub struct Hash {
    pub algorithm: HashAlgorithm,
    pub bytes: Vec<u8>,
}

impl Hash {
    pub fn new(algorithm: HashAlgorithm, bytes: &[u8]) -> Result<Self, anyhow::Error> {
        let expected_length: usize = algorithm.hash_length_in_bytes();
        if bytes.len() == expected_length {
            Ok(Hash {
                algorithm,
                bytes: bytes.to_vec(),
            })
        } else {
            Err(anyhow!(format!("The hash value does not have the correct length for the algorithm. The expected length is {} bytes, but the length of the supplied hash is {}.", expected_length, bytes.len())))
        }
    }
}

impl Display for Hash {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{}:{}",
            self.algorithm.hash_algorithm_to_str(),
            hex::encode(&self.bytes)
        ))
    }
}

#[cfg(test)]
mod tests {
    pub use super::*;
    use strum::IntoEnumIterator;

    const TEST_ARTIFACT_HASH_256: [u8; 32] = [
        0x6b, 0x29, 0xf2, 0xf1, 0xe5, 0x02, 0x4c, 0x41, 0x95, 0x06, 0xe9, 0x50, 0x3e, 0x02, 0x4b,
        0x3d, 0x8a, 0x5a, 0x08, 0xb6, 0xf6, 0xd5, 0x5b, 0x68, 0x88, 0x66, 0x79, 0x52, 0xd1, 0x04,
        0x15, 0x54,
    ];
    const TEST_ARTIFACT_HASH_512: [u8; 64] = [
        0x6b, 0x29, 0xf2, 0xf1, 0xe5, 0x02, 0x4c, 0x41, 0x95, 0x06, 0xe9, 0x50, 0x3e, 0x02, 0x4b,
        0x3d, 0x8a, 0x5a, 0x08, 0xb6, 0xf6, 0xd5, 0x5b, 0x68, 0x88, 0x66, 0x79, 0x52, 0xd1, 0x04,
        0x15, 0x54, 0x83, 0x74, 0x5a, 0xc0, 0x84, 0xfe, 0xf2, 0x12, 0x29, 0xd6, 0x57, 0x2c, 0xd4,
        0x14, 0xf9, 0xb2, 0xa4, 0x82, 0x06, 0xd6, 0x47, 0x62, 0xc5, 0x26, 0x81, 0x11, 0xd1, 0xc4,
        0x7a, 0x87, 0x4e, 0x71,
    ];

    const SHA256_HASH_ENCODED: &str =
        "5e6009f8ce7a159884aa5e5132ce8c84fefc979f237a0bce4652f90bc77e5591";
    const SHA512_HASH_ENCODED: &str = "838d2542932c2545f222a4daf74e0e1dc1bd76ce5742b4e3a92aaff2e28b038adf50e0bbdfe6da50ff4fc19f8a23a77ce8fd28a38456b33d43a62b3c86978954";

    #[test]
    pub fn test_digester_length() {
        for algorithm in HashAlgorithm::iter() {
            let digester = algorithm.digest_factory();

            assert_eq!(
                digester.hash_size_in_bytes(),
                algorithm.hash_length_in_bytes()
            );
        }
    }

    #[test]
    pub fn test_digester_256() {
        let mut digester = HashAlgorithm::SHA256.digest_factory();
        digester.update_hash(&TEST_ARTIFACT_HASH_256);
        let mut hash_buffer = [0; 32];
        digester.finalize_hash(&mut hash_buffer);
        assert_eq!(hex::encode(hash_buffer), SHA256_HASH_ENCODED);
    }

    #[test]
    pub fn test_digester_512() {
        let mut digester = HashAlgorithm::SHA512.digest_factory();
        digester.update_hash(&TEST_ARTIFACT_HASH_256);
        let mut hash_buffer = [0; 64];
        digester.finalize_hash(&mut hash_buffer);
        assert_eq!(hex::encode(hash_buffer), SHA512_HASH_ENCODED);
    }

    #[test]
    pub fn test_hash_new() {
        let hash = Hash::new(HashAlgorithm::SHA256, &TEST_ARTIFACT_HASH_256).unwrap();

        assert_eq!(hash.algorithm, HashAlgorithm::SHA256);
        assert_eq!(hash.bytes, TEST_ARTIFACT_HASH_256);
    }

    #[test]
    pub fn test_hash_256_display() {
        let hash = Hash::new(HashAlgorithm::SHA256, &TEST_ARTIFACT_HASH_256).unwrap();
        let display = format!("{}", hash);
        let to_string = format!(
            "{}:{}",
            hash.algorithm.hash_algorithm_to_str(),
            hex::encode(hash.bytes)
        );
        assert_eq!(display, to_string);
    }

    #[test]
    pub fn test_hash_512_display() {
        let hash = Hash::new(HashAlgorithm::SHA512, &TEST_ARTIFACT_HASH_512).unwrap();
        let display = format!("{}", hash);
        let to_string = format!(
            "{}:{}",
            hash.algorithm.hash_algorithm_to_str(),
            hex::encode(hash.bytes)
        );
        assert_eq!(display, to_string);
    }

    #[test]
    pub fn hash_length_does_not_match_algorithm_test() {
        assert!(
            Hash::new(HashAlgorithm::SHA512, &[0u8; 7]).is_err(),
            "A 56 bit hash value for SHA512 should be an error"
        )
    }
}
