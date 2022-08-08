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

use serde::{Deserialize, Serialize};
use std::path::PathBuf;

use crate::artifact_service::model::PackageType;

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub enum BuildStatus {
    Running,
    Success { artifact_urls: Vec<String> },
    Failure(String),
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq)]
pub struct BuildInfo {
    pub id: String,
    pub status: BuildStatus,
}

#[derive(Debug)]
pub struct BuildResultArtifact {
    pub artifact_specific_id: String,
    pub artifact_location: PathBuf,
    pub artifact_hash: String,
}

#[derive(Debug)]
pub enum BuildTrigger {
    FromSource,
    Verification,
}

#[derive(Debug)]
pub struct BuildResult {
    pub package_type: PackageType,
    pub package_specific_id: String,
    pub artifacts: Vec<BuildResultArtifact>,
}
