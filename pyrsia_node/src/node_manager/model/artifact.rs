extern crate serde_json;
use super::HashAlgorithm;
use serde_json::{Map, Value};

#[derive(Debug, Default)]
pub struct Artifact<'a> {
    pub hash: &'a [u8],
    pub algorithm: HashAlgorithm,
    pub artifact_name: String,
    pub creation_time: String,
    pub url: String,
    pub artifact_size: u32,
    pub mime_type: String,
    pub metadata: Map<String, Value>,
    pub source_url: String,
    pub artifact_type: String,
}
