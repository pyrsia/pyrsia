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
use crate::node_api::model::cli::{
    RequestAddAuthorizedNode, RequestBuildStatus, RequestDockerBuild, RequestDockerLog,
    RequestMavenBuild, RequestMavenLog,
};

use crate::artifact_service::service::ArtifactService;
use libp2p::PeerId;
use log::debug;
use std::str::FromStr;
use warp::{http::StatusCode, Rejection, Reply};

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

pub async fn handle_build_docker(
    request_docker_build: RequestDockerBuild,
    artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let build_id = artifact_service
        .request_build(PackageType::Docker, {
            get_package_specific_id(&request_docker_build.image)
        })
        .await
        .map_err(RegistryError::from)?;

    let build_id_as_json = serde_json::to_string(&build_id).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(build_id_as_json))
}

pub async fn handle_build_maven(
    request_maven_build: RequestMavenBuild,
    artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let build_id = artifact_service
        .request_build(PackageType::Maven2, request_maven_build.gav)
        .await
        .map_err(RegistryError::from)?;

    let build_id_as_json = serde_json::to_string(&build_id).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
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

pub async fn handle_get_peers(mut p2p_client: Client) -> Result<impl Reply, Rejection> {
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

pub async fn handle_get_status(mut p2p_client: Client) -> Result<impl Reply, Rejection> {
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

    let result_as_json = serde_json::to_string(&result).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(result_as_json)
        .unwrap())
}

pub async fn handle_inspect_log_maven(
    request_maven_log: RequestMavenLog,
    artifact_service: ArtifactService,
) -> Result<impl Reply, Rejection> {
    let result = artifact_service
        .transparency_log_service
        .search_transparency_logs(&PackageType::Maven2, &request_maven_log.gav)
        .map_err(RegistryError::from)?;

    let result_as_json = serde_json::to_string(&result).map_err(RegistryError::from)?;

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(result_as_json)
        .unwrap())
}

fn get_package_specific_id(package_specific_id: &str) -> String {
    match package_specific_id.contains('/') {
        true => package_specific_id.to_owned(),
        false => format!("library/{}", package_specific_id),
    }
}

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
