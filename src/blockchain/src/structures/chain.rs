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
use serde::{Deserialize, Serialize};

use super::block::Block;

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
}
