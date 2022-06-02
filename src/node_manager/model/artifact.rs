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
use super::super::HashAlgorithm;
use derive_builder::Builder;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Describes an individual artifact. This is not a signed struct because it is normally stored as
/// part a description of something that contains artifacts.
#[derive(Builder, Debug, Serialize, Deserialize, Clone, PartialEq)]
pub struct Artifact {
    /// The hash value that identifies the artifact.
    hash: Vec<u8>,
    /// The hash algorithm used to compute the hash value.
    algorithm: HashAlgorithm,
    /// The name of this artifact.
    #[builder(setter(strip_option), default)]
    name: Option<String>,
    /// ISO-8601 creation time
    #[builder(setter(strip_option), default)]
    creation_time: Option<String>,
    /// A URL associated with the artifact.
    #[builder(setter(strip_option), default)]
    url: Option<String>,
    /// The size of the artifact.
    #[builder(setter(strip_option), default)]
    size: Option<u64>,
    /// The mime type of the artifact
    #[builder(setter(strip_option), default)]
    mime_type: Option<String>,
    /// Attributes of an artifact that don't fit into one of this struct's fields can go in here as JSON
    #[builder(default)]
    metadata: Map<String, Value>,
    /// The URL of the source of the artifact
    #[builder(setter(strip_option), default)]
    source_url: Option<String>,
}

#[allow(unused)]
impl Artifact {
    pub fn hash(&self) -> &Vec<u8> {
        &self.hash
    }

    pub fn algorithm(&self) -> &HashAlgorithm {
        &self.algorithm
    }

    pub fn name(&self) -> &Option<String> {
        &self.name
    }

    pub fn creation_time(&self) -> &Option<String> {
        &self.creation_time
    }

    pub fn url(&self) -> &Option<String> {
        &self.url
    }

    pub fn size(&self) -> &Option<u64> {
        &self.size
    }

    pub fn mime_type(&self) -> &Option<String> {
        &self.mime_type
    }

    pub fn metadata(&self) -> &Map<String, Value> {
        &self.metadata
    }

    pub fn source_url(&self) -> &Option<String> {
        &self.source_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn builder_test() -> anyhow::Result<()> {
        let artifact: Artifact = ArtifactBuilder::default()
            .hash(vec![0x38u8, 0x4fu8])
            .algorithm(HashAlgorithm::SHA256)
            .name("acme".to_string())
            .build()?;
        println!("{:?}", artifact);
        match artifact.name() {
            Some(name) => assert_eq!("acme", name),
            None => assert!(false),
        }
        Ok(())
    }
}
