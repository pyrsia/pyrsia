extern crate serde_json;
use super::HashAlgorithm;
use serde_json::{Map, Value};

#[derive(Debug)]
pub struct Artifact<'a> {
    pub hash: &'a [u8],
    pub algorithm: HashAlgorithm,
    pub name: Option<String>,
    pub creation_time: Option<String>,
    pub url: Option<String>,
    pub size: u32,
    pub mime_type: Option<String>,
    pub metadata: Map<String, Value>,
    pub source_url: Option<String>,
    pub art_type: Option<String>,
}
