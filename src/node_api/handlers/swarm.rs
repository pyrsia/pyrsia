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
use crate::artifact_service::handlers::*;
use crate::artifact_service::storage::ArtifactStorage;
use crate::network::client::Client;
use crate::node_api::model::cli::{ArtifactsSummary, Status};

use log::debug;
use std::collections::HashMap;
use warp::{http::StatusCode, Rejection, Reply};

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

pub async fn handle_get_status(
    mut p2p_client: Client,
    artifact_storage: ArtifactStorage,
) -> Result<impl Reply, Rejection> {
    let peers = p2p_client.list_peers().await.map_err(RegistryError::from)?;

    let art_count_result = get_arts_summary(&artifact_storage);
    if art_count_result.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(art_count_result.err().unwrap().to_string()),
        }));
    }

    let disk_space_result = disk_usage(&artifact_storage);
    if disk_space_result.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(disk_space_result.err().unwrap().to_string()),
        }));
    }

    let cli_config = get_config();
    if cli_config.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(cli_config.err().unwrap().to_string()),
        }));
    }
    let mut total_artifacts = 0;
    let mut art_summ_map: HashMap<String, usize> = HashMap::new();
    for (k, v) in art_count_result.unwrap().iter() {
        if k == "SHA256" {
            total_artifacts += v;
            art_summ_map.insert("blobs".to_string(), *v);
        } else if k == "SHA512" {
            total_artifacts += v;
            art_summ_map.insert("manifests".to_string(), *v);
        }
    }
    let artifacts_summary = ArtifactsSummary {
        total: total_artifacts.to_string(),
        summary: art_summ_map,
    };

    let status = Status {
        artifact_count: artifacts_summary,
        peers_count: peers.len(),
        peer_id: p2p_client.local_peer_id.to_string(),
        disk_allocated: cli_config.unwrap().disk_allocated,
        disk_usage: format!("{:.4}", disk_space_result.unwrap()),
    };

    let status_as_json = serde_json::to_string(&status).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(status_as_json)
        .unwrap())
}
