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
use crate::transparency_log::log::TransparencyLogError;
use hyper::StatusCode;
use libp2p::PeerId;
use thiserror::Error;

#[derive(Debug, Error)]
pub enum BuildError {
    #[error("Build with ID {0} failed with error: {1}")]
    Failure(String, String),
    #[error("Failed to initialize a build: {0}")]
    InitializationFailed(String),
    #[error("Failure while accessing underlying storage: {0}")]
    TransparencyLogFailure(#[from] TransparencyLogError),
    #[error("Unauthorized PeerId: {0}")]
    UnauthorizedPeerId(PeerId),
    #[error("Artifact already exists. Fresh build not required: {0}")]
    ArtifactAlreadyExists(String),
    #[error("Invalid response from mapping service endpoint: {0}")]
    InvalidMappingResponse(String),
    #[error("Invalid response from pipeline service endpoint: {0}")]
    InvalidPipelineResponse(String),
    #[error("No mapping found for package with ID {package_specific_id} and type {package_type}")]
    MappingNotFound {
        package_type: PackageType,
        package_specific_id: String,
    },
    #[error("Request to mapping service endpoint failed with status {0}")]
    MappingServiceEndpointFailure(StatusCode),
    #[error("Failed to connect to mapping service endpoint: {0}")]
    MappingServiceEndpointRequestFailure(String),
    #[error("Request to pipeline service endpoint failed with status {0}")]
    PipelineServiceEndpointFailure(StatusCode),
    #[error("Failed to connect to pipeline service endpoint: {0}")]
    PipelineServiceEndpointRequestFailure(String),
    #[error("Failed to fetch build status: {0}")]
    BuildStatusFailed(String),
}
