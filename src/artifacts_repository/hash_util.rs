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

use anyhow::{anyhow, bail, Context, Result};
use multihash::{Multihash, MultihashGeneric};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256, Sha512};
use std::fmt::{Display, Formatter};
use std::str::FromStr;
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

/// The types of hash algorithms that the artifact manager supports
#[derive(EnumIter, Clone, Debug, PartialEq, EnumString, Serialize, Deserialize)]
pub enum HashAlgorithm {
    SHA256,
    SHA512,
}

impl HashAlgorithm {
    /// Translate a string that names a hash algorithm to the enum variant.
    pub fn str_to_hash_algorithm(algorithm_name: &str) -> Result<HashAlgorithm, anyhow::Error> {
        HashAlgorithm::from_str(&algorithm_name.to_uppercase()).with_context(|| {
            format!(
                "{} is not the name of a supported HashAlgorithm.",
                algorithm_name
            )
        })
    }

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

impl std::fmt::Display for HashAlgorithm {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(HashAlgorithm::hash_algorithm_to_str(self))
    }
}

#[derive(PartialEq, Debug)]
pub struct Hash {
    pub algorithm: HashAlgorithm,
    pub bytes: Vec<u8>,
}

// Code values used to indicate hash algorithms in Multihash. These values come from https://github.com/multiformats/multicodec/blob/master/table.csv
const MH_SHA2_256: u64 = 0x12;
const MH_SHA2_512: u64 = 0x13;

impl<'a> Hash {
    pub fn new(algorithm: HashAlgorithm, bytes: &'a [u8]) -> Result<Self, anyhow::Error> {
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

    pub fn to_multihash(&self) -> Result<Multihash> {
        match self.algorithm {
            HashAlgorithm::SHA256 => MultihashGeneric::wrap(MH_SHA2_256, &self.bytes),
            HashAlgorithm::SHA512 => MultihashGeneric::wrap(MH_SHA2_512, &self.bytes),
        }
        .context("Error creating a multihash from a Hash struct")
    }

    pub fn from_multihash(mh: Multihash) -> Result<Hash> {
        match mh.code() {
            MH_SHA2_256 => Hash::new(HashAlgorithm::SHA256, mh.digest()),
            MH_SHA2_512 => Hash::new(HashAlgorithm::SHA512, mh.digest()),
            _ => bail!(
                "Unable to create Hash struct from multihash with unknown type code 0x{:x}",
                mh.code()
            ),
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
    use env_logger::Target;
    use log::LevelFilter;

    #[ctor::ctor]
    fn init() {
        let _ignore = env_logger::builder()
            .is_test(true)
            .target(Target::Stdout)
            .filter(None, LevelFilter::Debug)
            .try_init();
    }

    const TEST_ARTIFACT_HASH: [u8; 32] = [
        0x6b, 0x29, 0xf2, 0xf1, 0xe5, 0x02, 0x4c, 0x41, 0x95, 0x06, 0xe9, 0x50, 0x3e, 0x02, 0x4b,
        0x3d, 0x8a, 0x5a, 0x08, 0xb6, 0xf6, 0xd5, 0x5b, 0x68, 0x88, 0x66, 0x79, 0x52, 0xd1, 0x04,
        0x15, 0x54,
    ];

    #[test]
    pub fn hash_to_multihash_to_hash() -> Result<()> {
        let hash = Hash::new(HashAlgorithm::SHA256, &TEST_ARTIFACT_HASH)?;
        let hash2 = Hash::from_multihash(hash.to_multihash()?)?;
        assert_eq!(
            hash, hash2,
            "Hash converted to multihash converted to hash should equal the original hash"
        );
        Ok(())
    }

    #[test]
    pub fn hash_length_does_not_match_algorithm_test() {
        assert!(
            Hash::new(HashAlgorithm::SHA512, &[0u8; 7]).is_err(),
            "A 56 bit hash value for SHA512 should be an error"
        )
    }
}
