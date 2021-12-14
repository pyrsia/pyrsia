extern crate pyrsia_client_lib;
extern crate serde_json;

use pyrsia_client_lib::signed::Signed;
use serde_json::{Map, Value};

use signed_struct::signed_struct;

#[signed_struct]
#[derive(Debug)]
pub struct Package {
    name: String,
    pkg_type: String,
    namespace_id: String,
    creation_time: Option<String>,
    modified_time: Option<String>,
    administrator: Option<Vec<u8>>,
    description: Option<String>,
    metadata: Map<String, Value>,
    project_url: Option<String>,
    project_name: Option<String>,
    versions: Vec<u8>,
}
