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
use std::path::{Path, PathBuf};

use crate::error::BlockchainError;
use crate::structures::header::Ordinal;

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
#[derive(Default)]
pub struct Blockchain {
    // trans_observers may be only used internally by blockchain service
    trans_observers: HashMap<Transaction, Box<TransactionCallback>>,
    // chain is the blocks of the blockchain
    chain: Chain,
    // the directory on the local file system to use for persisting the blocks in the blockchain
    blockchain_path: PathBuf,
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
    pub async fn new(
        keypair: &identity::ed25519::Keypair,
        blockchain_path: impl AsRef<Path>,
    ) -> Result<Self, BlockchainError> {
        let mut chain: Chain = Default::default();
        chain.load_blocks(&blockchain_path).await?;

        // Make the "genesis" block
        if chain.is_empty() {
            let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
            let transaction = Transaction::new(
                TransactionType::Create,
                local_id,
                "this is the first reserved transaction".as_bytes().to_vec(),
                keypair,
            );

            let block = Block::new(HashDigest::new(b""), 0, Vec::from([transaction]), keypair);
            Blockchain::save_block(&mut chain, block, &blockchain_path).await?
        }

        Ok(Self {
            trans_observers: Default::default(),
            chain,
            blockchain_path: blockchain_path.as_ref().to_path_buf(),
        })
    }

    pub fn empty_new(blockchain_path: impl AsRef<Path>) -> Self {
        Self {
            trans_observers: Default::default(),
            chain: Default::default(),
            blockchain_path: blockchain_path.as_ref().to_path_buf(),
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
    ) -> Result<(), BlockchainError> {
        let ed25519_key = match local_key {
            Ed25519(some) => some,
            _ => {
                return Err(BlockchainError::InvalidKey(format!("{:?}", local_key)));
            }
        };

        let submitter = Address::from(local_key.public());
        let trans_vec = vec![Transaction::new(
            TransactionType::Create,
            submitter,
            payload,
            ed25519_key,
        )];

        let last_block = match self.last_block() {
            Some(block) => block,
            None => {
                return Err(BlockchainError::EmptyBlockchain);
            }
        };

        let block = Block::new(
            last_block.header.hash(),
            last_block.header.ordinal + 1,
            trans_vec,
            ed25519_key,
        );

        // TODO: Consensus algorithm will be refactored
        self.commit_block(block.clone()).await
    }

    /// Update block after receiving the new block from other peers
    pub async fn update_block_from_peers(
        &mut self,
        block: Box<Block>,
    ) -> Result<(), BlockchainError> {
        self.commit_block(*block).await
    }

    /// Commit block and notify block listeners
    async fn commit_block(&mut self, block: Block) -> Result<(), BlockchainError> {
        Self::save_block(&mut self.chain, block, self.blockchain_path.as_path()).await
    }

    pub fn last_block(&self) -> Option<Block> {
        self.chain.last_block()
    }

    pub fn pull_blocks(&self, start: Ordinal, end: Ordinal) -> Result<Vec<Block>, BlockchainError> {
        Ok(self.chain.retrieve_blocks(start, end))
    }

    async fn save_block(
        chain: &mut Chain,
        block: Block,
        blockchain_path: impl AsRef<Path>,
    ) -> Result<(), BlockchainError> {
        let block_ordinal = block.header.ordinal;
        chain.add_block(block);
        chain
            .save_block(
                block_ordinal,
                blockchain_path
                    .as_ref()
                    .to_path_buf()
                    .join(format!("{}.ser", block_ordinal)),
            )
            .await
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::{Arc, Mutex};

    fn create_tmp_dir() -> PathBuf {
        tempfile::tempdir()
            .expect("could not create temporary directory")
            .into_path()
    }

    fn remove_tmp_dir(tmp_dir: PathBuf) {
        if tmp_dir.exists() {
            fs::remove_dir_all(&tmp_dir)
                .unwrap_or_else(|_| panic!("unable to remove test directory {:?}", tmp_dir));
        }
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_build_blockchain() {
        let tmp_dir = create_tmp_dir();
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let mut blockchain = Blockchain::new(&keypair, &tmp_dir)
            .await
            .expect("Blockchain should have been created.");

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
            .await
            .expect("Block should have been committed.");
        assert!(blockchain.chain.blocks().last().unwrap().verify());
        assert_eq!(2, blockchain.chain.len());

        remove_tmp_dir(tmp_dir);
    }

    #[tokio::test]
    async fn test_add_trans_listener() {
        let tmp_dir = create_tmp_dir();
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let mut blockchain = Blockchain::new(&keypair, &tmp_dir)
            .await
            .expect("Blockchain should have been created.");

        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            "some transaction".as_bytes().to_vec(),
            &keypair,
        );
        let called = Arc::new(Mutex::new(false));
        blockchain
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

        remove_tmp_dir(tmp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_last_block() {
        let tmp_dir = create_tmp_dir();
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let mut blockchain = Blockchain::new(&keypair, &tmp_dir)
            .await
            .expect("Blockchain should have been created.");

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
            .await
            .expect("Block should have been committed.");
        assert_ne!(None, blockchain.chain.last_block());

        remove_tmp_dir(tmp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_block() {
        let tmp_dir = create_tmp_dir();
        let keypair = identity::Keypair::generate_ed25519();
        let ed25519_key = match keypair.clone() {
            Ed25519(some) => some,
            _ => panic!("Key format is wrong"),
        };
        let mut blockchain = Blockchain::new(&ed25519_key, &tmp_dir)
            .await
            .expect("Blockchain should have been created.");

        let data = "Hello First Transaction";

        let result = blockchain
            .add_block(data.as_bytes().to_vec(), &keypair)
            .await;
        assert!(result.is_ok());
        assert_eq!(
            b"Hello First Transaction".to_vec(),
            blockchain.chain.last_block().unwrap().transactions[0].payload()
        );

        remove_tmp_dir(tmp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_update_block_from_peer() {
        let tmp_dir = create_tmp_dir();
        let keypair = identity::Keypair::generate_ed25519();
        let ed25519_key = match keypair.clone() {
            Ed25519(some) => some,
            _ => panic!("Key format is wrong"),
        };

        let mut blockchain = Blockchain::new(&ed25519_key, &tmp_dir)
            .await
            .expect("Blockchain should have been created.");

        let block = Box::new(Block::new(HashDigest::new(b""), 1, vec![], &ed25519_key));

        let result = blockchain.update_block_from_peers(block).await;
        assert!(result.is_ok());

        remove_tmp_dir(tmp_dir);
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pull_block() {
        let tmp_dir = create_tmp_dir();
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));
        let mut blockchain = Blockchain::new(&keypair, &tmp_dir)
            .await
            .expect("Blockchain should have been created.");

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
            .await
            .expect("Block should have been committed.");

        assert_eq!(1, blockchain.pull_blocks(0, 0).unwrap().len());
        assert_eq!(0, blockchain.pull_blocks(0, 2).unwrap().len());

        remove_tmp_dir(tmp_dir);
    }
}
