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

use super::{get_config, RegistryError, RegistryErrorCode};
use crate::artifact_service::service::ArtifactService;
use crate::node_api::model::cli::Status;

use log::debug;
use std::sync::Arc;
use tokio::sync::Mutex;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_peers(
    artifact_service: Arc<Mutex<ArtifactService>>,
) -> Result<impl Reply, Rejection> {
    let peers = artifact_service
        .lock()
        .await
        .p2p_client
        .list_peers()
        .await
        .map_err(RegistryError::from)?;
    debug!("Got received_peers: {:?}", peers);

    let str_peers: Vec<String> = peers.into_iter().map(|p| p.to_string()).collect();
    let str_peers_as_json = serde_json::to_string(&str_peers).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(str_peers_as_json)
        .unwrap())
}

pub async fn handle_get_status(
    artifact_service: Arc<Mutex<ArtifactService>>,
) -> Result<impl Reply, Rejection> {
    let mut artifact_service = artifact_service.lock().await;
    let peers = artifact_service
        .p2p_client
        .list_peers()
        .await
        .map_err(RegistryError::from)?;

    let cli_config = get_config();
    if cli_config.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(cli_config.err().unwrap().to_string()),
        }));
    }

    let status = Status {
        peers_count: peers.len(),
        peer_id: artifact_service.p2p_client.local_peer_id.to_string(),
    };

    let status_as_json = serde_json::to_string(&status).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(status_as_json)
        .unwrap())
}
