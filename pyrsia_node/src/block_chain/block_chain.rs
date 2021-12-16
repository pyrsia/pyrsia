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


use chrono::Utc;
use log::{info, warn};
use serde_json::error::Error;
use sha2::{Digest, Sha256};

use crate::block_chain::block::Block;

pub struct BlockChain {
    blocks: Vec<Block>,
}


const DIFFICULTY_PREFIX: &str = "00";

impl BlockChain {
    pub fn new() -> Self {
        BlockChain {
            blocks: vec![]
        }
    }
    pub fn genesis(&mut self) -> &mut Self {
        let genesis_block = Block {
            id: 0,
            timestamp: Utc::now().timestamp(),
            previous_hash: String::from("genesis"),
            data: String::from("genesis!"),
            nonce: 2836,
            hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43".to_string(),
        };
        self.blocks.push(genesis_block);
        self
    }
    pub(crate) fn dump(&self) -> Result<String, Error> {
        serde_json::to_string_pretty(&self.blocks).map(|pretty_json| {
            println!("{}", pretty_json);
            pretty_json
        })
    }

    pub fn add_block(&mut self, block: Block) -> Option<&Block> {
        // let block = self.mk_block( data.clone()).expect("error creating block");
        let last_block = self.blocks.last().expect("has a block");
        if self.is_block_valid(&block, last_block) {
            self.blocks.push(block);
            return self.blocks.last();
        }
        None
    }
    pub fn mk_block(&mut self, data: String) -> Option<Block> {
        let now = Utc::now();
        self.blocks.last()
            .map(|last_block| {
                let (nonce, hash) = mine_block(last_block.id, now.timestamp(), &last_block.hash, &data);
                (nonce, hash, last_block)
            })
            .map(|(nonce, hash, last_block)| Block {
                id: last_block.id.clone() + 1,
                hash,
                previous_hash: last_block.hash.clone(),
                timestamp: now.timestamp(),
                data,
                nonce,
            })
    }

    fn is_block_valid(&self, block: &Block, previous_block: &Block) -> bool {
        if block.previous_hash != previous_block.hash {
            warn!("block with id: {} has wrong previous hash", block.id);
            return false;
        }
        if !hash_to_binary_representation(&hex::decode(&block.hash).expect("can decode from hex")).starts_with(DIFFICULTY_PREFIX)
        {
            warn!("block with id: {} has invalid difficulty", block.id);
            return false;
        }
        if block.id != previous_block.id + 1 {
            warn!(
                "block with id: {} is not the next block after the latest: {}",
                block.id, previous_block.id
            );
            return false;
        }
        if hex::encode(calculate_hash(
            block.id,
            block.timestamp,
            &block.previous_hash,
            &block.data,
            block.nonce,
        )) != block.hash
        {
            warn!("block with id: {} has invalid hash", block.id);
            return false;
        }
        true
    }

    fn is_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 1..chain.len() {
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            if !self.is_block_valid(second, first) {
                return false;
            }
        }
        true
    }
}


fn mine_block(id: u64, timestamp: i64, previous_hash: &str, data: &str) -> (u64, String) {
    info!("mining block...");

    (0..u64::MAX).map(|nonce| (nonce, calculate_hash(id, timestamp, previous_hash, data, nonce)))
        .map(|(nonce, hash)| (nonce, hash.clone(), hash_to_binary_representation(&hash.clone())))
        .find(|(nonce, hash, binary_hash)| binary_hash.starts_with(DIFFICULTY_PREFIX))
        .map(|(nonce, hash, bin)| (nonce, hex::encode(hash))).expect("results")
}

fn calculate_hash(id: u64, timestamp: i64, previous_hash: &str, data: &str, nonce: u64) -> Vec<u8> {
    let data = serde_json::json!({
        "id": id,
        "previous_hash": previous_hash,
        "data": data,
        "timestamp": timestamp,
        "nonce": nonce
    });
    let mut hasher = Sha256::new();
    hasher.update(data.to_string().as_bytes());
    hasher.finalize().as_slice().to_owned()
}

fn hash_to_binary_representation(hash: &[u8]) -> String {
    hash.iter().map(|c| format!("{:b}", c)).fold("".to_string(), |cur, nxt| cur + &nxt)
}


