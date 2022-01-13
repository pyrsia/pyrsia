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

// this is to handle calls from cli that needs access info swarm specific from  kad dht
use super::super::*;
use super::RegistryError;
use super::RegistryErrorCode;
use log::{debug, error};
use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;
use warp::http::StatusCode;
use warp::{Rejection, Reply};

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
    let split = peers.split(',');
    let vec: Vec<&str> = split.collect();
    debug!("peers count: {}", vec.len());

    let art_count_result = get_arts_count();
    if art_count_result.is_err() {
        return Err(warp::reject::custom(RegistryError {
            code: RegistryErrorCode::Unknown(art_count_result.err().unwrap().to_string()),
        }));
    }

    let status = Status {
        artifact_count: art_count_result.unwrap(),
        peers_count: vec.len(),
        // TODO: sample disk space value, need implementation in upstream
        disk_space_available: String::from("983112"),
    };

    let ser_status = serde_json::to_string(&status).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(ser_status)
        .unwrap())
}
