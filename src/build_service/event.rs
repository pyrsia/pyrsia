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
use crate::build_service::model::{BuildInfo, BuildResult};
use crate::build_service::service::BuildService;
use log::{debug, error, warn};
use std::sync::Arc;
use tokio::sync::{mpsc, oneshot, Mutex};

#[derive(Debug)]
pub enum BuildEvent {
    CleanUp {
        build_id: String,
    },
    Failed {
        build_id: String,
        build_error: BuildError,
    },
    Start {
        package_type: PackageType,
        package_specific_id: String,
        sender: oneshot::Sender<Result<BuildInfo, BuildError>>,
    },
    Succeeded(BuildResult),
}

#[derive(Clone)]
pub struct BuildEventClient {
    build_event_sender: mpsc::Sender<BuildEvent>,
}

impl BuildEventClient {
    pub fn new(build_event_sender: mpsc::Sender<BuildEvent>) -> Self {
        Self { build_event_sender }
    }

    pub async fn clean_up(&self, build_id: String) {
        let _ = self
            .build_event_sender
            .send(BuildEvent::CleanUp { build_id })
            .await;
    }

    pub async fn send_build_success(&self, build_result: BuildResult) {
        let _ = self
            .build_event_sender
            .send(BuildEvent::Succeeded(build_result))
            .await;
    }

    pub async fn send_build_failure(&self, build_id: String, build_error: BuildError) {
        let _ = self
            .build_event_sender
            .send(BuildEvent::Failed {
                build_id,
                build_error,
            })
            .await;
    }

    pub async fn start_build(
        &self,
        package_type: PackageType,
        package_specific_id: String,
    ) -> Result<BuildInfo, BuildError> {
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
    ) -> Result<BuildInfo, BuildError> {
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
}

pub struct BuildEventLoop {
    artifact_service: Arc<Mutex<ArtifactService>>,
    build_service: Arc<Mutex<BuildService>>,
    build_event_receiver: mpsc::Receiver<BuildEvent>,
}

impl BuildEventLoop {
    pub fn new(
        artifact_service: Arc<Mutex<ArtifactService>>,
        build_service: Arc<Mutex<BuildService>>,
        build_event_receiver: mpsc::Receiver<BuildEvent>,
    ) -> Self {
        Self {
            artifact_service,
            build_service,
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
            BuildEvent::CleanUp { build_id } => {
                self.build_service.lock().await.clean_up_build(&build_id);
            }
            BuildEvent::Failed {
                build_id,
                build_error,
            } => {
                error!("{}", build_error.to_string());

                self.build_service.lock().await.clean_up_build(&build_id);
            }
            BuildEvent::Start {
                package_type,
                package_specific_id,
                sender,
            } => {
                let result = self
                    .build_service
                    .lock()
                    .await
                    .start_build(package_type, package_specific_id)
                    .await;
                let _ = sender.send(result);
            }
            BuildEvent::Succeeded(build_result) => {
                self.artifact_service
                    .lock()
                    .await
                    .handle_build_result(build_result)
                    .await;
            }
        }
    }
}
