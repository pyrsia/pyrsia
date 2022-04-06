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

use libp2p::{identity, PeerId};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};

use super::crypto::hash_algorithm::HashDigest;
use super::structures::{
    block::Block,
    transaction::{Transaction, TransactionType},
};

/// Define Supported Signature Algorithm
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SignatureAlgorithm {
    Ed25519,
}

#[derive(Deserialize, Serialize)]
pub struct Blockchain {
    #[serde(skip)]
    // this should actually be a Map<Transaction,Vec<OnTransactionSettled>> but that's later
    trans_observers: HashMap<Transaction, Box<dyn FnOnce(Transaction)>>,
    #[serde(skip)]
    block_observers: Vec<Box<dyn FnMut(Block)>>,
    blocks: Vec<Block>,
}

impl Debug for Blockchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Blockchain")
            .field("blocks", &self.blocks)
            .field("trans_observers", &self.trans_observers.len())
            .field("block_observers", &self.block_observers.len())
            .finish()
    }
}

impl Blockchain {
    pub fn new(keypair: &identity::ed25519::Keypair) -> Self {
        let local_id = PeerId::from(identity::PublicKey::Ed25519(keypair.public()));
        let genesis_pub_key: [u8; 32] = [
            0x7F,0x8A,0xC6,0x5B,0x00,0xD5,0xCD,0x7B,0x3B,0xCA,0x16,0x5B,0xFD,0x6B,0x7F,0xB2,
            0x5A,0x6A,0x25,0x11,0x01,0xD2,0x65,0x0E,0xB6,0x97,0xC7,0xBC,0x56,0x84,0x28,0x9A
        ];
        let transaction = Transaction::new(
            TransactionType::AddAuthority,
            local_id,
            genesis_pub_key.to_vec(),
            keypair,
        );
        // Make the "genesis" blocks
        let block = Block::new(HashDigest::new(b""), 0, Vec::from([transaction]), keypair);
        Self {
            trans_observers: Default::default(),
            block_observers: vec![],
            blocks: Vec::from([block]),
        }
    }

    pub fn submit_transaction<CallBack: 'static + FnOnce(Transaction)>(
        &mut self,
        trans: Transaction,
        on_done: CallBack,
    ) -> &mut Self {
        self.trans_observers.insert(trans, Box::new(on_done));
        self
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
        self.blocks.push(block);
        self.notify_block_event(self.blocks.last().expect("block must exist").clone());
    }
}

impl Display for Blockchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let json = serde_json::to_string_pretty(&self).expect("json format error");
        write!(f, "{}", json)
    }
}

#[cfg(test)]
mod tests {
    use std::cell::Cell;
    use std::rc::Rc;

    use super::*;

    #[test]
    fn test_build_blockchain() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = PeerId::from(identity::PublicKey::Ed25519(keypair.public()));
        let mut chain = Blockchain::new(&keypair);

        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            data.as_bytes().to_vec(),
            &keypair,
        );
        transactions.push(transaction);
        chain.add_block(Block::new(
            chain.blocks[0].header.hash(),
            chain.blocks[0].header.ordinal,
            transactions,
            &keypair,
        ));
        assert_eq!(true, chain.blocks.last().unwrap().verify());
        assert_eq!(2, chain.blocks.len());
        Ok(())
    }

    #[test]
    fn test_add_trans_listener() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = PeerId::from(identity::PublicKey::Ed25519(keypair.public()));
        let mut chain = Blockchain::new(&keypair);

        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            "some transaction".as_bytes().to_vec(),
            &keypair,
        );
        let called = Rc::new(Cell::new(false));
        chain
            .submit_transaction(transaction.clone(), {
                let called = called.clone();
                let transaction = transaction.clone();
                move |t: Transaction| {
                    assert_eq!(transaction, t);
                    called.set(true)
                }
            })
            .notify_transaction_settled(transaction);
        assert!(called.get());
        Ok(())
    }

    #[test]
    fn test_add_block_listener() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
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
}
