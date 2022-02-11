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
use crate::node_manager::{handlers::*, model::cli::Status};
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use warp::{http::StatusCode, Rejection, Reply};

pub async fn handle_get_peers(
    tx: Sender<String>,
    rx: Arc<Mutex<Receiver<String>>>,
) -> Result<impl Reply, Rejection> {
    match tx.send(String::from("peers")).await {
        Ok(_) => debug!("request for peers sent"),
        Err(_) => error!("failed to send stdin input"),
    }

    let peers = rx.lock().await.recv().await.unwrap();
    println!("Got received_peers: {}", peers);
    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(peers)
        .unwrap())
}

pub async fn handle_get_status(
    tx: Sender<String>,
    rx: Arc<Mutex<Receiver<String>>>,
) -> Result<impl Reply, Rejection> {
    match tx.send(String::from("peers")).await {
        Ok(_) => debug!("request for peers sent"),
        Err(_) => error!("failed to send stdin input"),
    }

    let peers = rx.lock().await.recv().await.unwrap();
    debug!("peers empty: {:?}", peers.is_empty());
    let mut peers_total = 0;
    if !peers.is_empty() {
        let res: Vec<String> = peers.split(',').map(|s| s.to_string()).collect();
        debug!("peers count: {}", res.len());
        peers_total = res.len();
    }

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
        peers_count: peers_total,
        disk_allocated: String::from(ALLOCATED_SPACE_FOR_ARTIFACTS),
        disk_usage: format!("{:.4}", disk_space_result.unwrap()),
    };

    let ser_status = serde_json::to_string(&status).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(ser_status)
        .unwrap())
}
