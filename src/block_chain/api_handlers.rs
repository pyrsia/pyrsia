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




use std::sync::Arc;
use tokio::sync::mpsc::{Receiver, Sender};
use tokio::sync::Mutex;



// Types of API requests
struct GetAllBlocks {}
struct GetBlockWithId {
    id: u32,
}
struct PutNewData {
    data: String,
}

// enum ApiMessage { Request(GetAllBlocks, GetAllBlocks, PutNewData), Answer([]Block, Block) }

// Just one Channel
struct MessageHandler<Request, Answer> {
    tx: Sender<Request>,
    rx: Arc<Mutex<Receiver<Answer>>>,
}

// 1. GET /blocks (processing) /// Async Block 1
// 2. PUT /block
// 3. Answer PUT               /// Wake Up "all" receivers
// 4. GET "reads" PUT answer   /// Both end up

//---------
// One channel may have racing conditions ???????

// impl<Request, Answer> MessageHandler<Request, Answer> {
//     pub fn new() -> Self {
//         let (tx, mut rx) = mpsc::channel(32);
//         MessageHandler{tx, rx: Arc::new(Mutex::new(rx))}
//     }
// }

// impl ApiOutliningMethodsRequired{
//     // Under the hood we use the channel to send and receive
//     async fn get_list_of_blocks() -> [Block];
//     async fn get_block_by_id(id: u32) -> Block;
//     async fn add_new_block(data: String) -> Block; // mk_block and commit
// }

// pub async fn handle_get_blocks(
//     blocks: MessageHandler<GetAllBlocks, BlockChain>
// ) -> Result<impl Reply, Rejection> {
//     let chain = blocks.get_list_of_blocks().await;

//     info!("Got receive_blocks: {}", chain);

//     // format the response
//     Ok(warp::http::response::Builder::new()
//         .header("Content-Type", "application/json")
//         .status(StatusCode::OK)
//         .body("figure out blocks to string")
//         .unwrap())
// }
