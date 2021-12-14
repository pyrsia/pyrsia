extern crate serde_json;
use serde_json::{Map, Value};
use strum_macros::{EnumIter, EnumString};

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

#[derive(Debug)]
pub struct PackageVersion {
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
