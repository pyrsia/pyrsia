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

use libp2p::PeerId;
use serde::{Deserialize, Serialize};
use std::fmt::Display;
use std::path::PathBuf;

use crate::artifact_service::model::PackageType;

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum BuildStatus {
    Running,
    Success { artifact_urls: Vec<String> },
    Failure(String),
}

impl Display for BuildStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let build_status = match self {
            BuildStatus::Running => String::from("RUNNING"),
            BuildStatus::Success { .. } => String::from("SUCCESS"),
            BuildStatus::Failure(message) => format!("FAILED - (Error: {})", message),
        };
        write!(f, "{}", build_status)
    }
}

#[derive(Clone, Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct BuildInfo {
    pub id: String,
    pub status: BuildStatus,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BuildResultArtifact {
    pub artifact_specific_id: String,
    pub artifact_location: PathBuf,
    pub artifact_hash: String,
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum BuildTrigger {
    FromSource,
    Verification(PeerId),
}

#[derive(Debug)]
pub struct BuildResult {
    pub package_type: PackageType,
    pub package_specific_id: String,
    pub artifacts: Vec<BuildResultArtifact>,
}
