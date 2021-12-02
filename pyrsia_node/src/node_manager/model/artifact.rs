extern crate serde_json;
use super::HashAlgorithm;
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct Artifact<'a> {
    artifact_hash: &'a [u8],
    artifact_algorithm: HashAlgorithm,
    artifact_name: String,
    creation_time: String,
    url: String,
    artifact_size: u32,
    mime_type: String,
    metadata: Map<String, Value>,
    source_url: String,
    artifact_type: String,
}
