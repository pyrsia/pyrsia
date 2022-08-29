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
use crate::error::BlockchainError;
use crate::structures::block::Block;
use codec::{Decode, Encode};
use log::warn;
use serde::{Deserialize, Serialize};
use tokio::{fs::OpenOptions, io::AsyncWriteExt};

use super::header::Ordinal;

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
            return Err(BlockchainError::InvalidBlockchainOrdianl(ordinal));
        }

        if self.blocks[ordinal as usize].header.ordinal != ordinal {
            return Err(BlockchainError::InvalidBlockchainOrdianl(ordinal));
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

                if end > block.header.ordinal {
                    warn!("The end ordinal {:?} out of bounds ", end);
                    end = block.header.ordinal;
                }
            }
        }

        let start_pos = match self.get_block_position(start) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

        let end_pos = match self.get_block_position(end) {
            Ok(v) => v,
            Err(e) => return Err(e),
        };

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
            Err(e) => return Err(BlockchainError::Error(e.to_string())),
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
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
    async fn test_save_last_block() -> Result<(), String> {
        let temp_file = "./blockchain.json";
        let mut chain: Chain = Default::default();
        let keypair = identity::ed25519::Keypair::generate();
        let transactions = vec![];
        let block = Block::new(HashDigest::new(b""), 0, transactions, &keypair);
        chain.add_block(block.clone());
        assert_eq!(1, chain.len());
        assert!(chain.save_block(0, 0, temp_file.to_string()).await.is_ok());
        assert!(fs::remove_file(temp_file).await.is_ok());
        Ok(())
    }
}
