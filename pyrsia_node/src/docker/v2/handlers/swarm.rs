//use futures::channel::mpsc::Receiver;
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
    debug!("Commad executing : $$$");

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
