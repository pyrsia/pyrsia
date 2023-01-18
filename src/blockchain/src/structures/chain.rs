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
use codec::{Decode, Encode};
use log::warn;
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs::{self, OpenOptions};
use tokio::io::AsyncWriteExt;

use super::header::Ordinal;
use crate::error::BlockchainError;
use crate::structures::block::Block;

#[derive(Serialize, Deserialize, Debug, Default, Decode, Encode, Hash, PartialEq, Eq)]
pub struct Chain {
    // The block sequence is always sorted by the ordinal, guaranteed by the hash and parent hash
    blocks: Vec<Block>,
}

impl Chain {
    pub fn blocks(&self) -> Vec<Block> {
        self.blocks.clone()
    }

    pub fn len(&self) -> usize {
        self.blocks.len()
    }

    pub fn is_empty(&self) -> bool {
        self.blocks.is_empty()
    }

    pub fn add_block(&mut self, block: Block) {
        self.blocks.push(block);
    }

    pub fn last_block(&self) -> Option<Block> {
        self.blocks().last().cloned()
    }

    pub fn get_block_position(&self, ordinal: Ordinal) -> Option<usize> {
        if ordinal >= self.len() as Ordinal {
            warn!("Blockchain try to get non-exsit block {:?}", ordinal);
            return None;
        }

        if ordinal > self.last_block().unwrap().header.ordinal {
            warn!("Blockchain try to get non-exsit block {:?}", ordinal);
            return None;
        }

        if self.blocks[ordinal as usize].header.ordinal != ordinal {
            warn!("Blockchain try to get non-exsit block {:?}", ordinal);
            return None;
        }

        Some(ordinal as usize)
    }

    pub fn retrieve_blocks(&self, start: Ordinal, end: Ordinal) -> Vec<Block> {
        if let (Some(start_pos), Some(end_pos)) =
            (self.get_block_position(start), self.get_block_position(end))
        {
            if start_pos <= end_pos {
                return self.blocks[start_pos..=end_pos].to_vec();
            }
        }

        Default::default()
    }

    pub async fn save_block(
        &self,
        ordinal: Ordinal,
        file_path: impl AsRef<Path>,
    ) -> Result<(), BlockchainError> {
        let block_position = self
            .get_block_position(ordinal)
            .ok_or_else(|| BlockchainError::InvalidBlockchainLength(self.len()))?;

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .append(false)
            .open(file_path)
            .await;

        match file {
            Ok(mut file) => {
                file.write_all(&bincode::serialize(&self.blocks[block_position])?)
                    .await?;

                file.sync_all().await?;
            }
            Err(e) => return Err(BlockchainError::IOError(e)),
        }

        Ok(())
    }

    /// Reads a list of blocks from the specified directory path
    /// and adds them to the chain.
    pub async fn load_blocks(&mut self, path: impl AsRef<Path>) -> Result<(), BlockchainError> {
        let blockchain_path = path.as_ref().to_path_buf();
        let mut ordinal = 0;
        loop {
            let block_path = blockchain_path.join(format!("{}.ser", ordinal));
            if let Ok(block_metadata) = fs::metadata(&block_path).await {
                if block_metadata.is_file() {
                    let block_bytes = fs::read(block_path).await?;
                    self.add_block(bincode::deserialize(&block_bytes)?);
                } else {
                    break;
                }
            } else {
                break;
            }

            ordinal += 1;
        }

        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use std::path::PathBuf;

    use crate::{
        crypto::hash_algorithm::HashDigest,
        structures::{
            chain::Chain,
            header::Address,
            transaction::{Transaction, TransactionType},
        },
    };
    use libp2p::identity;

    #[test]
    fn test_add_block() {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let mut chain: Chain = Default::default();

        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            data.as_bytes().to_vec(),
            &keypair,
        );
        transactions.push(transaction);
        assert_eq!(0, chain.len());
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block);
        assert_eq!(1, chain.len());
    }

    #[test]
    fn test_chain_is_empty() {
        let chain: Chain = Default::default();
        assert!(chain.is_empty());
    }

    #[test]
    fn test_chain_is_not_empty() {
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block);
        assert!(!chain.is_empty());
    }

    #[test]
    fn test_blocks() {
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block);
        assert_eq!(chain.len(), chain.blocks().len());
    }

    #[test]
    fn test_chain_length() {
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        chain.add_block(block);
        assert_eq!(2, chain.len());
    }

    #[test]
    fn test_get_last_block() {
        let mut chain: Chain = Default::default();
        assert_eq!(None, chain.last_block());
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(block, chain.last_block().unwrap());
    }

    #[tokio::test]
    async fn test_save_block_succeed() {
        let temp_file = get_temp_file();
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let block_1 = Block::new(HashDigest::new(b""), 0, vec![], &keypair);
        chain.add_block(block_1.clone());
        assert_eq!(1, chain.len());
        assert!(chain.save_block(0, &temp_file).await.is_ok());
        let serialized_block = bincode::serialize(&block_1.clone()).unwrap();
        let file_contents = fs::read(&temp_file).await.unwrap();
        assert_eq!(serialized_block, file_contents);
        assert!(fs::remove_file(temp_file).await.is_ok());
    }

    #[tokio::test]
    // Attempt to create file without permission, failed
    async fn test_save_block_failed_for_ioerror() {
        let temp_file = "/tests/resources/blockchain.json";
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(1, chain.len());
        assert!(chain.save_block(0, temp_file.to_string()).await.is_err());
    }

    #[tokio::test]
    // Attempt to save blocks whose starting ordinal exceeds the current blockchain length, failed
    async fn test_save_block_failed_for_invalid_blockhain_length() {
        let temp_file = get_temp_file();
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(1, chain.len());
        assert_eq!(
            "Err(InvalidBlockchainLength(1))",
            format!("{:?}", chain.save_block(5, temp_file).await)
        );
    }

    #[tokio::test]
    // Attempt to save blocks with invalid ordinal, failed
    async fn test_save_block_failed_for_invalid_blockhain_ordinal() {
        let temp_file = get_temp_file();
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(1, chain.len());
        chain.blocks[0].header.ordinal = 3;
        assert_eq!(
            "Err(InvalidBlockchainLength(1))",
            format!("{:?}", chain.save_block(0, temp_file).await)
        );
    }

    #[tokio::test]
    async fn test_load_empty_blocks() {
        let temp_dir = tempfile::tempdir().unwrap().into_path();
        let mut chain: Chain = Default::default();

        chain
            .load_blocks(temp_dir)
            .await
            .expect("blocks should have been loaded");

        assert!(chain.is_empty());
    }

    #[tokio::test]
    async fn test_load_blocks() {
        let temp_dir = tempfile::tempdir().unwrap().into_path();

        let keypair = identity::ed25519::Keypair::generate();
        let block1 = Block::new(HashDigest::new(b""), 0, vec![], &keypair);
        let block2 = Block::new(HashDigest::new(b""), 1, vec![], &keypair);

        let mut chain: Chain = Default::default();
        chain.add_block(block1);
        chain.add_block(block2);

        chain
            .save_block(0, temp_dir.join("0.ser"))
            .await
            .expect("block should be saved");
        chain
            .save_block(1, temp_dir.join("1.ser"))
            .await
            .expect("block should be saved");

        let mut chain2: Chain = Default::default();
        chain2
            .load_blocks(temp_dir)
            .await
            .expect("blocks should have been loaded");

        assert_eq!(2, chain2.len());
    }

    fn get_temp_file() -> PathBuf {
        tempfile::tempdir()
            .expect("could not create temporary directory")
            .into_path()
            .join("blockchain.ser")
    }

    #[test]
    fn test_retrieve_block() {
        let keypair = identity::ed25519::Keypair::generate();
        let local_id = Address::from(identity::PublicKey::Ed25519(keypair.public()));

        let mut chain: Chain = Default::default();
        assert_eq!(0, chain.len());

        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            TransactionType::Create,
            local_id,
            data.as_bytes().to_vec(),
            &keypair,
        );
        transactions.push(transaction);

        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block);
        assert_eq!(1, chain.len());

        let blocks = chain.retrieve_blocks(0, 0);
        assert_eq!(0, blocks[0].header.ordinal);

        assert_eq!(0, chain.retrieve_blocks(0, 1).len());
    }
}
