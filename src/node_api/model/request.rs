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

use crate::docker::error_util::RegistryError;
use crate::node_api::handlers::swarm::OutputTransparencyLog;
use serde::{Deserialize, Serialize};
use std::str::FromStr;

#[derive(Debug, Deserialize, Serialize, Default)]
pub struct Status {
    pub peers_count: usize,
    pub peer_id: String,
    pub peer_addrs: Vec<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestAddAuthorizedNode {
    pub peer_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestDockerBuild {
    pub image: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct TransparencyLogOutputParams {
    pub format: Option<ContentType>,
    pub content: Option<Content>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestDockerLog {
    pub image: String,
    pub output_params: Option<TransparencyLogOutputParams>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestMavenBuild {
    pub gav: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestMavenLog {
    pub gav: String,
    pub output_params: Option<TransparencyLogOutputParams>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct RequestBuildStatus {
    pub build_id: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum ContentType {
    JSON,
    CSV,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseContentTypeError {
    invalid_type: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum TransparencyLogField {
    Id,
    PackageType,
    PackageSpecificId,
    NumArtifacts,
    PackageSpecificArtifactId,
    ArtifactHash,
    SourceHash,
    ArtifactId,
    SourceId,
    Timestamp,
    Operation,
    NodeId,
    NodePublicKey,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Content {
    pub fields: Vec<TransparencyLogField>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ParseTransparencyLogFieldError {
    invalid_field: String,
}

impl Clone for ContentType {
    fn clone(&self) -> Self {
        match self {
            ContentType::JSON => ContentType::JSON,
            ContentType::CSV => ContentType::CSV,
        }
    }
}

impl Copy for ContentType {}

impl FromStr for ContentType {
    type Err = ParseContentTypeError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "json" => Ok(ContentType::JSON),
            "csv" => Ok(ContentType::CSV),
            _ => Err(ParseContentTypeError {
                invalid_type: s.to_owned(),
            }),
        }
    }
}

impl Default for ContentType {
    fn default() -> Self {
        ContentType::JSON
    }
}

impl ContentType {
    pub fn from(format: Option<&String>) -> Result<Self, ParseContentTypeError> {
        if let Some(val) = format {
            val.as_str().parse::<ContentType>()
        } else {
            Ok(Default::default())
        }
    }

    pub fn print_logs(&self, logs: String) {
        match self {
            ContentType::JSON => {
                let logs_as_json: serde_json::Value = serde_json::from_str(logs.as_str()).unwrap();
                println!("{}", serde_json::to_string_pretty(&logs_as_json).unwrap());
            }
            ContentType::CSV => {
                println!("{}", logs);
            }
        }
    }

    pub fn as_string(&self, logs: &Vec<OutputTransparencyLog>) -> Result<String, RegistryError> {
        Ok(match self {
            ContentType::JSON => serde_json::to_string(logs).map_err(RegistryError::from)?,
            ContentType::CSV => {
                let mut writer = csv::Writer::from_writer(vec![]);
                for rec in logs {
                    writer.serialize(rec).map_err(RegistryError::from)?;
                }

                let res = writer.into_inner().map_err(RegistryError::from)?;
                String::from_utf8(res).map_err(RegistryError::from)?
            }
        })
    }

    pub fn response_content_type(&self) -> String {
        match self {
            ContentType::JSON => "application/json".to_owned(),
            ContentType::CSV => "text/csv".to_owned(),
        }
    }
}

impl FromStr for TransparencyLogField {
    type Err = ParseTransparencyLogFieldError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let res = match s {
            "id" => TransparencyLogField::Id,
            "package_type" => TransparencyLogField::PackageType,
            "package_specific_id" => TransparencyLogField::PackageSpecificId,
            "num_artifacts" => TransparencyLogField::NumArtifacts,
            "package_specific_artifact_id" => TransparencyLogField::PackageSpecificArtifactId,
            "artifact_hash" => TransparencyLogField::ArtifactHash,
            "source_hash" => TransparencyLogField::SourceHash,
            "artifact_id" => TransparencyLogField::ArtifactId,
            "source_id" => TransparencyLogField::SourceId,
            "timestamp" => TransparencyLogField::Timestamp,
            "operation" => TransparencyLogField::Operation,
            "node_id" => TransparencyLogField::NodeId,
            "node_public_key" => TransparencyLogField::NodePublicKey,
            _ => {
                return Err(ParseTransparencyLogFieldError {
                    invalid_field: s.to_string(),
                })
            }
        };

        Ok(res)
    }
}

impl Clone for TransparencyLogField {
    fn clone(&self) -> Self {
        match self {
            TransparencyLogField::Id => TransparencyLogField::Id,
            TransparencyLogField::PackageType => TransparencyLogField::PackageType,
            TransparencyLogField::PackageSpecificId => TransparencyLogField::PackageSpecificId,
            TransparencyLogField::NumArtifacts => TransparencyLogField::NumArtifacts,
            TransparencyLogField::PackageSpecificArtifactId => {
                TransparencyLogField::PackageSpecificArtifactId
            }
            TransparencyLogField::ArtifactHash => TransparencyLogField::ArtifactHash,
            TransparencyLogField::SourceHash => TransparencyLogField::SourceHash,
            TransparencyLogField::ArtifactId => TransparencyLogField::ArtifactId,
            TransparencyLogField::SourceId => TransparencyLogField::SourceId,
            TransparencyLogField::Timestamp => TransparencyLogField::Timestamp,
            TransparencyLogField::Operation => TransparencyLogField::Operation,
            TransparencyLogField::NodeId => TransparencyLogField::NodeId,
            TransparencyLogField::NodePublicKey => TransparencyLogField::NodePublicKey,
        }
    }
}

impl TransparencyLogField {
    pub fn aaa(&self) -> (&str, &str) {
        match self {
            TransparencyLogField::Id => ("Id", "TransparencyLog record identity"),
            TransparencyLogField::PackageType => {
                ("PackageType", "Package type (maven, docker and so on)")
            }
            TransparencyLogField::PackageSpecificId => {
                ("PackageSpecificId", "Package specific identity")
            }
            TransparencyLogField::NumArtifacts => ("NumArtifacts", "Number of artifacts"),
            TransparencyLogField::PackageSpecificArtifactId => (
                "PackageSpecificArtifactId",
                "Package specific artifact identity",
            ),
            TransparencyLogField::ArtifactHash => ("ArtifactHash", "Artifact hash"),
            TransparencyLogField::SourceHash => ("SourceHash", "Source hash"),
            TransparencyLogField::ArtifactId => ("ArtifactId", "Artifact identity"),
            TransparencyLogField::SourceId => ("SourceId", "Source identity"),
            TransparencyLogField::Timestamp => ("Timestamp", "Timestamp"),
            TransparencyLogField::Operation => (
                "Operation",
                "Operation (AddArtifact, RemoveArtifact, AddNode, RemoveNode)",
            ),
            TransparencyLogField::NodeId => ("NodeId", "Peer node identity"),
            TransparencyLogField::NodePublicKey => ("NodePublicKey", "Node public key"),
        }
    }
}

impl Default for Content {
    fn default() -> Self {
        Content {
            fields: vec![
                TransparencyLogField::Id,
                TransparencyLogField::PackageType,
                TransparencyLogField::PackageSpecificId,
                TransparencyLogField::NumArtifacts,
                TransparencyLogField::PackageSpecificArtifactId,
                TransparencyLogField::ArtifactHash,
                TransparencyLogField::SourceHash,
                TransparencyLogField::ArtifactId,
                TransparencyLogField::SourceId,
                TransparencyLogField::Timestamp,
                TransparencyLogField::Operation,
                TransparencyLogField::NodeId,
                TransparencyLogField::NodePublicKey,
            ],
        }
    }
}

impl Content {
    pub fn from(fields: Option<Vec<String>>) -> Result<Self, ParseTransparencyLogFieldError> {
        Ok(if let Some(val) = fields {
            let mut res = Vec::with_capacity(val.len());
            for fld in val {
                res.push(fld.as_str().parse::<TransparencyLogField>()?)
            }

            Content { fields: res }
        } else {
            Default::default()
        })
    }

    pub fn size(&self) -> usize {
        self.fields.len()
    }
}
