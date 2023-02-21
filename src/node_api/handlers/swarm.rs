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
use crate::docker::error_util::{RegistryError, RegistryErrorCode};
use crate::network::client::Client;
use crate::node_api::model::request::*;
use crate::transparency_log::log::TransparencyLog;
use std::future::Future;

use crate::artifact_service::service::ArtifactService;
use crate::build_service::error::BuildError;
use crate::node_api::model::response::BuildSuccessResponse;
use libp2p::PeerId;
use log::debug;
use serde::ser::SerializeStruct;
use serde::{Serialize, Serializer};
use std::str::FromStr;
use warp::{http::StatusCode, Rejection, Reply};

#[derive(Default)]
struct ResponseBuilder {
    format: ContentType,
    output_fields: Content,
}

pub struct OutputTransparencyLog<'a> {
    output_fields: &'a Content,
    origin: &'a TransparencyLog,
}

impl Serialize for OutputTransparencyLog<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut s = serializer.serialize_struct("TransparencyLog", self.output_fields.size())?;
        for field in &self.output_fields.fields {
            match field {
                TransparencyLogField::Id => s.serialize_field("id", &self.origin.id)?,
                TransparencyLogField::PackageType => {
                    s.serialize_field("package_type", &self.origin.package_type)?
                }
                TransparencyLogField::PackageSpecificId => {
                    s.serialize_field("package_specific_id", &self.origin.package_specific_id)?
                }
                TransparencyLogField::NumArtifacts => {
                    s.serialize_field("num_artifacts", &self.origin.num_artifacts)?
                }
                TransparencyLogField::PackageSpecificArtifactId => s.serialize_field(
                    "package_specific_artifact_id",
                    &self.origin.package_specific_artifact_id,
                )?,
                TransparencyLogField::ArtifactHash => {
                    s.serialize_field("artifact_hash", &self.origin.artifact_hash)?
                }
                TransparencyLogField::SourceHash => {
                    s.serialize_field("source_hash", &self.origin.source_hash)?
                }
                TransparencyLogField::ArtifactId => {
                    s.serialize_field("artifact_id", &self.origin.artifact_id)?
                }
                TransparencyLogField::SourceId => {
                    s.serialize_field("source_id", &self.origin.source_id)?
                }
                TransparencyLogField::Timestamp => {
                    s.serialize_field("timestamp", &self.origin.timestamp)?
                }
                TransparencyLogField::Operation => {
                    s.serialize_field("operation", &self.origin.operation)?
                }
                TransparencyLogField::NodeId => {
                    s.serialize_field("node_id", &self.origin.node_id)?
                }
                TransparencyLogField::NodePublicKey => {
                    s.serialize_field("node_public_key", &self.origin.node_public_key)?
                }
            };
        }

        s.end()
    }
}

impl ResponseBuilder {
    pub fn from(output_params: Option<TransparencyLogOutputParams>) -> Self {
        if let Some(params) = output_params {
            ResponseBuilder {
                format: params.format.unwrap_or_default(),
                output_fields: params.content.unwrap_or_default(),
            }
        } else {
            Default::default()
        }
    }

    fn wrap<'a>(&'a self, logs: &'a [TransparencyLog]) -> Vec<OutputTransparencyLog> {
        logs.iter()
            .map(|l| OutputTransparencyLog {
                output_fields: &self.output_fields,
                origin: l,
            })
            .collect()
    }

    pub fn create_response(&self, logs: &[TransparencyLog]) -> Result<impl Reply, Rejection> {
        let wrapped_logs = self.wrap(logs);
        let body = self.format.as_string(&wrapped_logs)?;

        Ok(warp::http::response::Builder::new()
            .status(StatusCode::OK)
            .header("Content-Type", self.format.response_content_type())
            .header("Content-Length", body.as_bytes().len())
            .body(body)
            .map_err(RegistryError::from)?)
    }
}

pub async fn handle_add_authorized_node(
    request_add_authorized_node: RequestAddAuthorizedNode,
    artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let peer_id =
        PeerId::from_str(&request_add_authorized_node.peer_id).map_err(|_| RegistryError {
            code: RegistryErrorCode::BadRequest(format!(
                "PeerId has invalid format: {}",
                request_add_authorized_node.peer_id
            )),
        })?;

    artifact_service
        .transparency_log_service
        .add_authorized_node(peer_id)
        .await
        .map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .status(StatusCode::CREATED)
        .body(""))
}

/// Special handle for Artifact Already Exist before responding to build request result
async fn handle_err_artifact_already_exists<F>(
    f: impl FnOnce() -> F,
) -> Result<BuildSuccessResponse, RegistryError>
where
    F: Future<Output = Result<String, BuildError>>,
{
    let request_build_result = f().await;
    match request_build_result {
        Ok(build_result_str) => Ok(BuildSuccessResponse {
            build_id: Some(build_result_str),
            message: None,
            success_status_code: StatusCode::OK,
        }),
        Err(err) => match err {
            BuildError::ArtifactAlreadyExists(_) => Ok(BuildSuccessResponse {
                build_id: None,
                message: Some(err.to_string()),
                success_status_code: StatusCode::FOUND,
            }),
            _ => Err(RegistryError::from(err)),
        },
    }
}

pub async fn handle_build_docker(
    request_docker_build: RequestDockerBuild,
    artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let request_build_result = || async {
        artifact_service
            .request_build(PackageType::Docker, {
                get_package_specific_id(&request_docker_build.image)
            })
            .await
    };

    let build_id = handle_err_artifact_already_exists(request_build_result).await?;

    let build_id_as_json = serde_json::to_string(&build_id).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(build_id.success_status_code)
        .body(build_id_as_json))
}

pub async fn handle_build_maven(
    request_maven_build: RequestMavenBuild,
    artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let request_build_result = || async {
        artifact_service
            .request_build(PackageType::Maven2, request_maven_build.gav)
            .await
    };

    let build_id = handle_err_artifact_already_exists(request_build_result).await?;

    let build_id_as_json = serde_json::to_string(&build_id).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(build_id.success_status_code)
        .body(build_id_as_json))
}

pub async fn handle_build_status(
    request_build_status: RequestBuildStatus,
    mut artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let build_id = request_build_status.build_id;

    let result = artifact_service
        .get_build_status(&build_id)
        .await
        .map_err(RegistryError::from)?;

    let build_status = serde_json::to_string(&result).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(build_status))
}

pub async fn handle_get_peers(p2p_client: Client) -> Result<impl Reply, Rejection> {
    let peers = p2p_client.list_peers().await.map_err(RegistryError::from)?;
    debug!("Got received_peers: {:?}", peers);

    let str_peers: Vec<String> = peers.into_iter().map(|p| p.to_string()).collect();
    let str_peers_as_json = serde_json::to_string(&str_peers).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(str_peers_as_json)
        .unwrap())
}

pub async fn handle_get_status(p2p_client: Client) -> Result<impl Reply, Rejection> {
    let status = p2p_client.status().await.map_err(RegistryError::from)?;

    let status_as_json = serde_json::to_string(&status).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(status_as_json)
        .unwrap())
}

pub async fn handle_inspect_log_docker(
    request_docker_log: RequestDockerLog,
    artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let result = artifact_service
        .transparency_log_service
        .search_transparency_logs(
            &PackageType::Docker,
            get_package_specific_id(&request_docker_log.image).as_str(),
        )
        .map_err(RegistryError::from)?;

    ResponseBuilder::from(request_docker_log.output_params).create_response(&result)
}

pub async fn handle_inspect_log_maven(
    request_maven_log: RequestMavenLog,
    artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let result = artifact_service
        .transparency_log_service
        .search_transparency_logs(&PackageType::Maven2, &request_maven_log.gav)
        .map_err(RegistryError::from)?;

    ResponseBuilder::from(request_maven_log.output_params).create_response(&result)
}

fn get_package_specific_id(package_specific_id: &str) -> String {
    match package_specific_id.contains('/') {
        true => package_specific_id.to_owned(),
        false => format!("library/{}", package_specific_id),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_get_package_specific_id() {
        let package_specific_id = "library/alpine:3.16.2";
        assert_eq!(
            package_specific_id,
            get_package_specific_id(package_specific_id)
        )
    }

    #[test]
    fn test_get_package_specific_id_as_official_image() {
        let package_specific_id = "alpine:3.16.2";
        let official_image_tag = "library/alpine:3.16.2";
        assert_eq!(
            official_image_tag,
            get_package_specific_id(package_specific_id)
        )
    }
}
