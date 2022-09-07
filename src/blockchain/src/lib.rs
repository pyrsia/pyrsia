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

pub mod args;
pub mod blockchain;
pub mod crypto;
pub mod error;
pub mod identities;
pub mod network;
pub mod providers;
pub mod signature;
pub mod structures;

use crate::network::NetworkData;
use crate::providers::DataStore;
use crate::structures::block::Block;
use crate::structures::header::Ordinal;

pub use aleph_bft::{default_config, run_session, NodeIndex};
use futures::{
    channel::{
        mpsc::{UnboundedReceiver, UnboundedSender},
        oneshot,
    },
    FutureExt, StreamExt,
};
use futures_timer::Delay;
use log::{debug, info, trace};
use std::sync::Mutex;
use std::{
    sync::Arc,
    time::{self, Duration},
};

pub type BlockPlan = Arc<dyn Fn(Ordinal) -> NodeIndex + Sync + Send + 'static>;

pub struct ChainConfig {
    // Our NodeIndex.
    pub node_ix: NodeIndex,
    // Number of random bytes to include in the block.
    pub data_size: usize,
    // Delay between blocks
    pub blocktime_ms: u128,
    // Delay before the first block should be created
    pub init_delay_ms: u128,
    // f(k) means who should author the kth block
    pub authorship_plan: BlockPlan,
}

pub fn gen_chain_config(
    node_ix: NodeIndex,
    n_members: usize,
    data_size: usize,
    blocktime_ms: u128,
    init_delay_ms: u128,
) -> ChainConfig {
    //Round robin block authorship plan.
    // let authorship_plan = Arc::new(move |num: u64| NodeIndex(((num as usize) % n_members) + 1));
    let authorship_plan = Arc::new(move |num: u128| NodeIndex((num as usize) % n_members));
    ChainConfig {
        node_ix,
        data_size,
        blocktime_ms,
        init_delay_ms,
        authorship_plan,
    }
}

// Runs a process that maintains a simple blockchain. The blocks are created every config.blocktime_ms
// milliseconds and the block authors are determined by config.authorship_plan. The default config
// uses round robin authorship: node k creates blocks number n if n%n_members = k.
// A node will create a block n only if:
// 1) it received the previous block (n-1)
// 2) it is the nth block author
// 3) enough time has passed -- to maintain blocktime of roughly config.blocktime_ms milliseconds.
// This process holds two channel endpoints: block_rx to receive blocks from the network and
// block_tx to push created blocks to the network (to send them to all the remaining nodes).
pub async fn run_blockchain(
    config: ChainConfig,
    mut data_store: DataStore,
    current_block: Arc<Mutex<Block>>,
    mut blocks_from_network: UnboundedReceiver<Block>,
    _blocks_for_network: UnboundedSender<Block>,
    mut messages_from_network: UnboundedReceiver<NetworkData>,
    mut exit: oneshot::Receiver<()>,
) {
    let start_time = time::Instant::now();
    for block_num in 1u128.. {
        while current_block.lock().unwrap().header.ordinal < block_num {
            let curr_author = (config.authorship_plan)(block_num);
            trace!("The current block author is {:?}", curr_author);
            if curr_author == config.node_ix {
                // We need to create the block, but at the right time
                info!(
                    "🔔 It's my turn to create a new block -- block_num {}",
                    block_num
                );
                let curr_time = time::Instant::now();
                //TODO(prince-chrismc): This wants to be u64 so we need to do some magic here
                let block_delay_ms = (block_num - 1) * config.blocktime_ms + config.init_delay_ms;
                let block_creation_time =
                    start_time + Duration::from_millis(block_delay_ms.try_into().unwrap());
                if curr_time >= block_creation_time {
                    // TODO(prince-chrismc): Figure out how to generate new blocks and push them on the network
                    // let block = Block::new(block_num, config.data_size);
                    // blocks_for_network
                    //     .unbounded_send(block)
                    //     .expect("network should accept blocks");
                    info!("📝 Saving locally generated block");
                    // TODO(prince-chrismc): Generate blocks from "known transactions"
                    // data_store.add_block(block_num);
                }
            }
            // We tick every 125ms.
            let mut delay_fut = Delay::new(Duration::from_millis(125)).fuse();

            futures::select! {
                maybe_block = blocks_from_network.next() => {
                    if let Some(block) = maybe_block {
                        info!("🧾 Adding new block {} from the network", block.header.ordinal);
                        data_store.add_block(block);
                        //We drop the block at this point, only keep track of the fact that we received it.
                    }
                }
                maybe_message = messages_from_network.next() => {
                    if let Some(message) = maybe_message {
                        trace!("recording new message from network");
                        data_store.add_message(message);
                    }
                }
                _ = &mut delay_fut => {
                    //We do nothing, but this takes us out of the select.
                }
                _ = &mut exit => {
                    debug!("Received exit signal.");
                    return;
                },
            }
        }
    }
}
