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
use crate::build_service::error::BuildError;
use crate::build_service::model::BuildResult;
use crate::build_service::service::BuildService;
use log::{debug, error, warn};
use std::sync::Arc;
use tokio::sync::{mpsc, Mutex};

#[derive(Debug)]
pub enum BuildEvent {
    BuildCleanup {
        build_id: String,
    },
    BuildFailed {
        build_id: String,
        build_error: BuildError,
    },
    BuildSucceeded(BuildResult),
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
            BuildEvent::BuildCleanup { build_id } => {
                self.build_service.lock().await.cleanup_build(&build_id);
            }
            BuildEvent::BuildFailed {
                build_id,
                build_error,
            } => {
                error!("{}", build_error.to_string());

                self.build_service.lock().await.cleanup_build(&build_id);
            }
            BuildEvent::BuildSucceeded(build_result) => {
                self.artifact_service
                    .lock()
                    .await
                    .handle_build_result(build_result)
                    .await;
            }
        }
    }
}
