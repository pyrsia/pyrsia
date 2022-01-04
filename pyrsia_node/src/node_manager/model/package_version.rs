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
extern crate pyrsia_client_lib;
extern crate serde_json;

use crate::node_manager::model::artifact::Artifact;
use crate::node_manager::model::package_type::PackageTypeName;
use pyrsia_client_lib::signed::Signed;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};
use signed_struct::signed_struct;
use strum_macros::{Display, EnumIter};

#[signed_struct]
#[derive(Debug)]
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
    metadata: Map<String, Value>,
    /// ISO-8601 creation time
    creation_time: Option<String>,
    /// ISO-8601 modification time
    modified_time: Option<String>,
    /// tags associated with this PackageVersion
    tags: Vec<String>,
    /// A description of this package version.
    description: Option<String>,
    /// The artifacts that are used by this package version.
    artifacts: Vec<Artifact>,
}

#[derive(EnumIter, Clone, Debug, PartialEq, Display, Serialize, Deserialize)]
pub enum LicenseTextMimeType {
    Text,
    Html,
    Xml,
}
