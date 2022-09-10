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

pub type TransactionCallback = dyn FnOnce(Transaction) + Send + Sync;

/// Define Supported Signature Algorithm
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SignatureAlgorithm {
    Ed25519,
}

pub struct Blockchain {
    // trans_observers may be only used internally by blockchain service
    trans_observers: HashMap<Transaction, Box<TransactionCallback>>,
    // chain is the blocks of the blockchain
    chain: Chain,
}

impl Debug for Blockchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("Blockchain")
            .field("chain", &self.chain)
            .field("trans_observers", &self.trans_observers.len())
            .finish()
    }
}

impl Blockchain {
    pub fn new(keypair: &identity::ed25519::Keypair) -> Self {
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            "this is the first reserved transaction".as_bytes().to_vec(),
            keypair,
        );
        // Make the "genesis" blocks
        let block = Block::new(HashDigest::new(b""), 0, Vec::from([transaction]), keypair);
        let mut chain: Chain = Default::default();
        chain.add_block(block);
        Self {
            trans_observers: Default::default(),
            chain,
        }
    }

    pub fn submit_transaction<CallBack: 'static + FnOnce(Transaction) + Send + Sync>(
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

    /// Add block after receiving payload and keypair
    pub async fn add_block(
        &mut self,
        payload: Vec<u8>,
        local_key: &identity::Keypair,
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
            ed25519_key,
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
            ed25519_key,
        );

        // TODO: Consensus algorithm will be refactored
        self.commit_block(block.clone()).await;

        Ok(())
    }

    /// Update block after receiving the new block from other peers
    pub async fn update_block_from_peers(&mut self, block: Box<Block>) {
        self.commit_block(*block).await;
    }

    /// Commit block and notify block listeners
    async fn commit_block(&mut self, block: Block) {
        self.chain.add_block(block);
    }

    pub fn last_block(&self) -> Option<Block> {
        self.chain.last_block()
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use std::sync::{Arc, Mutex};

    use super::*;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_build_blockchain() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let mut blockchain = Blockchain::new(&keypair);

        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            data.as_bytes().to_vec(),
            &keypair,
        );
        transactions.push(transaction);
        assert_eq!(1, blockchain.chain.len());
        blockchain
            .commit_block(Block::new(
                blockchain.chain.blocks()[0].header.hash(),
                blockchain.chain.blocks()[0].header.ordinal + 1,
                transactions,
                &keypair,
            ))
            .await;
        assert_eq!(true, blockchain.chain.blocks().last().unwrap().verify());
        assert_eq!(2, blockchain.chain.len());
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
        let called = Arc::new(Mutex::new(false));
        chain
            .submit_transaction(transaction.clone(), {
                let called = called.clone();
                let transaction = transaction.clone();
                move |t: Transaction| {
                    assert_eq!(transaction, t);
                    *called.lock().unwrap() = true;
                }
            })
            .notify_transaction_settled(transaction);
        assert!(*called.lock().unwrap());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_last_block() -> Result<(), String> {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let mut blockchain = Blockchain::new(&keypair);

        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            data.as_bytes().to_vec(),
            &keypair,
        );
        transactions.push(transaction);
        assert_eq!(1, blockchain.chain.len());
        blockchain
            .commit_block(Block::new(
                blockchain.chain.blocks()[0].header.hash(),
                blockchain.chain.blocks()[0].header.ordinal + 1,
                transactions,
                &keypair,
            ))
            .await;
        assert_ne!(None, blockchain.chain.last_block());
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_block() -> Result<(), String> {
        let keypair = identity::Keypair::generate_ed25519();
        let ed25519_key = match keypair.clone() {
            Ed25519(some) => some,
            _ => return Err("Key format is wrong".to_string()),
        };

        let mut blockchain = Blockchain::new(&ed25519_key);

        let data = "Hello First Transaction";

        let result = blockchain
            .add_block(data.as_bytes().to_vec(), &keypair)
            .await;
        assert_eq!(result.is_ok(), true);
        assert_eq!(
            b"Hello First Transaction".to_vec(),
            blockchain.chain.last_block().unwrap().transactions[0].payload()
        );
        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_block_from_peer() -> Result<(), String> {
        let keypair = identity::Keypair::generate_ed25519();
        let ed25519_key = match keypair.clone() {
            Ed25519(some) => some,
            _ => return Err("Key format is wrong".to_string()),
        };

        let mut blockchain = Blockchain::new(&ed25519_key);

        let block = Box::new(Block::new(HashDigest::new(b""), 1, vec![], &ed25519_key));

        let result = blockchain.update_block_from_peers(block).await;
        assert_eq!(result, ());

        Ok(())
    }
}
