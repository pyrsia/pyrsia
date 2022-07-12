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
use hyper::StatusCode;
use thiserror::Error;

#[derive(Debug, Error, PartialEq)]
pub enum BuildError {
    #[error("No mapping found for package with ID {package_specific_id} and type {package_type}")]
    MappingNotFound {
        package_type: PackageType,
        package_specific_id: String,
    },
    #[error("Request to mapping service endpoint failed with status {0}")]
    MappingServiceEndpointFailure(StatusCode),
    #[error("Failed to connect to mapping service endpoint: {0}")]
    MappingServiceEndpointRequestFailure(String),
    #[error("Build failed: {0}")]
    BuildFailure(String),
}

impl From<reqwest::Error> for BuildError {
    fn from(error: reqwest::Error) -> Self {
        BuildError::MappingServiceEndpointRequestFailure(error.to_string())
    }
}
