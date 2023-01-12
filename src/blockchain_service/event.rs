/*
   Copyright 2022 JFrog Ltd

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

use crate::artifact_service::service::ArtifactService;
use crate::blockchain_service::service::BlockchainService;
use libp2p::PeerId;
use log::{debug, error, warn};
use pyrsia_blockchain_network::error::BlockchainError;
use pyrsia_blockchain_network::structures::block::Block;
use pyrsia_blockchain_network::structures::header::Ordinal;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum BlockchainEvent {
    AddBlock {
        payload: Vec<u8>,
        sender: oneshot::Sender<Result<(), BlockchainError>>,
    },
    PullBlocksFromPeer {
        peer_id: PeerId,
        sender: oneshot::Sender<Result<Ordinal, BlockchainError>>,
    },
    PullBlocksLocal {
        start: Ordinal,
        end: Ordinal,
        sender: oneshot::Sender<Result<Vec<Block>, BlockchainError>>,
    },
    HandleBlockBroadcast {
        block_ordinal: Ordinal,
        block: Box<Block>,
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    HandlePullBlocks {
        start: Ordinal,
        end: Ordinal,
        sender: oneshot::Sender<anyhow::Result<Vec<Block>>>,
    },
    HandleQueryBlockOrdinal {
        sender: oneshot::Sender<anyhow::Result<Ordinal>>,
    },
}

#[derive(Clone)]
pub struct BlockchainEventClient {
    blockchain_event_sender: mpsc::Sender<BlockchainEvent>,
}

impl BlockchainEventClient {
    pub fn new(blockchain_event_sender: mpsc::Sender<BlockchainEvent>) -> Self {
        Self {
            blockchain_event_sender,
        }
    }

    pub async fn add_block(&self, payload: Vec<u8>) -> Result<(), BlockchainError> {
        let (sender, receiver) = oneshot::channel();
        self.blockchain_event_sender
            .send(BlockchainEvent::AddBlock { payload, sender })
            .await
            .unwrap_or_else(|e| {
                error!("Error blockchain_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    pub async fn pull_blocks_from_peer(
        &self,
        peer_id: &PeerId,
    ) -> Result<Ordinal, BlockchainError> {
        let (sender, receiver) = oneshot::channel();
        self.blockchain_event_sender
            .send(BlockchainEvent::PullBlocksFromPeer {
                peer_id: *peer_id,
                sender,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error blockchain_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    pub async fn pull_blocks_local(
        &self,
        start: Ordinal,
        end: Ordinal,
    ) -> Result<Vec<Block>, BlockchainError> {
        let (sender, receiver) = oneshot::channel();
        self.blockchain_event_sender
            .send(BlockchainEvent::PullBlocksLocal { start, end, sender })
            .await
            .unwrap_or_else(|e| {
                error!("Error blockchain_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    pub async fn handle_broadcast_blockchain(
        &self,
        block_ordinal: Ordinal,
        block: Block,
    ) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.blockchain_event_sender
            .send(BlockchainEvent::HandleBlockBroadcast {
                block_ordinal,
                block: Box::new(block),
                sender,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error blockchain_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    pub async fn handle_pull_blockchain_from_peer(
        &self,
        start: Ordinal,
        end: Ordinal,
    ) -> anyhow::Result<Vec<Block>> {
        let (sender, receiver) = oneshot::channel();
        self.blockchain_event_sender
            .send(BlockchainEvent::HandlePullBlocks { start, end, sender })
            .await
            .unwrap_or_else(|e| {
                error!("Error blockchain_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    pub async fn handle_query_block_ordinal_from_peer(&self) -> anyhow::Result<Ordinal> {
        let (sender, receiver) = oneshot::channel();
        self.blockchain_event_sender
            .send(BlockchainEvent::HandleQueryBlockOrdinal { sender })
            .await
            .unwrap_or_else(|e| {
                error!("Error blockchain_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }
}

pub struct BlockchainEventLoop {
    artifact_service: ArtifactService,
    blockchain_service: BlockchainService,
    blockchain_event_receiver: mpsc::Receiver<BlockchainEvent>,
}

impl BlockchainEventLoop {
    pub fn new(
        artifact_service: ArtifactService,
        blockchain_service: BlockchainService,
        blockchain_event_receiver: mpsc::Receiver<BlockchainEvent>,
    ) -> Self {
        Self {
            artifact_service,
            blockchain_service,
            blockchain_event_receiver,
        }
    }

    pub fn blockchain_service(&self) -> &BlockchainService {
        &self.blockchain_service
    }

    pub async fn run(mut self) {
        loop {
            match self.blockchain_event_receiver.recv().await {
                Some(blockchain_event) => {
                    self.handle_blockchain_event(blockchain_event).await;
                }
                None => {
                    warn!("Got empty build event");
                    return;
                }
            }
        }
    }

    async fn handle_blockchain_event(&mut self, blockchain_event: BlockchainEvent) {
        debug!("Handle BlockchainEvent: {:?}", blockchain_event);
        match blockchain_event {
            BlockchainEvent::AddBlock { payload, sender } => {
                let result = self.blockchain_service.add_payload(payload).await;
                sender.send(result).unwrap_or_else(|e| {
                    error!("add block error. {:#?}", e);
                });
            }
            BlockchainEvent::PullBlocksFromPeer { peer_id, sender } => {
                let result = self
                    .blockchain_service
                    .init_pull_from_others(&peer_id)
                    .await;
                sender.send(result).unwrap_or_else(|e| {
                    error!("pull blocks from peer error. {:#?}", e);
                });
            }
            BlockchainEvent::PullBlocksLocal { start, end, sender } => {
                debug!("Handling pull blocks from {:?} to {:?} ", start, end);

                let result = self.blockchain_service.pull_blocks(start, end).await;
                sender.send(result).unwrap_or_else(|e| {
                    error!("pull blocks local error. {:#?}", e);
                });
            }
            BlockchainEvent::HandleBlockBroadcast {
                block_ordinal,
                block,
                sender,
            } => {
                debug!("Handling broadcast blocks");

                let payloads = block.fetch_payload();
                if let Err(e) = self
                    .blockchain_service
                    .add_block(block_ordinal, block)
                    .await
                {
                    sender.send(Err(e.into())).unwrap_or_else(|e| {
                        error!("block broadcast error. {:#?}", e);
                    });
                } else if let Err(e) = self.artifact_service.handle_block_added(payloads).await {
                    sender.send(Err(e)).unwrap_or_else(|e| {
                        error!("block broadcast error. {:#?}", e);
                    });
                } else {
                    sender.send(Ok(())).unwrap_or_else(|e| {
                        error!("block broadcast error. {:#?}", e);
                    });
                }
            }
            BlockchainEvent::HandlePullBlocks { start, end, sender } => {
                debug!("Handling pull blocks from {:?} to {:?} ", start, end);

                let result = self.blockchain_service.pull_blocks(start, end).await;
                sender
                    .send(result.map_err(|e| e.into()))
                    .unwrap_or_else(|e| {
                        error!("block broadcast error. {:#?}", e);
                    });
            }
            BlockchainEvent::HandleQueryBlockOrdinal { sender } => {
                debug!("Handling query block ordinal");

                match self.blockchain_service.query_last_block().await {
                    Some(latest_block) => {
                        let highest_ordinal = latest_block.header.ordinal;
                        sender.send(Ok(highest_ordinal)).unwrap_or_else(|e| {
                            error!("block broadcast error. {:#?}", e);
                        });
                    }
                    None => {
                        sender
                            .send(Err(BlockchainError::InvalidBlockchainLength(0).into()))
                            .unwrap_or_else(|e| {
                                error!("block broadcast error. {:#?}", e);
                            });
                    }
                }
            }
        }
    }
}
