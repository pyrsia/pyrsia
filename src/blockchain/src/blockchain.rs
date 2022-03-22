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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fmt::{self, Debug, Display, Formatter};

use super::block::*;
use super::crypto::hash_algorithm::HashDigest;
use super::header::*;

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

/// Define Configuration Information
#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Config {
    pub blockchain_id: BlockchainId,
    pub signature_algorithm: SignatureAlgorithm,
    pub key_size: u32,
}

impl Config {
    pub fn new() -> Self {
        Self {
            blockchain_id: BlockchainId::Pyrsia,
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

#[derive(Serialize, Deserialize)]
pub struct Blockchain {
    #[serde(skip)]
    // this should actually be a Map<Transaction,Vec<OnTransactionSettled>> but that's later
    pub trans_observers: HashMap<Transaction, Box<dyn FnOnce(Transaction)>>,
    #[serde(skip)]
    pub block_observers: Vec<Box<dyn FnMut(Block)>>,
    pub blocks: Vec<Block>,
}

impl Debug for Blockchain {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Blockchain {
            trans_observers: _,
            blocks,
            block_observers: _,
        } = self;

        f.debug_struct("Blockchain")
            .field("blocks", blocks)
            .field("trans_observers", &self.trans_observers.len())
            .field("block_observers", &self.block_observers.len())
            .finish()
    }
}

impl Blockchain {
    pub fn new(keypair: &identity::ed25519::Keypair) -> Self {
        let local_id = HashDigest::new(&get_publickey_from_keypair(&keypair).encode());
        Self {
            trans_observers: Default::default(),
            block_observers: vec![],
            // this is the "genesis" blocks
            blocks: Vec::from([Block::new(
                Header::new(PartialHeader::new(
                    HashDigest::new(b""),
                    local_id,
                    HashDigest::new(b""),
                    1,
                )),
                Vec::from([Transaction::new(
                    PartialTransaction::new(
                        TransactionType::AddAuthority,
                        local_id,
                        "this needs to be the root authority".as_bytes().to_vec(),
                    ),
                    &keypair,
                )]),
                keypair,
            )]),
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

// Create a new block
// why isn't this just Block::new
pub fn new_block(
    keypair: &identity::ed25519::Keypair,
    transactions: &[Transaction],
    parent_hash: HashDigest,
    previous_number: u128,
) -> Block {
    let local_id = HashDigest::new(&get_publickey_from_keypair(keypair).encode());
    let transaction_root = HashDigest::new(&bincode::serialize(transactions).unwrap());
    let block_header = Header::new(PartialHeader::new(
        parent_hash,
        local_id,
        transaction_root,
        previous_number + 1,
    ));
    Block::new(block_header, transactions.to_vec(), keypair)
}

//ToDo
pub fn generate_ed25519() -> identity::Keypair {
    //RFC8032
    identity::Keypair::generate_ed25519()
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
        let keypair = generate_ed25519();
        let ed25519_keypair = match keypair {
            identity::Keypair::Ed25519(v) => v,
            identity::Keypair::Rsa(_) => todo!(),
            identity::Keypair::Secp256k1(_) => todo!(),
        };
        let local_id = HashDigest::new(&get_publickey_from_keypair(&ed25519_keypair).encode());
        let mut chain = Blockchain::new(&ed25519_keypair);

        let mut transactions = vec![];
        let data = "Hello First Transaction";
        let transaction = Transaction::new(
            PartialTransaction::new(TransactionType::Create, local_id, data.as_bytes().to_vec()),
            &ed25519_keypair,
        );
        transactions.push(transaction);
        chain.add_block(new_block(
            &ed25519_keypair,
            &transactions,
            chain.blocks[0].header.hash,
            chain.blocks[0].header.number,
        ));
        assert_eq!(true, chain.blocks.last().unwrap().verify());
        assert_eq!(2, chain.blocks.len());
        Ok(())
    }

    #[test]
    fn test_add_trans_listener() -> Result<(), String> {
        let keypair = generate_ed25519();
        let ed25519_keypair = match keypair {
            identity::Keypair::Ed25519(v) => v,
            identity::Keypair::Rsa(_) => todo!(),
            identity::Keypair::Secp256k1(_) => todo!(),
        };
        let local_id = HashDigest::new(&get_publickey_from_keypair(&ed25519_keypair).encode());
        let mut chain = Blockchain::new(&ed25519_keypair);

        let transaction = Transaction::new(
            PartialTransaction::new(
                TransactionType::Create,
                local_id,
                "some transaction".as_bytes().to_vec(),
            ),
            &ed25519_keypair,
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
        let ed25519_keypair = match generate_ed25519() {
            identity::Keypair::Ed25519(v) => v,
            identity::Keypair::Rsa(_) => todo!(),
            identity::Keypair::Secp256k1(_) => todo!(),
        };
        let local_id = HashDigest::new(&get_publickey_from_keypair(&ed25519_keypair).encode());

        let block_header = Header::new(PartialHeader::new(
            HashDigest::new(b""),
            local_id,
            HashDigest::new(b""),
            1,
        ));

        let block = Block::new(
            block_header,
            Vec::new(),
            &identity::ed25519::Keypair::generate(),
        );
        let mut chain = Blockchain::new(&ed25519_keypair);
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
