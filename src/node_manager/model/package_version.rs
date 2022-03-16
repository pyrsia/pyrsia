/*
   Copyright 2021 JFrog Ltd

   Licensed under the Apache License, Version 2.0 (the "License");
   you may not use this file except in compliance with the License.
   You may obtain a copy of the License at

       http://www.apache.org/licenses/LICENSE-2.0

   Unless required by applicable law or agreed to in writing, software
   distributed under the License is distributed on an "AS IS" BASIS,
   WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
   See the License for the specific language governing permissions and
   limitations under the License.
*/

extern crate serde_json;

use super::artifact::Artifact;
use super::package_type::PackageTypeName;

use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use strum_macros::{Display, EnumIter, EnumString};

#[derive(Debug, PartialEq, Deserialize, Serialize)]
pub struct PackageVersion {
    /// The unique ID of this package version
    pub id: String,
    /// The id of the namespace that this PackageVersion's package is part of.
    pub namespace_id: String,
    /// The name of this PackageVersions's package.
    pub name: String,
    /// The type of package (Docker, Conan, npm, ...)
    pub pkg_type: PackageTypeName,
    /// The version identifier for this package. It must be unique within the package that it belongs to.
    pub version: String,
    /// The text of the license for this package version.
    pub license_text: Option<String>,
    /// The type of text in the `license_text` field.
    pub license_text_mimetype: Option<LicenseTextMimeType>,
    /// The URL for the license for this package version.
    pub license_url: Option<String>,
    /// Attributes of a package version that don't fit into one of this struct's fields can go in here as JSON
    pub metadata: Map<String, Value>,
    /// ISO-8601 creation time
    pub creation_time: Option<String>,
    /// ISO-8601 modification time
    pub modified_time: Option<String>,
    /// tags associated with this PackageVersion
    pub tags: Vec<String>,
    /// A description of this package version.
    pub description: Option<String>,
    /// The artifacts that are used by this package version.
    pub artifacts: Vec<Artifact>,
}

impl PackageVersion {
    pub fn new(
        id: String,
        namespace_id: String,
        name: String,
        pkg_type: PackageTypeName,
        metadata: Map<String, Value>,
        version: String,
        artifacts: Vec<Artifact>,
    ) -> PackageVersion {
        PackageVersion {
            id,
            namespace_id,
            name,
            pkg_type,
            version,
            metadata,
            artifacts,

            license_text: Default::default(),
            creation_time: Default::default(),
            description: Default::default(),
            license_text_mimetype: Default::default(),
            license_url: Default::default(),
            modified_time: Default::default(),
            tags: Default::default(),
        }
    }

    pub fn id(&self) -> &String {
        &self.id
    }

    pub fn namespace_id(&self) -> &String {
        &self.namespace_id
    }

    pub fn name(&self) -> &String {
        &self.name
    }

    pub fn pkg_type(&self) -> &PackageTypeName {
        &self.pkg_type
    }

    pub fn version(&self) -> &String {
        &self.version
    }

    pub fn license_text(&self) -> &Option<String> {
        &self.license_text
    }
}

#[derive(EnumIter, Debug, Display, PartialEq, EnumString, Serialize, Deserialize, Clone)]
pub enum LicenseTextMimeType {
    Text,
    Html,
    Xml,
}
