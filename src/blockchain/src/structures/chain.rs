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
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use super::header::Ordinal;
use crate::error::BlockchainError;
use crate::structures::block::Block;

#[derive(Serialize, Deserialize, Debug, Default, Clone, Decode, Encode, Hash, PartialEq, Eq)]
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

    pub fn get_block_position(&self, ordinal: Ordinal) -> Result<usize, BlockchainError> {
        if ordinal >= self.len() as Ordinal {
            return Err(BlockchainError::InvalidBlockchainLength(self.len()));
        }

        if ordinal > self.last_block().unwrap().header.ordinal {
            return Err(BlockchainError::InvalidBlockchainLength(self.len()));
        }

        if ordinal >= usize::MAX as Ordinal - 1 {
            return Err(BlockchainError::InvalidBlockchainOrdinal(ordinal));
        }

        if self.blocks[ordinal as usize].header.ordinal != ordinal {
            return Err(BlockchainError::InvalidBlockchainOrdinal(ordinal));
        }

        Ok(ordinal as usize)
    }

    pub async fn save_block(
        &self,
        start: Ordinal,
        mut end: Ordinal,
        file_path: String,
    ) -> Result<(), BlockchainError> {
        match self.last_block() {
            None => return Err(BlockchainError::InvalidBlockchainLength(self.len())),
            Some(block) => {
                if start > block.header.ordinal {
                    return Err(BlockchainError::InvalidBlockchainLength(self.len()));
                }
                // The end ordinal out of range is currently allowed, replacing it with the ordinal of the last block.
                if end > block.header.ordinal {
                    warn!("The end ordinal {:?} out of bounds ", end);
                    end = block.header.ordinal;
                }
            }
        }

        let start_pos = self.get_block_position(start)?;

        let end_pos = self.get_block_position(end)?;

        if start_pos > end_pos {
            return Err(BlockchainError::InvalidBlockchainPosition(
                start_pos, end_pos,
            ));
        }

        let file = OpenOptions::new()
            .create(true)
            .write(true)
            .read(true)
            .append(true)
            .open(file_path)
            .await;

        match file {
            Ok(mut file) => {
                file.write_all(&serde_json::to_vec(&self.blocks[start_pos..=end_pos]).unwrap())
                    .await?;

                file.sync_all().await?;
            }
            Err(e) => return Err(BlockchainError::IOError(e)),
        }

        Ok(())
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use std::env;

    use crate::{
        crypto::hash_algorithm::HashDigest,
        structures::{
            block::Block,
            chain::Chain,
            header::Address,
            transaction::{Transaction, TransactionType},
        },
    };
    use libp2p::identity;
    use tokio::fs;

    #[test]
    fn test_add_block() -> Result<(), String> {
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
        Ok(())
    }

    #[test]
    fn test_chain_is_empty() -> Result<(), String> {
        let chain: Chain = Default::default();

        assert!(chain.is_empty());

        Ok(())
    }

    #[test]
    fn test_chain_is_not_empty() -> Result<(), String> {
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block);
        assert_eq!(false, chain.is_empty());

        Ok(())
    }

    #[test]
    fn test_blocks() -> Result<(), String> {
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block);
        assert_eq!(chain.len(), chain.blocks().len());

        Ok(())
    }

    #[test]
    fn test_chain_length() -> Result<(), String> {
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        chain.add_block(block);
        assert_eq!(2, chain.len());

        Ok(())
    }

    #[test]
    fn test_get_last_block() -> Result<(), String> {
        let mut chain: Chain = Default::default();
        assert_eq!(None, chain.last_block());
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(block, chain.last_block().unwrap());
        Ok(())
    }

    #[tokio::test]
    async fn test_save_block_succeed() -> Result<(), String> {
        let temp_file: &str = &get_temp_file().unwrap();
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(1, chain.len());
        chain.add_block(block.clone());
        assert!(chain.save_block(0, 1, temp_file.to_string()).await.is_ok());
        let contents = fs::read(temp_file).await.unwrap();
        assert_eq!(
            "[{\"header\":{\"parent_h",
            std::str::from_utf8(&contents[..=20]).unwrap()
        );
        assert!(fs::remove_file(temp_file).await.is_ok());
        Ok(())
    }

    #[tokio::test]
    // Attempt to create file without permission, failed
    async fn test_save_block_failed_for_ioerror() -> Result<(), String> {
        let temp_file = "/tests/resources/blockchain.json";
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(1, chain.len());
        assert!(chain.save_block(0, 0, temp_file.to_string()).await.is_err());
        Ok(())
    }

    #[tokio::test]
    // Attempt to save blocks whose starting ordinal exceeds the current blockchain length, failed
    async fn test_save_block_failed_for_invalid_blockhain_length() -> Result<(), String> {
        let temp_file = get_temp_file().unwrap();
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(1, chain.len());
        assert_eq!(
            "Err(InvalidBlockchainLength(1))",
            format!("{:?}", chain.save_block(5, 0, temp_file.to_string()).await)
        );
        Ok(())
    }

    #[tokio::test]
    // Attempt to save blocks with invalid ordinal, failed
    async fn test_save_block_failed_for_invalid_blockhain_ordinal() -> Result<(), String> {
        let temp_file = get_temp_file().unwrap();
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(1, chain.len());
        chain.blocks[0].header.ordinal = 3;
        assert_eq!(
            "Err(InvalidBlockchainOrdinal(0))",
            format!("{:?}", chain.save_block(0, 0, temp_file.to_string()).await)
        );
        Ok(())
    }

    fn get_temp_file() -> Result<String, anyhow::Error> {
        // test blockchain file in tests/resources/ dir
        let mut curr_dir = env::temp_dir();
        curr_dir.push("blockchain.json");
        Ok(String::from(curr_dir.to_string_lossy()))
    }
}
