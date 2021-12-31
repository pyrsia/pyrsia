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
extern crate pyrsia_client_lib;
extern crate serde;
extern crate serde_json;
use super::super::HashAlgorithm;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

/// Describes an individual artifact. This is not a signed struct because it is normally stored as
/// part a descripion of something that contains artifacts.
#[derive(Debug, Serialize, Deserialize)]
pub struct Artifact {
    /// The hash value that identifies the artifact.
    hash: Vec<u8>,
    /// The hash algorithm used to compute the hash value.
    algorithm: HashAlgorithm,
    /// The name of this artifact.
    name: Option<String>,
    /// ISO-8601 creation time
    creation_time: Option<String>,
    /// A URL associated with the artifact.
    url: Option<String>,
    /// The size of the artifact.
    size: Option<u64>,
    /// The mime type of the artifact
    mime_type: Option<String>,
    /// Attributes of an artifact that don't fit into one of this struct's fields can go in here as JSON
    metadata: Map<String, Value>,
    /// The URL of the source of the artifact
    source_url: Option<String>,
}

impl Artifact {
    #[allow(clippy::too_many_arguments)]
    fn new(
        hash: Vec<u8>,
        algorithm: HashAlgorithm,
        name: Option<String>,
        creation_time: Option<String>,
        url: Option<String>,
        size: Option<u64>,
        mime_type: Option<String>,
        metadata: Map<String, Value>,
        source_url: Option<String>,
    ) -> Artifact {
        Artifact {
            hash,
            algorithm,
            name,
            creation_time,
            url,
            size,
            mime_type,
            metadata,
            source_url,
        }
    }

    fn hash(&self) -> &Vec<u8> {
        &self.hash
    }

    fn algorithm(&self) -> &HashAlgorithm {
        &self.algorithm
    }

    fn name(&self) -> &Option<String> {
        &self.name
    }

    fn creation_time(&self) -> &Option<String> {
        &self.creation_time
    }

    fn url(&self) -> &Option<String> {
        &self.url
    }

    fn size(&self) -> &Option<u64> {
        &self.size
    }

    fn mime_type(&self) -> &Option<String> {
        &self.mime_type
    }

    fn metadata(&self) -> &Map<String, Value> {
        &self.metadata
    }

    fn source_url(&self) -> &Option<String> {
        &self.source_url
    }
}
