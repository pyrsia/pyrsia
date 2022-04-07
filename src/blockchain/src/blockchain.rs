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

use identity::ed25519::Keypair;
use identity::PublicKey::Ed25519;
use libp2p::{identity, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use super::crypto::hash_algorithm::HashDigest;
use crate::structures::block::*;
use crate::structures::chain::*;
use crate::structures::transaction::*;

/// BlockchainId identifies the current chain
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BlockchainId {
    Pyrsia,
}

/// Define Supported Signature Algorithm
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SignatureAlgorithm {
    Ed25519,
}

pub struct Blockchain {
    // this should actually be a Map<Transaction,Vec<OnTransactionSettled>> but that's later
    trans_observers: HashMap<Transaction, Box<dyn FnOnce(Transaction)>>,
    key_pair: Keypair,
    local_id: PeerId,
    block_observers: Vec<Box<dyn FnMut(Block)>>,
    chain: Chain,
}

impl Debug for Blockchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Blockchain")
            .field("chain", &self.chain)
            .field("trans_observers", &self.trans_observers.len())
            .field("block_observers", &self.block_observers.len())
            .finish()
    }
}

impl Blockchain {
    pub fn new(keypair: &Keypair) -> Self {
        let local_id = PeerId::from(Ed25519(keypair.public()));
        let genesis_pub_key: [u8; 44] = [
            0x30, 0x2a, 0x30, 0x05, 0x06, 0x03, 0x2b, 0x65, 0x70, 0x03, 0x21, 0x00, 0xed, 0xbf,
            0x0f, 0xc3, 0xea, 0x90, 0x29, 0x1e, 0x03, 0x0e, 0xa9, 0x5c, 0x3d, 0x96, 0x17, 0xc3,
            0x47, 0x05, 0x6f, 0xa3, 0x12, 0x60, 0x89, 0xa3, 0x96, 0x07, 0x91, 0xc6, 0x01, 0xbf,
            0x9a, 0x72,
        ];
        let transaction = Transaction::new(
            TransactionType::GrantAuthority,
            local_id,
            genesis_pub_key.to_vec(),
            keypair,
        );
        // Make the "genesis" blocks
        let block = Block::new(HashDigest::new(b""), 0, Vec::from([transaction]), keypair);
        let mut chain: Chain = Default::default();
        chain.blocks.push(block);
        Self {
            trans_observers: Default::default(),
            key_pair: keypair.clone(),
            local_id: local_id.clone(),
            block_observers: vec![],
            chain,
        }
    }

    pub fn blocks(&self) -> Vec<Block> {
        self.chain.blocks.clone()
    }

    pub fn submit_transaction<CallBack: 'static + FnOnce(Transaction)>(
        &mut self,
        trans_type: TransactionType,
        data: Vec<u8>,
        on_done: CallBack,
    ) -> Result<(), String> {
        let trans = Transaction::new(trans_type, self.local_id.clone(), data, &self.key_pair);
        match trans.is_valid() {
            Ok(()) => {
                let t = trans.clone();
                self.trans_observers.insert(trans, Box::new(on_done));
                // it doesn't actually settle at this point. Aleph will do that
                self.notify_transaction_settled(t);
                Ok(())
            }
            Err(e) => {
                println!("{}", e);
                Err(e)
            }
        }
    }

    pub fn notify_transaction_settled(&mut self, trans: Transaction) {
        // if there were no observers, we don't care
        if let Some(on_settled) = self.trans_observers.remove(&trans) {
            on_settled(trans)
        }
    }

    pub fn add_block_listener<CallBack: 'static + FnMut(Block)>(
        &mut self,
        on_block: CallBack,
    ) -> &mut Self {
        self.block_observers.push(Box::new(on_block));
        self
    }

    pub fn notify_block_event(&mut self, block: Block) -> &mut Self {
        self.block_observers
            .iter_mut()
            .for_each(|notify| notify(block.clone()));
        self
    }

    #[warn(dead_code)]
    pub fn add_block(&mut self, block: Block) {
        self.chain.blocks.push(block);
        self.notify_block_event(self.chain.blocks.last().expect("block must exist").clone());
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;
    use TransactionType::RevokeAuthority;
    use crate::structures::transaction::TransactionType::GrantAuthority;

    use super::*;

    #[test]
    fn test_build_blockchain() -> Result<(), String> {
        let keypair = Keypair::generate();
        let local_id = PeerId::from(Ed25519(keypair.public()));
        let mut chain = Blockchain::new(&keypair);

        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            TransactionType::AddArtifact,
            local_id,
            data.as_bytes().to_vec(),
            &keypair,
        );
        transactions.push(transaction);
        chain.add_block(Block::new(
            chain.blocks()[0].header.hash(),
            chain.blocks()[0].header.ordinal,
            transactions,
            &keypair,
        ));
        assert_eq!(true, chain.blocks().last().unwrap().verify());
        assert_eq!(2, chain.blocks().len());
        Ok(())
    }

    #[test]
    fn test_add_trans_listener() -> Result<(), String> {
        let keypair = Keypair::generate();
        let mut chain = Blockchain::new(&keypair);

        let data = "some transaction";
        let called = Rc::new(Cell::new(false));
        chain
            .submit_transaction(TransactionType::AddArtifact, data.as_bytes().to_vec(), {
                let called = called.clone();
                move |t: Transaction| {
                    assert_eq!(data.as_bytes().to_vec(), t.payload());
                    called.set(true)
                }
            })?;
        assert!(called.get());
        Ok(())
    }

    #[test]
    fn test_add_block_listener() -> Result<(), String> {
        let keypair = Keypair::generate();
        let block = Block::new(
            HashDigest::new(b"Hello World!"),
            1u128,
            Vec::new(),
            &keypair,
        );
        let mut chain = Blockchain::new(&keypair);
        let called = Rc::new(Cell::new(false));

        chain
            .add_block_listener({
                let called = called.clone();
                let block = block.clone();
                move |b: Block| {
                    assert_eq!(block, b);
                    called.set(true);
                }
            })
            .add_block(block);

        assert!(called.get()); // called is still false
        Ok(())
    }

    #[test]
    fn test_revoke_authority() -> Result<(), String> {
        let keypair = Keypair::generate();
        let mut chain = Blockchain::new(&keypair);
        let raw_pub_key: [u8;32] = [
            0x0F,0x30,0x2A,0xAC,0x9E,0x34,0xC8,0xF0,0x90,0x75,0x08,0xB1,0x15,0x2E,0xEA,0xFC,
            0x69,0x67,0x90,0x22,0x27,0x84,0x0D,0x4C,0x32,0xB6,0xED,0xF5,0xF0,0x7A,0xFC,0x87
        ];
        chain.submit_transaction(RevokeAuthority, raw_pub_key.to_vec(), |_|{})
    }
    #[test]
    fn test_revoke_authority_bad_key(){
        let keypair = Keypair::generate();
        let mut chain = Blockchain::new(&keypair);
        let raw_pub_key: [u8;1] = [0x00];
        let ok = match chain.submit_transaction(RevokeAuthority, raw_pub_key.to_vec(), |_|{}){
            Ok(_) => false,
            Err(_) => true
        };
        assert!(ok)
    }

    #[test]
    fn test_add_authority() -> Result<(), String> {
        let keypair = Keypair::generate();
        let mut chain = Blockchain::new(&keypair);
        let raw_pub_key: [u8;32] = [
            0x0F,0x30,0x2A,0xAC,0x9E,0x34,0xC8,0xF0,0x90,0x75,0x08,0xB1,0x15,0x2E,0xEA,0xFC,
            0x69,0x67,0x90,0x22,0x27,0x84,0x0D,0x4C,0x32,0xB6,0xED,0xF5,0xF0,0x7A,0xFC,0x87
        ];
        chain.submit_transaction(GrantAuthority, raw_pub_key.to_vec(), |_|{})
    }
    #[test]
    fn test_add_authority_bad_key() {
        let keypair = Keypair::generate();
        let mut chain = Blockchain::new(&keypair);
        let raw_pub_key: [u8;1] = [0x00];
        let ok = match chain.submit_transaction(RevokeAuthority, raw_pub_key.to_vec(), |_|{}){
            Ok(_) => false,
            Err(_) => true
        };
        assert!(ok)
    }
}
