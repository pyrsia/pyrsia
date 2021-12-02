extern crate serde_json;
use serde_json::{Map, Value};
use strum_macros::{EnumIter, EnumString};

#[derive(Debug)]
pub struct Package {
    package_name: String,
    package_type:String,
    namespace_id: String,
    creation_time: String,
    modified_time: String,
    administrator: String,
    description: String,
    namespace_path: Vec<u8>,
    metadata: Optional<Map<String, Value>>,
    project_url: String,
    project_name: String,
    versions:Vec<u8>
}

#[derive(Debug)]
pub struct PackageVersion {
    id: String,
    package_version: String,
    license_text: String,
    license_text_mimetype: LicenseTextMimeType,
    license_url: String,
    creation_time: String,
    metadata: Map<String, Value>,
    tags: String,
    description: String,
    is_release: bool

}

#[derive(EnumIter, Debug, PartialEq, EnumString)]
pub enum LicenseTextMimeType {
    HTML,
    TEXT,
    XML
}