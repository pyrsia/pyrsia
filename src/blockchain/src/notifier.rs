use std::fmt;
use std::fmt::{Debug, Formatter};

use crate::block::{Block, Transaction};
use crate::blockchain::Blockchain;

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct BlockchainError;

pub type BlockchainResult = std::result::Result<(), BlockchainError>;
pub type OnTransactionSettled = Box<dyn FnOnce(&Transaction)>;
pub type OnBlockEvent = Box<dyn FnMut(&Block)>;

// But we require certain bounds to get things done...
impl<'lt> Blockchain<'lt> {
    // should we borrow or own this transaction?
    pub fn submit_transaction(&mut self, trans: &'lt Transaction, on_done: OnTransactionSettled) {
        self.trans_observers.insert(trans, on_done);
    }
    // block_chain.add_block_listener(|block| {
    // save to db
    //})
    pub fn notify_transaction_settled(&mut self, trans: &Transaction) {
        // if there were no observers, we don't care
        if let Some(on_settled) = self.trans_observers.remove(trans) {
            on_settled(trans)
        }
    }

    pub fn add_block_listener(&mut self, on_block: OnBlockEvent) -> &mut Self {
        self.block_observers.push(on_block);
        self
    }

    pub fn notify_block_event(&mut self, block: &Block) -> &mut Self {
        self.block_observers
            .iter_mut()
            .for_each(|notify| notify(block));
        self
    }
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid first item to double")
    }
}

impl<'lt> Debug for Blockchain<'lt> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        let Blockchain {
            trans_observers: _,
            genesis_block,
            blocks,
            block_observers: _,
        } = self;

        f.debug_struct("Blockchain")
            .field("genesis_block", genesis_block)
            .field("blocks", blocks)
            .finish()
    }
}
