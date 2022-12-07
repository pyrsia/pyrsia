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

use crate::artifact_service::model::PackageType;
use crate::artifact_service::service::ArtifactService;
use crate::blockchain_service::service::{BlockchainCommand, BlockchainService};
use crate::build_service::error::BuildError;
use crate::build_service::model::{BuildResult, BuildTrigger};
use crate::build_service::service::BuildService;
use crate::network::blockchain_protocol::BlockchainResponse;
use crate::verification_service::service::VerificationService;
use bincode::{deserialize, serialize};
use libp2p::request_response::ResponseChannel;
use libp2p::PeerId;
use log::{debug, error, warn};
use pyrsia_blockchain_network::error::BlockchainError;
use pyrsia_blockchain_network::structures::block::Block;
use pyrsia_blockchain_network::structures::header::Ordinal;
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum BuildEvent {
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
        channel: ResponseChannel<BlockchainResponse>,
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    HandlePullBlocks {
        start: Ordinal,
        end: Ordinal,
        channel: ResponseChannel<BlockchainResponse>,
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    HandleQueryBlockOrdinal {
        channel: ResponseChannel<BlockchainResponse>,
        sender: oneshot::Sender<anyhow::Result<()>>,
    },
    Failed {
        build_id: String,
        build_error: BuildError,
    },
    Start {
        package_type: PackageType,
        package_specific_id: String,
        sender: oneshot::Sender<Result<String, BuildError>>,
    },
    Succeeded {
        build_id: String,
        package_type: PackageType,
        package_specific_id: String,
        build_trigger: BuildTrigger,
        artifact_urls: Vec<String>,
    },
    Result {
        build_id: String,
        build_trigger: BuildTrigger,
        build_result: BuildResult,
    },
    Verify {
        package_type: PackageType,
        package_specific_id: String,
        sender: oneshot::Sender<Result<String, BuildError>>,
    },
}

#[derive(Clone)]
pub struct BuildEventClient {
    build_event_sender: mpsc::Sender<BuildEvent>,
}

impl BuildEventClient {
    pub fn new(build_event_sender: mpsc::Sender<BuildEvent>) -> Self {
        Self { build_event_sender }
    }

    pub async fn start_build(
        &self,
        package_type: PackageType,
        package_specific_id: String,
    ) -> Result<String, BuildError> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::Start {
                package_type,
                package_specific_id,
                sender,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver
            .await
            .map_err(|e| BuildError::InitializationFailed(e.to_string()))?
    }

    pub async fn verify_build(
        &self,
        package_type: PackageType,
        package_specific_id: String,
        _package_specific_artifact_id: String,
        _artifact_hash: String,
    ) -> Result<String, BuildError> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::Verify {
                package_type,
                package_specific_id,
                sender,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver
            .await
            .map_err(|e| BuildError::InitializationFailed(e.to_string()))?
    }

    pub async fn build_succeeded(
        &self,
        build_id: &str,
        package_type: PackageType,
        package_specific_id: String,
        build_trigger: BuildTrigger,
        artifact_urls: Vec<String>,
    ) {
        self.build_event_sender
            .send(BuildEvent::Succeeded {
                build_id: build_id.to_owned(),
                package_type,
                package_specific_id,
                build_trigger,
                artifact_urls,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
    }

    pub async fn build_failed(&self, build_id: &str, build_error: BuildError) {
        self.build_event_sender
            .send(BuildEvent::Failed {
                build_id: build_id.to_owned(),
                build_error,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
    }

    pub async fn build_result(
        &self,
        build_id: &str,
        build_trigger: BuildTrigger,
        build_result: BuildResult,
    ) {
        self.build_event_sender
            .send(BuildEvent::Result {
                build_id: build_id.to_owned(),
                build_trigger,
                build_result,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
    }

    pub async fn add_block(&self, payload: Vec<u8>) -> Result<(), BlockchainError> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::AddBlock { payload, sender })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    pub async fn pull_blocks_from_peer(
        &self,
        peer_id: &PeerId,
    ) -> Result<Ordinal, BlockchainError> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::PullBlocksFromPeer {
                peer_id: *peer_id,
                sender,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    pub async fn pull_blocks_local(
        &self,
        start: Ordinal,
        end: Ordinal,
    ) -> Result<Vec<Block>, BlockchainError> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::PullBlocksLocal { start, end, sender })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    pub async fn handle_incoming_blockchain_command(
        &self,
        data: Vec<u8>,
        channel: ResponseChannel<BlockchainResponse>,
    ) -> anyhow::Result<()> {
        debug!("Handling request blockchain");
        match BlockchainCommand::try_from(data[0])? {
            BlockchainCommand::Broadcast => {
                debug!("Blockchain receives BlockchainCommand::Broadcast");
                let block_ordinal: Ordinal = deserialize(&data[1..17])?;
                let block: Block = deserialize(&data[17..])?;
                self.handle_broadcast_blockchain(block_ordinal, block, channel)
                    .await
            }
            BlockchainCommand::PullFromPeer => {
                debug!("Blockchain receives BlockchainCommand::PullFromPeer");
                let start: Ordinal = deserialize(&data[1..17])?;
                let end: Ordinal = deserialize(&data[17..])?;
                self.handle_pull_blockchain_from_peer(start, end, channel)
                    .await
            }
            BlockchainCommand::QueryHighestBlockOrdinal => {
                debug!("Blockchain receives BlockchainCommand::QueryHighestBlockOrdinal");
                self.handle_query_block_ordinal_from_peer(channel).await
            }
            _ => {
                debug!("Blockchain receives other command");
                todo!()
            }
        }
    }

    async fn handle_broadcast_blockchain(
        &self,
        block_ordinal: Ordinal,
        block: Block,
        channel: ResponseChannel<BlockchainResponse>,
    ) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::HandleBlockBroadcast {
                block_ordinal,
                block: Box::new(block),
                channel,
                sender,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    async fn handle_pull_blockchain_from_peer(
        &self,
        start: Ordinal,
        end: Ordinal,
        channel: ResponseChannel<BlockchainResponse>,
    ) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::HandlePullBlocks {
                start,
                end,
                channel,
                sender,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }

    async fn handle_query_block_ordinal_from_peer(
        &self,
        channel: ResponseChannel<BlockchainResponse>,
    ) -> anyhow::Result<()> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::HandleQueryBlockOrdinal { channel, sender })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver.await.map_err(BlockchainError::ChannelClosed)?
    }
}

pub struct BuildEventLoop {
    artifact_service: ArtifactService,
    blockchain_service: BlockchainService,
    build_service: BuildService,
    verification_service: VerificationService,
    build_event_receiver: mpsc::Receiver<BuildEvent>,
}

impl BuildEventLoop {
    pub fn new(
        artifact_service: ArtifactService,
        blockchain_service: BlockchainService,
        build_service: BuildService,
        verification_service: VerificationService,
        build_event_receiver: mpsc::Receiver<BuildEvent>,
    ) -> Self {
        Self {
            artifact_service,
            blockchain_service,
            build_service,
            verification_service,
            build_event_receiver,
        }
    }

    pub fn blockchain_service(&self) -> &BlockchainService {
        &self.blockchain_service
    }

    pub async fn run(mut self) {
        loop {
            match self.build_event_receiver.recv().await {
                Some(build_event) => {
                    self.handle_build_event(build_event).await;
                }
                None => {
                    warn!("Got empty build event");
                    return;
                }
            }
        }
    }

    async fn handle_build_event(&mut self, build_event: BuildEvent) {
        debug!("Handle BuildEvent: {:?}", build_event);
        match build_event {
            BuildEvent::Start {
                package_type,
                package_specific_id,
                sender,
            } => {
                let result = self
                    .build_service
                    .start_build(package_type, package_specific_id, BuildTrigger::FromSource)
                    .await;
                sender.send(result).unwrap_or_else(|e| {
                    error!("build error. {:#?}", e);
                });
            }
            BuildEvent::Verify {
                package_type,
                package_specific_id,
                sender,
            } => {
                let result = self
                    .build_service
                    .start_build(
                        package_type,
                        package_specific_id,
                        BuildTrigger::Verification,
                    )
                    .await;
                sender.send(result).unwrap_or_else(|e| {
                    error!("build error. {:#?}", e);
                });
            }
            BuildEvent::Failed {
                build_id,
                build_error,
            } => {
                error!("{}", build_error.to_string());

                self.verification_service
                    .handle_build_failed(&build_id, build_error);
            }
            BuildEvent::Succeeded {
                build_id,
                package_type,
                package_specific_id,
                build_trigger,
                artifact_urls,
            } => {
                self.build_service
                    .handle_successful_build(
                        &build_id,
                        package_type,
                        package_specific_id,
                        build_trigger,
                        artifact_urls,
                    )
                    .await;
            }
            BuildEvent::Result {
                build_id,
                build_trigger,
                build_result,
            } => {
                if let Err(error) = match build_trigger {
                    BuildTrigger::FromSource => {
                        self.artifact_service
                            .handle_build_result(&build_id, build_result)
                            .await
                    }
                    BuildTrigger::Verification => {
                        self.verification_service
                            .handle_build_result(&build_id, build_result)
                            .await
                    }
                } {
                    error!(
                        "Failed to handle build result for build with ID {}: {:?}",
                        build_id, error
                    )
                }

                self.build_service.clean_up_build(&build_id);
            }
            BuildEvent::AddBlock { payload, sender } => {
                let result = self.blockchain_service.add_payload(payload).await;
                sender.send(result).unwrap_or_else(|e| {
                    error!("add block error. {:#?}", e);
                });
            }
            BuildEvent::PullBlocksFromPeer { peer_id, sender } => {
                let result = self
                    .blockchain_service
                    .init_pull_from_others(&peer_id)
                    .await;
                sender.send(result).unwrap_or_else(|e| {
                    error!("pull blocks from peer error. {:#?}", e);
                });
            }
            BuildEvent::PullBlocksLocal { start, end, sender } => {
                debug!("Handling pull blocks from {:?} to {:?} ", start, end);

                let result = self.blockchain_service.pull_blocks(start, end).await;
                sender.send(result).unwrap_or_else(|e| {
                    error!("pull blocks local error. {:#?}", e);
                });
            }
            BuildEvent::HandleBlockBroadcast {
                block_ordinal,
                block,
                channel,
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
                    let response_data = vec![0u8];

                    let result = self
                        .artifact_service
                        .p2p_client
                        .respond_blockchain(response_data, channel)
                        .await;
                    sender.send(result).unwrap_or_else(|e| {
                        error!("block broadcast error. {:#?}", e);
                    });
                }
            }
            BuildEvent::HandlePullBlocks {
                start,
                end,
                sender,
                channel,
            } => {
                debug!("Handling pull blocks from {:?} to {:?} ", start, end);

                match self.blockchain_service.pull_blocks(start, end).await {
                    Ok(v) => {
                        let result = self
                            .artifact_service
                            .p2p_client
                            .respond_blockchain(serialize(&v).unwrap(), channel)
                            .await;
                        sender.send(result).unwrap_or_else(|e| {
                            error!("block broadcast error. {:#?}", e);
                        });
                    }
                    Err(e) => {
                        sender.send(Err(e.into())).unwrap_or_else(|e| {
                            error!("block broadcast error. {:#?}", e);
                        });
                    }
                }
            }
            BuildEvent::HandleQueryBlockOrdinal { channel, sender } => {
                debug!("Handling query block ordinal");

                match self.blockchain_service.query_last_block().await {
                    Some(latest_block) => {
                        let latest_ordinal = latest_block.header.ordinal;
                        let result = self
                            .artifact_service
                            .p2p_client
                            .respond_blockchain(serialize(&latest_ordinal).unwrap(), channel)
                            .await;
                        sender.send(result).unwrap_or_else(|e| {
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
