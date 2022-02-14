use std::fmt;
use std::fmt::{Debug, Formatter};

use crate::block::{Block, Transaction};
use crate::blockchain::Blockchain;

#[derive(Hash, Eq, PartialEq, Clone)]
pub struct BlockchainError;

pub type BlockchainResult = std::result::Result<(), BlockchainError>;

pub type OnTransactionSettled<'a> = Box<dyn FnOnce(Transaction) + 'a>;
pub type OnBlockEvent = Box<dyn FnMut(Block)>;

// But we require certain bounds to get things done...
impl Blockchain {
    // should we borrow or own this transaction?
    pub fn submit_transaction<'a>(&mut self, trans: Transaction, on_done: OnTransactionSettled<'a>) {
        self.trans_observers.insert(trans, on_done);
    }
    // block_chain.add_block_listener(|block| {
    // save to db
    //})
    pub fn notify_transaction_settled(&mut self, trans: Transaction) {
        // if there were no observers, we don't care
        if let Some(on_settled) = self.trans_observers.remove(&trans) {
            on_settled(trans)
        }
    }

    pub fn add_block_listener(&mut self, on_block: OnBlockEvent) -> &mut Self {
        self.block_observers.push(on_block);
        self
    }

    pub fn notify_block_event(&mut self, block: Block) -> &mut Self {
        self.block_observers
            .iter_mut()
            .for_each(|notify| notify(block.clone()));
        self
    }
}

impl fmt::Display for BlockchainError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "invalid first item to double")
    }
}

impl Debug for Blockchain {
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

mod test {
    use libp2p::identity;
    use rand::Rng;

    use crate::block::{
        get_publickey_from_keypair, Block, PartialTransaction, Transaction, TransactionType,
    };
    use crate::blockchain::{generate_ed25519, Blockchain};
    use crate::header::hash;
    use crate::notifier::OnBlockEvent;
    use crate::{block, header};

    #[test]
    fn test_add_trans_listener() -> Result<(), String> {
        let keypair = generate_ed25519();
        let ed25519_keypair = match keypair {
            identity::Keypair::Ed25519(v) => v,
            identity::Keypair::Rsa(_) => todo!(),
            identity::Keypair::Secp256k1(_) => todo!(),
        };
        let local_id = hash(&get_publickey_from_keypair(&ed25519_keypair).encode());
        let mut chain = Blockchain::new(&ed25519_keypair);

        let transaction = Transaction::new(
            PartialTransaction::new(
                TransactionType::Create,
                local_id,
                "some transaction".as_bytes().to_vec(),
                rand::thread_rng().gen::<u128>(),
            ),
            &ed25519_keypair,
        );
        let mut called: bool = false;
        let mut lambda = |trans: Transaction| {
            assert_eq!(transaction, trans);
            called = true;
        };
        chain.submit_transaction(transaction.clone(), Box::new(lambda));
        chain.notify_transaction_settled(transaction.clone());
        assert!(called);
        Ok(())
    }

    #[test]
    fn test_add_block_listener() -> Result<(), String> {
        let ed25519_keypair = match generate_ed25519() {
            identity::Keypair::Ed25519(v) => v,
            identity::Keypair::Rsa(_) => todo!(),
            identity::Keypair::Secp256k1(_) => todo!(),
        };
        let local_id = hash(&get_publickey_from_keypair(&ed25519_keypair).encode());

        let block_header = header::Header::new(header::PartialHeader::new(
            header::hash(b""),
            local_id,
            header::hash(b""),
            1,
            rand::thread_rng().gen::<u128>(),
        ));

        let mut block = block::Block::new(
            block_header,
            Vec::new(),
            &identity::ed25519::Keypair::generate(),
        );
        let mut chain = Blockchain::new(&ed25519_keypair);
        let mut called: bool = false;

        let foo = |b: Block| {
            called = true;
            assert_eq!(block, b);
        };
        chain.add_block_listener(Box::new(foo));
        chain.add_block(block);
        assert!(called);
        Ok(())
    }
}
