extern crate pyrsia_client_lib;
extern crate serde_json;

use super::HashAlgorithm;
use pyrsia_client_lib::signed::Signed;
use serde_json::{Map, Value};

use signed_struct::signed_struct;

#[signed_struct]
#[derive(Debug)]
struct Artifact<'a> {
    hash: &'a [u8],
    algorithm: HashAlgorithm,
    name: Option<String>,
    creation_time: Option<String>,
    url: Option<String>,
    size: u32,
    mime_type: Option<String>,
    metadata: Map<String, Value>,
    source_url: Option<String>,
    art_type: Option<String>,
}
