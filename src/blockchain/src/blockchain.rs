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

use libp2p::identity;
use libp2p::identity::Keypair::Ed25519;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Debug, Formatter};

use super::crypto::hash_algorithm::HashDigest;
use super::structures::{
    block::Block,
    chain::Chain,
    header::Address,
    transaction::{Transaction, TransactionType},
};

/// Define Supported Signature Algorithm
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SignatureAlgorithm {
    Ed25519,
}

pub struct Blockchain {
    // this should actually be a Map<Transaction,Vec<OnTransactionSettled>> but that's later
    trans_observers: HashMap<Transaction, Box<dyn FnOnce(Transaction)>>,
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
    pub fn new(keypair: &identity::ed25519::Keypair) -> Self {
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            "this needs to be the root authority".as_bytes().to_vec(),
            keypair,
        );
        // Make the "genesis" blocks
        let block = Block::new(HashDigest::new(b""), 0, Vec::from([transaction]), keypair);
        let mut chain: Chain = Default::default();
        chain.blocks.push(block);
        Self {
            trans_observers: Default::default(),
            block_observers: vec![],
            chain,
        }
    }

    pub fn blocks(&self) -> Vec<Block> {
        self.chain.blocks.clone()
    }

    pub fn last_block(&self) -> Option<Block> {
        let length = self.chain.blocks.len();
        if length == 0 {
            None
        } else {
            Some(self.chain.blocks[length - 1].clone())
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

    /// Add block after receiving payload and keypair
    pub fn add_block(
        &mut self,
        payload: Vec<u8>,
        local_key: identity::Keypair,
    ) -> anyhow::Result<()> {
        let submitter = Address::from(local_key.public());
        let ed25519_key = match local_key {
            Ed25519(some) => some,
            _ => {
                anyhow::bail!(
                    "Blockchain: Key {:?} is not valid Ed25519 format",
                    local_key
                );
            }
        };
        let trans_vec = vec![Transaction::new(
            TransactionType::Create,
            submitter,
            payload,
            &ed25519_key,
        )];

        let last_block = match self.last_block() {
            Some(block) => block,
            None => {
                anyhow::bail!("Blockchain: Local blockchain does non exist!!");
            }
        };

        let block = Block::new(
            last_block.header.hash(),
            last_block.header.ordinal + 1,
            trans_vec,
            &ed25519_key,
        );

        // TODO: Consensus algorithm will be refactored
        self.commit_block(block);
        Ok(())
    }

    /// Commit block and notify block listeners
    fn commit_block(&mut self, block: Block) {
        self.chain.blocks.push(block);
        self.notify_block_event(self.chain.blocks.last().expect("block must exist").clone());
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
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
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
        chain.commit_block(Block::new(
            chain.blocks()[0].header.hash(),
            chain.blocks()[0].header.ordinal + 1,
            transactions,
            &keypair,
        ));
        assert_eq!(true, chain.blocks().last().unwrap().verify());
        assert_eq!(2, chain.blocks().len());
        Ok(())
    }

    #[test]
    fn test_add_trans_listener() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
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
            .commit_block(block);

        assert!(called.get()); // called is still false
        Ok(())
    }

    #[test]
    fn test_block() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
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
        chain.commit_block(Block::new(
            chain.blocks()[0].header.hash(),
            chain.blocks()[0].header.ordinal + 1,
            transactions,
            &keypair,
        ));
        assert_eq!(2, chain.blocks().len());
        Ok(())
    }

    #[test]
    fn test_last_block() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
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
        chain.commit_block(Block::new(
            chain.blocks()[0].header.hash(),
            chain.blocks()[0].header.ordinal + 1,
            transactions,
            &keypair,
        ));
        assert_ne!(None, chain.last_block());
        Ok(())
    }

    #[test]
    fn test_add_block() -> Result<(), String> {
        let keypair = identity::Keypair::generate_ed25519();
        let ed25519_key = match keypair.clone() {
            Ed25519(some) => some,
            _ => return Err("Key format is wrong".to_string()),
        };

        let mut chain = Blockchain::new(&ed25519_key);

        let data = "Hello First Transaction";

        let result = chain.add_block(data.as_bytes().to_vec(), keypair);
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            b"Hello First Transaction".to_vec(),
            chain.last_block().unwrap().transactions[0].payload()
        );
        Ok(())
    }
}
