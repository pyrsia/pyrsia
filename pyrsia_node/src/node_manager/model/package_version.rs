extern crate pyrsia_client_lib;
extern crate serde_json;

use pyrsia_client_lib::signed::Signed;
use serde_json::{Map, Value};
use signed_struct::signed_struct;
use strum_macros::{EnumIter, EnumString};

#[signed_struct]
#[derive(Debug)]
pub struct PackageVersion<'a> {
    id: String,
    version: String,
    pkg_id: String,
    license_text: Option<String>,
    license_text_mimetype: Option<LicenseTextMimeType>,
    license_url: Option<String>,
    creation_time: Option<String>,
    metadata: Map<String, Value>,
    tags: Vec<String>,
    description: Option<String>,
}

#[derive(EnumIter, Debug, PartialEq, EnumString)]
pub enum LicenseTextMimeType {
    Text,
    Html,
    Xml,
}
