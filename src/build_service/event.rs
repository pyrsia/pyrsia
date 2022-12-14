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
use crate::build_service::error::BuildError;
use crate::build_service::model::{BuildResult, BuildStatus, BuildTrigger};
use crate::build_service::service::BuildService;
use crate::verification_service::service::VerificationService;
use log::{debug, error, warn};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum BuildEvent {
    Failed {
        build_id: String,
        build_error: BuildError,
    },
    Status {
        build_id: String,
        sender: oneshot::Sender<Result<String, BuildError>>,
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

    pub async fn get_build_status(&self, build_id: &str) -> Result<String, BuildError> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::Status {
                build_id: String::from(build_id),
                sender,
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver
            .await
            .map_err(|e| BuildError::BuildStatusFailed(e.to_string()))?
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
}

pub struct BuildEventLoop {
    artifact_service: ArtifactService,
    build_service: BuildService,
    verification_service: VerificationService,
    build_event_receiver: mpsc::Receiver<BuildEvent>,
}

impl BuildEventLoop {
    pub fn new(
        artifact_service: ArtifactService,
        build_service: BuildService,
        verification_service: VerificationService,
        build_event_receiver: mpsc::Receiver<BuildEvent>,
    ) -> Self {
        Self {
            artifact_service,
            build_service,
            verification_service,
            build_event_receiver,
        }
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
            BuildEvent::Status { build_id, sender } => {
                let result = match self.build_service.get_build_status(&build_id).await {
                    Ok(build_info) => {
                        let build_status = match build_info.status {
                            BuildStatus::Running => String::from("RUNNING"),
                            BuildStatus::Success { .. } => String::from("SUCCESS"),
                            BuildStatus::Failure(message) => {
                                format!("FAILED - (Error: {})", message)
                            }
                        };
                        Ok(build_status)
                    }
                    Err(build_error) => Err(build_error),
                };
                sender.send(result).unwrap_or_else(|build_error| {
                    error!("build error. {:#?}", build_error);
                });
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
        }
    }
}
