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
use log::{debug, info, error};
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
    info!("Got received_peers: {}", peers);
    Ok(warp::http::response::Builder::new()
        .header("Content-Type", "application/octet-stream")
        .status(StatusCode::OK)
        .body(peers)
        .unwrap())
}

// replace string with Block
pub async fn handle_get_blocks(
    tx: Sender<String>,
    mut rx: Receiver<String>, // TODO: Make this a struct that can be serialized to JSON
) -> Result<impl Reply, Rejection> {
    // Send "digested" request data to main
    match tx.send(String::from("blocks")).await {
        Ok(_) => debug!("request for peers sent"),
        Err(_) => error!("failed to send stdin input"),
    }

    // get result from main ( where the block chain lives )
    let blocks = rx.recv().await.unwrap();
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
// hand put block
