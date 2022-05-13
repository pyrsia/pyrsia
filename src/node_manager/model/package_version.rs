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

    pub fn get_artifact_by_mime_type(&self, mime_types: Vec<&str>) -> Option<&Artifact> {
        for artifact in &self.artifacts {
            if let Some(mime_type) = artifact.mime_type() {
                if mime_types.contains(&mime_type.as_str()) {
                    return Some(artifact);
                }
            }
        }
        None
    }
}

#[derive(EnumIter, Debug, Display, PartialEq, EnumString, Serialize, Deserialize, Clone)]
pub enum LicenseTextMimeType {
    Text,
    Html,
    Xml,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts_repository::hash_util::HashAlgorithm;
    use crate::node_manager::model::artifact::ArtifactBuilder;

    #[test]
    fn get_artifact_by_mime_type() -> anyhow::Result<()> {
        let artifact: Artifact = ArtifactBuilder::default()
            .hash(vec![0x38u8, 0x4fu8])
            .algorithm(HashAlgorithm::SHA256)
            .mime_type("text/xml".to_string())
            .name("acme".to_string())
            .build()?;

        let package_version = PackageVersion::new(
            "id".to_string(),
            "namespace_id".to_string(),
            "name".to_string(),
            PackageTypeName::Docker,
            Map::new(),
            "version".to_string(),
            vec![artifact.clone()],
        );

        let artifacts = package_version.get_artifact_by_mime_type(vec!["application/json"]);
        assert!(artifacts.is_none());

        let artifacts = package_version.get_artifact_by_mime_type(vec!["text/xml"]);
        assert!(artifacts.is_some());
        assert_eq!(artifacts.unwrap().hash(), artifact.hash());

        let artifacts =
            package_version.get_artifact_by_mime_type(vec!["application/json", "text/xml"]);
        assert!(artifacts.is_some());
        assert_eq!(artifacts.unwrap().hash(), artifact.hash());

        Ok(())
    }
}
