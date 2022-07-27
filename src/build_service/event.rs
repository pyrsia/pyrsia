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
use crate::build_service::model::{BuildResult, BuildTrigger};
use crate::build_service::service::BuildService;
use crate::verification_service::service::VerificationService;
use log::{debug, error, warn};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug)]
pub enum BuildEvent {
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
        let _ = self
            .build_event_sender
            .send(BuildEvent::Start {
                package_type,
                package_specific_id,
                sender,
            })
            .await;
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
        let _ = self
            .build_event_sender
            .send(BuildEvent::Verify {
                package_type,
                package_specific_id,
                sender,
            })
            .await;
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
        let _ = self
            .build_event_sender
            .send(BuildEvent::Succeeded {
                build_id: build_id.to_owned(),
                package_type,
                package_specific_id,
                build_trigger,
                artifact_urls,
            })
            .await;
    }

    pub async fn build_failed(&self, build_id: &str, build_error: BuildError) {
        let _ = self
            .build_event_sender
            .send(BuildEvent::Failed {
                build_id: build_id.to_owned(),
                build_error,
            })
            .await;
    }

    pub async fn build_result(
        &self,
        build_id: &str,
        build_trigger: BuildTrigger,
        build_result: BuildResult,
    ) {
        let _ = self
            .build_event_sender
            .send(BuildEvent::Result {
                build_id: build_id.to_owned(),
                build_trigger,
                build_result,
            })
            .await;
    }
}

pub struct BuildEventLoop {
    artifact_service: Arc<Mutex<ArtifactService>>,
    build_service: Arc<Mutex<BuildService>>,
    verification_service: Arc<Mutex<VerificationService>>,
    build_event_receiver: mpsc::Receiver<BuildEvent>,
}

impl BuildEventLoop {
    pub fn new(
        artifact_service: Arc<Mutex<ArtifactService>>,
        build_service: Arc<Mutex<BuildService>>,
        verification_service: Arc<Mutex<VerificationService>>,
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
            tokio::select! {
                next_build_event = self.build_event_receiver.recv() => match next_build_event {
                    Some(build_event) => {
                        self.handle_build_event(build_event).await;
                    },
                    None => {
                        warn!("Got empty build event");
                        return;
                    }
                }
            }
        }
    }

    async fn handle_build_event(&self, build_event: BuildEvent) {
        debug!("Handle BuildEvent: {:?}", build_event);
        match build_event {
            BuildEvent::Start {
                package_type,
                package_specific_id,
                sender,
            } => {
                let result = self
                    .build_service
                    .lock()
                    .await
                    .start_build(package_type, package_specific_id, BuildTrigger::FromSource)
                    .await;
                let _ = sender.send(result);
            }
            BuildEvent::Verify {
                package_type,
                package_specific_id,
                sender,
            } => {
                let result = self
                    .build_service
                    .lock()
                    .await
                    .start_build(
                        package_type,
                        package_specific_id,
                        BuildTrigger::Verification,
                    )
                    .await;
                let _ = sender.send(result);
            }
            BuildEvent::Failed {
                build_id,
                build_error,
            } => {
                error!("{}", build_error.to_string());

                self.verification_service
                    .lock()
                    .await
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
                    .lock()
                    .await
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
                            .lock()
                            .await
                            .handle_build_result(&build_id, build_result)
                            .await
                    }
                    BuildTrigger::Verification => {
                        self.verification_service
                            .lock()
                            .await
                            .handle_build_result(&build_id, build_result)
                            .await
                    }
                } {
                    error!(
                        "Failed to handle build result for build with ID {}: {:?}",
                        build_id, error
                    )
                }

                self.build_service.lock().await.clean_up_build(&build_id);
            }
        }
    }
}
