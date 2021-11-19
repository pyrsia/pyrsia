use anyhow::{anyhow, Result};
use ring::digest;
use std::fmt::{Display, Formatter};
use crate::hash::HashAlgorithm::{SHA256, SHA512};

/// The digester trait is an abstraction that we use to hide the differences in the interfaces
/// provided for different hash algorithms. Each time we want to compute a hash, we will create a
/// struct that has an implementation of this trait.
/// We will provide implementations of this trait for each hash algorithm that we support.
pub trait Digester {
    /// Update the hash computation in the struct with the given input data. This should be called
    /// at least once for every hash computation.
    /// * input — A slice of bytes to be included in the hash computation.
    fn update_hash(&mut self, input: &[u8]);

    /// Returns the size in bytes of the hash value that will be produced by this struct. This is
    /// useful for allocating the memory to store the hash value.
    fn hash_size_in_bytes(&self) -> usize;

    /// This method is called once after all the data for the hash computation has been passed to
    /// the `update_hash` method. This method fills the mutable slice referenced by `hash_buffer`
    /// with the hash value.
    fn finalize_hash(&mut self, hash_buffer: &mut [u8]);
}

impl Digester for digest::Context {
    fn update_hash(&mut self, input: &[u8]) {
        self.update(input);
    }

    fn finalize_hash(&mut self, hash_buffer: &mut [u8]) {
        hash_buffer.copy_from_slice(self.clone().finish().as_ref());
    }

    fn hash_size_in_bytes(&self) -> usize {
        self.algorithm().output_len
    }
}

/// The types of hash algorithms that the artifact manager supports
#[derive(Debug, PartialEq)]
pub enum HashAlgorithm {
    SHA256,
    SHA512,
}

static HASH_ALGORITHMS: [HashAlgorithm;2] = [SHA256, SHA512];

impl HashAlgorithm {
    /// Call this method on a variant of HashAlgorithm to get a Digester that implements the algorithm
    pub fn digest_factory(&self) -> Box<dyn Digester> {
        match self {
            HashAlgorithm::SHA256 => Box::new(digest::Context::new(&digest::SHA256)),
            HashAlgorithm::SHA512 => Box::new(digest::Context::new(&digest::SHA512)),
        }
    }

    /// Translate a HashAlgorithm to a string.
    pub fn to_str(&self) -> &'static str {
        match self {
            HashAlgorithm::SHA256 => "SHA256",
            HashAlgorithm::SHA512 => "SHA512",
        }
    }

    /// Translate a string to a hashAlgorithm
    pub fn from_str(name: &str) -> Result<HashAlgorithm, anyhow::Error> {
        match name.to_uppercase().as_str() {
            "SHA256" => Ok(HashAlgorithm::SHA256),
            "SHA512" => Ok(HashAlgorithm::SHA512),
            _ => Err(anyhow!("Unknown hash algorithm {}", name))
        }
    }

    /// Return an iterator over the HashAlgorithm variants
    pub fn iter() -> core::slice::Iter<'static, HashAlgorithm>{
        HASH_ALGORITHMS.iter()
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
        f.write_str(HashAlgorithm::to_str(self))
    }
}

/// This struct is used to package a hash value and identify its associated algorithm.
pub struct Hash<'a> {
    pub algorithm: HashAlgorithm,
    pub bytes: &'a [u8],
}

impl<'a> Hash<'a> {
    pub fn new(algorithm: HashAlgorithm, bytes: &'a [u8]) -> anyhow::Result<Self, anyhow::Error> {
        let expected_length: usize = algorithm.hash_length_in_bytes();
        if bytes.len() == expected_length {
            Ok(Hash { algorithm, bytes })
        } else {
            Err(anyhow!(format!("The hash value does not have the correct length for the algorithm. The expected length is {} bytes, but the length of the supplied hash is {}.", expected_length, bytes.len())))
        }
    }
}

impl Display for Hash<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        f.write_str(&format!(
            "{}:{}",
            self.algorithm.to_str(),
            hex::encode(self.bytes)
        ))
    }
}
