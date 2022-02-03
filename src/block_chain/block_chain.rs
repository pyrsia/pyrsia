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

use super::block::*;
use super::header::*;
use libp2p::identity;
use rand::Rng;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

/// BlockchainId identifies the current chain
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum BlockchainId {
    Pyrsia,
}

/// Define Supported Hash Algorithm
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum HashAlgorithm {
    Keccak,
}

/// Define Supported Signature Algorithm
#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum SignatureAlgorithm {
    Ed25519,
}

/// Define Configuration Information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub blockchain_id: BlockchainId,
    pub hash_algorithm: HashAlgorithm,
    pub hash_size: u32, //sizes of u8
    pub signature_algorithm: SignatureAlgorithm,
    pub key_size: u32,
}

impl Config {
    pub fn new() -> Self {
        Self {
            blockchain_id: BlockchainId::Pyrsia,
            hash_algorithm: HashAlgorithm::Keccak,
            hash_size: 32, //256bits
            signature_algorithm: SignatureAlgorithm::Ed25519,
            key_size: 32, //256bits
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self::new()
    }
}

/// Define Genesis Block
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct GenesisBlock {
    pub header: Header,
    pub config: Config,
    pub signature: BlockSignature,
}

impl GenesisBlock {
    pub fn new(keypair: &identity::ed25519::Keypair) -> Self {
        let local_id = hash(&get_publickey_from_keypair(keypair).encode());
        let config = Config::new();
        let header = Header::new(PartialHeader::new(
            hash(b""),
            local_id,
            hash(&(bincode::serialize(&config).unwrap())),
            0,
            rand::thread_rng().gen::<u128>(),
        ));

        Self {
            header,
            config,
            signature: Signature::new(&bincode::serialize(&header.current_hash).unwrap(), keypair),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Blockchain {
    pub genesis_block: GenesisBlock,
    pub blocks: Vec<Block>,
}

impl Blockchain {
    pub fn new(keypair: &identity::Keypair) -> Self {
        let ed25519_keypair = match keypair {
            identity::Keypair::Ed25519(v) => v,
            identity::Keypair::Rsa(_) => todo!(),
            identity::Keypair::Secp256k1(_) => todo!(),
        };
        Self {
            genesis_block: GenesisBlock::new(ed25519_keypair),
            blocks: vec![],
        }
    }

    pub fn new_block(
        &mut self,
        keypair: &identity::ed25519::Keypair,
        transactions: &[Transaction],
    ) {
        let (parent_hash, previous_number) = match self.blocks.last() {
            Some(block) => (block.header.current_hash, block.header.number),
            None => (
                self.genesis_block.header.current_hash,
                self.genesis_block.header.number,
            ),
        };

        let local_id = hash(&get_publickey_from_keypair(keypair).encode());
        let transaction_root = hash(&bincode::serialize(transactions).unwrap());
        let block_header = Header::new(PartialHeader::new(
            parent_hash,
            local_id,
            transaction_root,
            previous_number + 1,
            rand::thread_rng().gen::<u128>(),
        ));
        let block = Block::new(block_header, transactions.to_vec(), keypair);
        self.blocks.push(block);
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
    use super::*;

    #[test]
    fn test_build_blockchain() -> Result<(), String> {
        let local_keypair = identity::Keypair::generate_ed25519();

        let mut chain = Blockchain::new(&local_keypair);

        let keypair = match local_keypair {
            identity::Keypair::Ed25519(v) => v,
            identity::Keypair::Rsa(_) => todo!(),
            identity::Keypair::Secp256k1(_) => todo!(),
        };
        let local_id = hash(&get_publickey_from_keypair(&keypair).encode());
        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            PartialTransaction::new(
                TransactionType::Create,
                local_id,
                data.as_bytes().to_vec(),
                rand::thread_rng().gen::<u128>(),
            ),
            &keypair,
        );
        transactions.push(transaction);
        chain.new_block(&keypair, &transactions);
        chain.new_block(&keypair, &transactions);
        assert_eq!(true, chain.blocks.last().unwrap().verify());
        assert_eq!(2, chain.blocks.len());
        Ok(())
    }
}
