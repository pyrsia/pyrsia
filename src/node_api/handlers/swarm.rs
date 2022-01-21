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
use crate::block_chain::block_chain::BlockChain;
use crate::node_manager::{handlers::get_arts_count, model::cli::Status};

use log::{debug, error, info};
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

    let status = Status {
        artifact_count: art_count_result.unwrap(),
        peers_count: peers_total,
        // TODO: placeholder disk space value, need implementation in upstream
        disk_space_available: String::from("983112"),
    };

    let ser_status = serde_json::to_string(&status).unwrap();

    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(ser_status)
        .unwrap())
}

// TODO Move to block chain module
pub async fn handle_get_blocks(
    tx: Sender<String>,
    rx: Arc<Mutex<Receiver<BlockChain>>>,
) -> Result<impl Reply, Rejection> {
    // Send "digested" request data to main
    match tx.send(String::from("blocks")).await {
        Ok(_) => debug!("request for peers sent"),
        Err(_) => error!("failed to send stdin input"),
    }

    // get result from main ( where the block chain lives )
    let block_chain = rx.lock().await.recv().await.unwrap();
    let blocks = format!("{}", block_chain);
    info!("Got receive_blocks: {}", blocks);

    // format the response
    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/json")
        .status(StatusCode::OK)
        .body(blocks)
        .unwrap())
}

// Next Step:
// handle_get_block_id
// hand_put_block
