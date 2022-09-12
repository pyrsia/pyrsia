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
use pyrsia_blockchain_network::blockchain::Blockchain;
use pyrsia_blockchain_network::error::BlockchainError;
use pyrsia_blockchain_network::structures::block::Block;
use pyrsia_blockchain_network::structures::header::Ordinal;
use std::fmt::{self, Debug, Formatter};

use crate::network::client::Client;

#[derive(Copy, Clone, Debug, Eq, PartialEq)]
#[repr(u8)]
pub enum BlockchainCommand {
    Broadcast =1,
    PushFromPeer =2 ,
    PullFromPeer =3,
    QueryHighestBlockOrdinal =4,
}

impl TryFrom<u8> for BlockchainCommand {
    type Error = &'static BlockchainError;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        match value {
            1u8 => Ok(Self::Broadcast),
            2u8 => Ok(Self::PushFromPeer),
            3u8 => Ok(Self::PullFromPeer),
            4u8 => Ok(Self::QueryHighestBlockOrdinal),
            _ =>Err(&BlockchainError::InvalidBlockchainCmd)
        }
    }
}


pub struct BlockchainService {
    pub blockchain: Blockchain,
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
    pub fn new(keypair: &identity::ed25519::Keypair, p2p_client: Client) -> Self {
        Self {
            blockchain: Blockchain::new(keypair),
            p2p_client,
        }
    }

    /// Add payload to blockchain. It will be called by other services (e.g. transparent logging service)
    pub async fn add_payload(&mut self, payload: Vec<u8>, local_key: &identity::Keypair) ->Result<(), BlockchainError> {
        let _ = self.blockchain.add_block(payload, local_key).await;
        self.broadcast_blockchain(Box::new(self.blockchain.last_block().unwrap()))
            .await?;
        Ok(())
    }

    /// Notify other nodes to add a new block.
    async fn broadcast_blockchain(&mut self, block: Box<Block>) ->Result<(), BlockchainError> {
        let peer_list = self.p2p_client.list_peers().await?;
        let cmd = BlockchainCommand::Broadcast as u8;
        let block_ordinal = block.header.ordinal as u128;

        let mut buf:Vec<u8> = vec![];

        buf.push(cmd);
        buf.append(&mut bincode::serialize(&block_ordinal).unwrap());
        buf.append(&mut bincode::serialize(&block).unwrap());

        for peer_id in peer_list.iter() {
            self.p2p_client.request_blockchain(peer_id, buf.clone()).await?;
        }

        Ok(())
    }

    /// Add a new block to local blockchain.
    pub async fn add_block(&mut self, ordinal: Ordinal, block: Box<Block>) {
        let last_block = self.blockchain.last_block();

        match last_block {
            None => {
                if ordinal == 0 {
                    self.blockchain.update_block_from_peers(block).await;
                }
            }

            Some(last_block) => {
                if ordinal == last_block.header.ordinal + 1 {
                    self.blockchain.update_block_from_peers(block).await;
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use pyrsia_blockchain_network::crypto::hash_algorithm::HashDigest;
    use tokio::sync::mpsc;

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_payload() -> Result<(), String> {
        let (sender, _) = mpsc::channel(1);
        let keypair = identity::ed25519::Keypair::generate();
        let local_peer_id = identity::PublicKey::Ed25519(keypair.public()).to_peer_id();
        let client = Client {
            sender,
            local_peer_id,
        };

        let mut blockchain_service = BlockchainService::new(&keypair, client);
        let payload = vec![];

        blockchain_service
            .add_payload(payload, &identity::Keypair::Ed25519(keypair))
            .await;

        assert!(blockchain_service.blockchain.last_block().is_some());

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_add_block() -> Result<(), String> {
        let (sender, _) = mpsc::channel(1);
        let keypair = identity::ed25519::Keypair::generate();
        let local_peer_id = identity::PublicKey::Ed25519(keypair.public()).to_peer_id();
        let client = Client {
            sender,
            local_peer_id,
        };

        let mut blockchain_service = BlockchainService::new(&keypair, client);

        let last_block = blockchain_service.blockchain.last_block().unwrap();

        let block = Block::new(last_block.header.hash(), 1, vec![], &keypair);
        blockchain_service
            .add_block(1, Box::new(block.clone()))
            .await;

        let last_block = blockchain_service.blockchain.last_block().unwrap();
        assert_eq!(last_block, block);

        Ok(())
    }

    #[tokio::test(flavor = "multi_thread")]
    async fn test_notify_blockchain() -> Result<(), String> {
        let (sender, _) = mpsc::channel(1);
        let keypair = identity::ed25519::Keypair::generate();
        let local_peer_id = identity::PublicKey::Ed25519(keypair.public()).to_peer_id();
        let client = Client {
            sender,
            local_peer_id,
        };

        let mut blockchain_service = BlockchainService::new(&keypair, client);

        let block = Box::new(Block::new(HashDigest::new(b""), 1, vec![], &keypair));

        assert!(blockchain_service.broadcast_blockchain(block).await.is_ok());

        Ok(())
    }

    #[test]
    fn test_debug() -> Result<(), String> {
        let (sender, _) = mpsc::channel(1);
        let keypair = identity::ed25519::Keypair::generate();
        let local_peer_id = identity::PublicKey::Ed25519(keypair.public()).to_peer_id();
        let client = Client {
            sender,
            local_peer_id,
        };

        let blockchain_service = BlockchainService::new(&keypair, client);

        assert_ne!(
            format!("This is blockchain service {blockchain_service:?}"),
            "This is blockchain service"
        );
        Ok(())
    }

    #[test]
    fn test_blochchain_command_convert_to_u8() -> Result<(), String> {
        assert_eq!(1u8, BlockchainCommand::Broadcast as u8);

        assert_eq!(2u8, BlockchainCommand::PushFromPeer as u8);
  
        assert_eq!(3u8, BlockchainCommand::PullFromPeer as u8);
  
        assert_eq!(4u8, BlockchainCommand::QueryHighestBlockOrdinal as u8);
        
        Ok(())
    }

    #[test]
    fn test_blochchain_command_convert_from_u8() -> Result<(), String> {
        
        assert_eq!(BlockchainCommand::try_from(1u8).unwrap(), BlockchainCommand::Broadcast);
        
        assert_eq!(BlockchainCommand::try_from(2u8).unwrap(), BlockchainCommand::PushFromPeer);

        assert_eq!(BlockchainCommand::try_from(3u8).unwrap(), BlockchainCommand::PullFromPeer);

        assert_eq!(BlockchainCommand::try_from(4u8).unwrap(), BlockchainCommand::QueryHighestBlockOrdinal);

        assert!(BlockchainCommand::try_from(47u8).is_err());

        Ok(())
    }
}
