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
extern crate signed;
extern crate signed_struct;

use super::artifact::Artifact;
use super::package_type::PackageTypeName;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use signed::signed::Signed;
use signed_struct::signed_struct;
use strum_macros::{EnumIter, EnumString};

#[signed_struct]
#[derive(Debug, PartialEq)]
pub struct PackageVersion {
    /// The unique ID of this package version
    id: String,
    /// The id of the namespace that this PackageVersion's package is part of.
    namespace_id: String,
    /// The name of this PackageVersions's package.
    name: String,
    /// The type of package (Docker, Conan, npm, ...)
    pkg_type: PackageTypeName,
    /// The version identifier for this package. It must be unique within the package that it belongs to.
    version: String,
    /// The text of the license for this package version.
    license_text: Option<String>,
    /// The type of text in the `license_text` field.
    license_text_mimetype: Option<LicenseTextMimeType>,
    /// The URL for the license for this package version.
    license_url: Option<String>,
    /// Attributes of a package version that don't fit into one of this struct's fields can go in here as JSON
    #[builder(default = "serde_json::Map::new()")]
    metadata: Map<String, Value>,
    /// ISO-8601 creation time
    creation_time: Option<String>,
    /// ISO-8601 modification time
    modified_time: Option<String>,
    /// tags associated with this PackageVersion
    #[builder(default = "Vec::new()")]
    tags: Vec<String>,
    /// A description of this package version.
    description: Option<String>,
    /// The artifacts that are used by this package version.
    artifacts: Vec<Artifact>,
}

#[derive(EnumIter, Debug, PartialEq, EnumString, Serialize, Deserialize, Clone)]
pub enum LicenseTextMimeType {
    Text,
    Html,
    Xml,
}
