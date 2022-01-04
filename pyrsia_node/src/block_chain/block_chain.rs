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
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::anyhow;
use log::{error, info};
use serde_json::error::Error;
use sha2::{Digest, Sha256};

use super::block::Block;

#[derive(Debug, Clone)]
pub struct BlockChain {
    blocks: Vec<Block>,
}

const DIFFICULTY_PREFIX: &str = "00";

impl BlockChain {
    pub fn new() -> Self {
        BlockChain {
            blocks: Vec::from([Block {
                id: 0,
                timestamp: now(),
                previous_hash: String::from("genesis"),
                data: String::from("genesis!"),
                nonce: 2836,
                hash: "0000f816a87f806bb0073dcf026a64fb40c946b5abee2573702828694d5b4c43"
                    .to_string(),
            }]),
        }
    }

    pub fn dump(&self) -> Result<String, Error> {
        serde_json::to_string_pretty(&self.blocks).map(|pretty_json| {
            info!("{}", pretty_json);
            pretty_json
        })
    }

    pub fn mk_block(&mut self, data: String) -> Option<Block> {
        let now = now();
        self.blocks
            .last()
            .map(|last_block| {
                let (nonce, hash) = mine_block(last_block.id, now, &last_block.hash, &data);
                (nonce, hash, last_block)
            })
            .map(|(nonce, hash, last_block)| Block {
                id: last_block.id.clone() + 1,
                hash,
                previous_hash: last_block.hash.clone(),
                timestamp: now,
                data,
                nonce,
            })
    }

    fn are_blocks_sequential(previous_block: &Block, next: Block) -> Result<Block, anyhow::Error> {
        if next.previous_hash != previous_block.hash {
            return Err(anyhow!(
                "block with id: {} has wrong previous hash",
                next.id
            ));
        }
        if !hash_to_binary_representation(&hex::decode(&next.hash).expect("can decode from hex"))
            .starts_with(DIFFICULTY_PREFIX)
        {
            return Err(anyhow!("block with id: {} has invalid difficulty", next.id));
        }
        if next.id != previous_block.id + 1 {
            return Err(anyhow!(
                "block with id: {} is not the next block after the latest: {}",
                next.id,
                previous_block.id
            ));
        }
        if hex::encode(calculate_hash(
            next.id,
            next.timestamp,
            &next.previous_hash,
            &next.data,
            next.nonce,
        )) != next.hash
        {
            return Err(anyhow!("block with id: {} has invalid hash", next.id));
        }
        Ok(next)
    }

    fn is_chain_valid(&self, chain: &[Block]) -> bool {
        for i in 1..chain.len() {
            let first = chain.get(i - 1).expect("has to exist");
            let second = chain.get(i).expect("has to exist");
            match BlockChain::are_blocks_sequential(first, second.clone()) {
                Err(e) => {
                    return false;
                }
                Ok(_) => {
                    continue;
                }
            };
        }
        true
    }
}

fn now() -> u128 {
    match SystemTime::now().duration_since(UNIX_EPOCH) {
        Ok(n) => n.as_millis(),
        Err(_) => panic!("SystemTime before UNIX EPOCH!"),
    }
}

fn mine_block(id: u64, timestamp: u128, previous_hash: &str, data: &str) -> (u64, String) {
    info!("mining block...");

    (0..u64::MAX)
        .map(|nonce| {
            (
                nonce,
                calculate_hash(id, timestamp, previous_hash, data, nonce),
            )
        })
        .map(|(nonce, hash)| {
            (
                nonce,
                hash.clone(),
                hash_to_binary_representation(&hash.clone()),
            )
        })
        .find(|(_nonce, _hash, binary_hash)| binary_hash.starts_with(DIFFICULTY_PREFIX))
        .map(|(nonce, hash, _bin)| (nonce, hex::encode(hash)))
        .expect("results")
}

fn calculate_hash(
    id: u64,
    timestamp: u128,
    previous_hash: &str,
    data: &str,
    nonce: u64,
) -> Vec<u8> {
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
    hash.iter()
        .map(|c| format!("{:b}", c))
        .fold("".to_string(), |cur, nxt| format!("{}{}", cur, nxt))
}

impl Ledger for BlockChain {
    fn add_entry(mut self, block: Block) -> Result<BlockChain, anyhow::Error> {
        // let block = self.mk_block( data.clone()).expect("error creating block");
        let last_block = self.blocks.last().expect("has a block");
        match BlockChain::are_blocks_sequential(last_block, block) {
            Ok(b) => {
                self.blocks.push(b);
                Ok(self)
            }
            Err(e) => {
                error!("{}", e);
                Err(e)
            }
        }
    }

    fn is_valid(self) -> Result<bool, anyhow::Error> {
        todo!()
    }
}

impl Iterator for BlockChain {
    type Item = ();

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

pub trait Ledger {
    fn add_entry(/*mut*/ self, entry: Block) -> Result<BlockChain, anyhow::Error>;

    fn is_valid(self) -> Result<bool, anyhow::Error>;
}

// trait Observer<T> {
//     fn receive_event(event_name: &str, thing: T);
// }
//
// trait Observable {
//     fn add_observer(observer: dyn Observer<T>) -> dyn Observable;
// }

// Ledger::validate(ledger)
// ledger.is_validate()
