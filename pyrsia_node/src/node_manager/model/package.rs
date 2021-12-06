extern crate serde_json;
use serde_json::{Map, Value};
use strum_macros::{EnumIter, EnumString};

#[derive(Debug, Default)]
pub struct Package {
    pub name: String,
    pub r#type: String,
    pub namespace_id: String,
    pub creation_time: String,
    pub modified_time: String,
    pub administrator: String,
    pub description: String,
    pub namespace_path: Vec<u8>,
    pub metadata: Map<String, Value>,
    pub project_url: String,
    pub project_name: String,
    pub versions: Vec<u8>,
}

#[derive(Debug, Default)]
pub struct PackageVersion {
    pub id: String,
    pub version: String,
    pub license_text: String,
    pub license_text_mimetype: LicenseTextMimeType,
    pub license_url: String,
    pub creation_time: String,
    pub metadata: Map<String, Value>,
    pub tags: String,
    pub description: String,
    pub is_release: bool,
}

#[derive(EnumIter, Debug, PartialEq, EnumString)]
pub enum LicenseTextMimeType {
    TEXT,
    HTML,
    XML,
}

impl Default for LicenseTextMimeType {
    fn default() -> Self {
        LicenseTextMimeType::TEXT
    }
}
