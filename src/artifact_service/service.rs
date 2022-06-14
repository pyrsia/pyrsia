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
