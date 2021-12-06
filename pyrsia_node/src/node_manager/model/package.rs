extern crate serde_json;
use serde_json::{Map, Value};
use strum_macros::{EnumIter, EnumString};

#[derive(Debug)]
pub struct Package {
    pub name: String,
    pub pkg_type: String,
    pub namespace_id: String,
    pub creation_time: Option<String>,
    pub modified_time: Option<String>,
    pub administrator: Option<Vec<u8>>,
    pub description: Option<String>,
    pub metadata: Map<String, Value>,
    pub project_url: Option<String>,
    pub project_name: Option<String>,
    pub versions: Vec<u8>,
}

#[derive(Debug)]
pub struct PackageVersion {
    pub id: String,
    pub version: String,
    pub pkg_id: String,
    pub license_text: Option<String>,
    pub license_text_mimetype: Option<LicenseTextMimeType>,
    pub license_url: Option<String>,
    pub creation_time: Option<String>,
    pub metadata: Map<String, Value>,
    pub tags: Vec<String>,
    pub description: Option<String>,
    pub is_release: bool,
}

#[derive(EnumIter, Debug, PartialEq, EnumString)]
pub enum LicenseTextMimeType {
    TEXT,
    HTML,
    XML,
}
