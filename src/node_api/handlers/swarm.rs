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

use super::{RegistryError, RegistryErrorCode};
use crate::network::p2p;
use crate::node_manager::{handlers::*, model::cli::Status};
use log::debug;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_peers(mut p2p_client: p2p::Client) -> Result<impl Reply, Rejection> {
    let peers = p2p_client.list_peers().await;
    debug!("Got received_peers: {:?}", peers);

    let str_peers: Vec<String> = peers.into_iter().map(|p| p.to_string()).collect();
    let str_peers_as_json = serde_json::to_string(&str_peers).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(str_peers_as_json)
        .unwrap())
}

pub async fn handle_get_status(mut p2p_client: p2p::Client) -> Result<impl Reply, Rejection> {
    let peers = p2p_client.list_peers().await;

    let art_count_result = get_arts_count();
    if art_count_result.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(art_count_result.err().unwrap().to_string()),
        }));
    }

    let disk_space_result = disk_usage(ARTIFACTS_DIR.as_str());
    if disk_space_result.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(disk_space_result.err().unwrap().to_string()),
        }));
    }

    let status = Status {
        artifact_count: art_count_result.unwrap(),
        peers_count: peers.len(),
        peer_id: p2p_client.local_peer_id.to_string(),
        disk_allocated: String::from(ALLOCATED_SPACE_FOR_ARTIFACTS),
        disk_usage: format!("{:.4}", disk_space_result.unwrap()),
    };

    let status_as_json = serde_json::to_string(&status).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(status_as_json)
        .unwrap())
}
