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

use crate::artifact_service::model::PackageType;

use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub enum SourceRepository {
    Git { url: String, tag: String },
}

#[derive(Debug, Deserialize, Serialize, Eq, PartialEq)]
pub struct MappingInfo {
    pub package_type: PackageType,
    pub package_specific_id: String,
    pub source_repository: Option<SourceRepository>,
    pub build_spec_url: Option<String>,
}
