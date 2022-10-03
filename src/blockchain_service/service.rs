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

use bincode::{deserialize, serialize};
use libp2p::identity;
use libp2p::PeerId;
use log::warn;
use pyrsia_blockchain_network::blockchain::Blockchain;
use pyrsia_blockchain_network::error::BlockchainError;
use pyrsia_blockchain_network::structures::block::Block;
use pyrsia_blockchain_network::structures::header::Ordinal;
use std::cmp::Ordering;
use std::fmt::{self, Debug, Formatter};

use crate::network::client::Client;

/// Blockchain command length is 1 byte
pub const BLOCKCHAIN_COMMAND_LENGTH: usize = 1;

/// Blockchain ordinal length is 16 bytes
pub const BLOCKCHAIN_ORDINAL_LENGTH: usize = 16;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BlockchainCommand {
    Broadcast = 1,                // Broadcast the updated block to all other nodes
    PushToPeer = 2,               // Send a block to a peer
    PullFromPeer = 3,             // Pull blocks from a peer
    QueryHighestBlockOrdinal = 4, // Query the current highest (latest) block ordinal number from other nodes
}

impl TryFrom<u8> for BlockchainCommand {
    type Error = &'static BlockchainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1u8 => Ok(Self::Broadcast),
            2u8 => Ok(Self::PushToPeer),
            3u8 => Ok(Self::PullFromPeer),
            4u8 => Ok(Self::QueryHighestBlockOrdinal),
            _ => Err(&BlockchainError::InvalidBlockchainCmd),
        }
    }
}

impl TryFrom<u8> for BlockchainCommand {
    type Error = &'static BlockchainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1u8 => Ok(Self::Broadcast),
            2u8 => Ok(Self::PushToPeer),
            3u8 => Ok(Self::PullFromPeer),
            4u8 => Ok(Self::QueryHighestBlockOrdinal),
            _ => Err(&BlockchainError::InvalidBlockchainCmd),
        }
    }
}

pub struct BlockchainService {
    blockchain: Blockchain,
    pub keypair: identity::ed25519::Keypair,
    pub p2p_client: Client,
}

impl Debug for BlockchainService {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        f.debug_struct("BlockchainService")
            .field("blockchain", &self.blockchain)
            .field("p2p_client", &self.p2p_client)
            .finish()
    }
}

impl BlockchainService {
    pub fn init_first_blockchain_node(
        local_keypair: &identity::ed25519::Keypair,
        blockchain_keypair: &identity::ed25519::Keypair,
        p2p_client: Client,
    ) -> Self {
        Self {
            blockchain: Blockchain::new(blockchain_keypair),
            keypair: local_keypair.to_owned(),
            p2p_client,
        }
    }

    pub fn init_other_blockchain_node(
        local_keypair: &identity::ed25519::Keypair,
        p2p_client: Client,
    ) -> Self {
        Self {
            blockchain: Default::default(),
            keypair: local_keypair.to_owned(),
            p2p_client,
        }
    }

    /// Add payload to blockchain. It will be called by other services (e.g. transparent logging service)
    pub async fn add_payload(&mut self, payload: Vec<u8>) -> Result<(), BlockchainError> {
        self.blockchain
            .add_block(payload, &identity::Keypair::Ed25519(self.keypair.clone()))
            .await?;

        self.broadcast_blockchain(Box::new(self.blockchain.last_block().unwrap()))
            .await?;
        Ok(())
    }

    /// Notify other nodes to add a new block.
    async fn broadcast_blockchain(&mut self, block: Box<Block>) -> Result<(), BlockchainError> {
        let peer_list = self.p2p_client.list_peers().await.unwrap_or_default();
        let cmd = BlockchainCommand::Broadcast as u8;
        let block_ordinal = block.header.ordinal as u128;

        let mut buf: Vec<u8> = vec![];

        let block = *block;

        log::debug!("Blockchain get block to broadcast:{:?}", block);

        let block = *block;

        log::debug!("Blockchain get block to broadcast:{:?}", block);

        buf.push(cmd);
        buf.append(&mut serialize(&block_ordinal).unwrap());
        buf.append(&mut serialize(&block).unwrap());

        for peer_id in peer_list.iter() {
            self.p2p_client
                .request_blockchain(peer_id, buf.clone())
                .await?;
        }

        Ok(())
    }

    async fn query_blockchain_ordinal(
        &mut self,
        other_peer_id: &PeerId,
    ) -> Result<Ordinal, BlockchainError> {
        let cmd = BlockchainCommand::QueryHighestBlockOrdinal as u8;

        let mut buf: Vec<u8> = vec![];

        log::debug!("Blockchain query ordinal from : {:?}", other_peer_id);

        buf.push(cmd);

        let ordinal = deserialize(
            &self
                .p2p_client
                .request_blockchain(other_peer_id, buf.clone())
                .await?,
        )
        .unwrap();

        Ok(ordinal)
    }

    async fn pull_block_from_other_nodes(
        &mut self,
        other_peer_id: &PeerId,
        start: Ordinal,
        end: Ordinal,
    ) -> Result<Vec<Block>, BlockchainError> {
        let cmd = BlockchainCommand::PullFromPeer as u8;

        let mut buf: Vec<u8> = vec![];

        log::debug!("Blockchain query ordinal from : {:?}", other_peer_id);

        buf.push(cmd);
        buf.append(&mut serialize(&start).unwrap());
        buf.append(&mut serialize(&end).unwrap());

        let blocks = deserialize(
            &self
                .p2p_client
                .request_blockchain(other_peer_id, buf.clone())
                .await?,
        )
        .unwrap();

        Ok(blocks)
    }

    /// Add a new block to local blockchain.
    pub async fn add_block(
        &mut self,
        ordinal: Ordinal,
        block: Box<Block>,
    ) -> Result<(), BlockchainError> {
        let last_block = self.blockchain.last_block();

        match last_block {
            None => {
                if ordinal == 0 {
                    self.blockchain.update_block_from_peers(block).await;
                }
            }

            Some(last_block) => {
                let expected = last_block.header.ordinal + 1;
                match ordinal.cmp(&expected) {
                    Ordering::Greater => return Err(BlockchainError::LaggingBlockchainData),
                    Ordering::Less => warn!("Blockchain received a duplicate block!"),
                    Ordering::Equal => self.blockchain.update_block_from_peers(block).await,
                }
            }
        }

        Ok(())
    }

    /// Retrieve Blocks form start ordinal number to end ordinal number (including end ordinal number)
    pub async fn pull_blocks(
        &self,
        start: Ordinal,
        end: Ordinal,
    ) -> Result<Vec<Block>, BlockchainError> {
        self.blockchain.pull_blocks(start, end)
    }

    pub async fn query_last_block(&self) -> Option<Block> {
        self.blockchain.last_block()
    }

    pub async fn init_pull_from_others(
        &mut self,
        other_peer_id: &PeerId,
    ) -> Result<Ordinal, BlockchainError> {
        // Always start with the genesis block
        let ordinal = self.query_blockchain_ordinal(other_peer_id).await?;

        for block in self
            .pull_block_from_other_nodes(other_peer_id, 0, ordinal)
            .await?
            .iter()
        {
            let ordinal = block.header.ordinal;
            let block = block.clone();
            self.add_block(ordinal, Box::new(block)).await?;
        }

        Ok(ordinal)
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use libp2p::identity::Keypair;
    use pyrsia_blockchain_network::crypto::hash_algorithm::HashDigest;
    use tokio::sync::mpsc;

    fn create_blockchain_service() -> BlockchainService {
        let (sender, _) = mpsc::channel(1);
        let ed25519_keypair = identity::ed25519::Keypair::generate();
        let local_peer_id = identity::PublicKey::Ed25519(ed25519_keypair.public()).to_peer_id();
        let client = Client {
            sender,
            local_peer_id,
        };

        BlockchainService::init_first_blockchain_node(&ed25519_keypair, &ed25519_keypair, client)
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_payload() -> Result<(), String> {
        let mut blockchain_service = create_blockchain_service();

        let payload = vec![];
        assert!(blockchain_service.blockchain.last_block().is_some());
        assert!(blockchain_service.add_payload(payload).await.is_ok());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_block() -> Result<(), String> {
        let mut blockchain_service = create_blockchain_service();

        let last_block = blockchain_service.blockchain.last_block().unwrap();

        let block = Block::new(
            last_block.header.hash(),
            1,
            vec![],
            &blockchain_service.keypair,
        );
        let _ = blockchain_service
            .add_block(1, Box::new(block.clone()))
            .await;

        let last_block = blockchain_service.blockchain.last_block().unwrap();
        assert_eq!(last_block, block);

        // Ordinal is not next, return error.
        assert!(blockchain_service
            .add_block(3, Box::new(block.clone()))
            .await
            .is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_init_first_blockchain_node() -> Result<(), String> {
        let (sender, _) = mpsc::channel(1);
        let ed25519_keypair = identity::ed25519::Keypair::generate();
        let local_peer_id = identity::PublicKey::Ed25519(ed25519_keypair.public()).to_peer_id();
        let client = Client {
            sender,
            local_peer_id,
        };

        let blockchain_service = BlockchainService::init_first_blockchain_node(
            &ed25519_keypair,
            &ed25519_keypair,
            client,
        );
        let block = blockchain_service.query_last_block().await.unwrap();
        assert_eq!(0, block.header.ordinal);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_init_other_blockchain_node() -> Result<(), String> {
        let (sender, _) = mpsc::channel(1);
        let ed25519_keypair = identity::ed25519::Keypair::generate();
        let local_peer_id = identity::PublicKey::Ed25519(ed25519_keypair.public()).to_peer_id();
        let client = Client {
            sender,
            local_peer_id,
        };

        let blockchain_service =
            BlockchainService::init_other_blockchain_node(&ed25519_keypair, client);
        assert_eq!(None, blockchain_service.query_last_block().await);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_pull_blocks() -> Result<(), String> {
        let blockchain_service = create_blockchain_service();
        assert_eq!(1, blockchain_service.pull_blocks(0, 0).await.unwrap().len());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_query_last_block() -> Result<(), String> {
        let mut blockchain_service = create_blockchain_service();

        let last_block = blockchain_service.blockchain.last_block().unwrap();

        let block = Block::new(
            last_block.header.hash(),
            1,
            vec![],
            &blockchain_service.keypair,
        );
        let _ = blockchain_service
            .add_block(1, Box::new(block.clone()))
            .await;

        assert_eq!(
            1,
            blockchain_service
                .query_last_block()
                .await
                .unwrap()
                .header
                .ordinal
        );

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_query_blockchain_ordinal_with_invalid_other_peer() -> Result<(), String> {
        let mut blockchain_service = create_blockchain_service();

        let other_peer_id = Keypair::generate_ed25519().public().to_peer_id();

        assert!(blockchain_service
            .query_blockchain_ordinal(&other_peer_id)
            .await
            .is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_init_pull_from_others_with_invalid_other_peer() -> Result<(), String> {
        let mut blockchain_service = create_blockchain_service();

        let other_peer_id = Keypair::generate_ed25519().public().to_peer_id();

        assert!(blockchain_service
            .init_pull_from_others(&other_peer_id)
            .await
            .is_err());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_notify_blockchain() -> Result<(), String> {
        let mut blockchain_service = create_blockchain_service();

        let block = Box::new(Block::new(
            HashDigest::new(b""),
            1,
            vec![],
            &blockchain_service.keypair,
        ));
        assert!(blockchain_service.broadcast_blockchain(block).await.is_ok());

        Ok(())
    }

    #[test]
    fn test_debug() -> Result<(), String> {
        let blockchain_service = create_blockchain_service();

        assert_ne!(
            format!("This is blockchain service {blockchain_service:?}"),
            "This is blockchain service"
        );
        Ok(())
    }

    #[test]
    fn test_blochchain_command_convert_to_u8() -> Result<(), String> {
        assert_eq!(1u8, BlockchainCommand::Broadcast as u8);

        assert_eq!(2u8, BlockchainCommand::PushToPeer as u8);

        assert_eq!(3u8, BlockchainCommand::PullFromPeer as u8);

        assert_eq!(4u8, BlockchainCommand::QueryHighestBlockOrdinal as u8);

        Ok(())
    }

    #[test]
    fn test_blochchain_command_convert_from_u8() -> Result<(), String> {
        assert_eq!(
            BlockchainCommand::try_from(1u8).unwrap(),
            BlockchainCommand::Broadcast
        );

        assert_eq!(
            BlockchainCommand::try_from(2u8).unwrap(),
            BlockchainCommand::PushToPeer
        );

        assert_eq!(
            BlockchainCommand::try_from(3u8).unwrap(),
            BlockchainCommand::PullFromPeer
        );

        assert_eq!(
            BlockchainCommand::try_from(4u8).unwrap(),
            BlockchainCommand::QueryHighestBlockOrdinal
        );

        assert!(BlockchainCommand::try_from(47u8).is_err());

        Ok(())
    }
}
