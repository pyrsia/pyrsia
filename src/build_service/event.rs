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
use libp2p::PeerId;
use log::{debug, error, info, warn};
use tokio::sync::{mpsc, oneshot};

#[derive(Debug)]
pub enum BuildEvent {
    Failed {
        build_id: String,
        build_error: BuildError,
    },
    Status {
        build_id: String,
        sender: oneshot::Sender<Result<BuildStatus, BuildError>>,
    },
    Start {
        package_type: PackageType,
        package_specific_id: String,
        sender: oneshot::Sender<Result<String, BuildError>>,
        build_trigger: BuildTrigger,
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
        package_specific_id: &str,
    ) -> Result<String, BuildError> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::Start {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                sender,
                build_trigger: BuildTrigger::FromSource,
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
        requestor: PeerId,
        package_type: PackageType,
        package_specific_id: &str,
    ) -> Result<String, BuildError> {
        let (sender, receiver) = oneshot::channel();
        self.build_event_sender
            .send(BuildEvent::Start {
                package_type,
                package_specific_id: package_specific_id.to_owned(),
                sender,
                build_trigger: BuildTrigger::Verification(requestor),
            })
            .await
            .unwrap_or_else(|e| {
                error!("Error build_event_sender. {:#?}", e);
            });
        receiver
            .await
            .map_err(|e| BuildError::InitializationFailed(e.to_string()))?
    }

    pub async fn get_build_status(&self, build_id: &str) -> Result<BuildStatus, BuildError> {
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
        package_specific_id: &str,
        build_trigger: BuildTrigger,
        artifact_urls: Vec<String>,
    ) {
        self.build_event_sender
            .send(BuildEvent::Succeeded {
                build_id: build_id.to_owned(),
                package_type,
                package_specific_id: package_specific_id.to_owned(),
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
                build_trigger,
            } => {
                let result = self
                    .build_service
                    .start_build(package_type, package_specific_id, build_trigger)
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
                let result = self
                    .build_service
                    .get_build_status(&build_id)
                    .await
                    .map(|info| info.status);
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
                    BuildTrigger::Verification(leader_peer_id) => {
                        info!("Finished build with ID {}, triggered by authorative leader node {:?}: {:?}", build_id, leader_peer_id, build_result);
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

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use crate::build_service::model::BuildResultArtifact;
    use crate::util::test_util;
    use hyper::StatusCode;
    use std::path::PathBuf;

    #[tokio::test]
    async fn test_start_build() {
        let (client, mut receiver) = test_util::tests::create_build_event_client();

        let random_package_specific_id = test_util::tests::random_string(30);
        let cloned_random_package_specific_id = random_package_specific_id.clone();

        tokio::spawn(async move {
            client
                .start_build(PackageType::Docker, &random_package_specific_id)
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(BuildEvent::Start { package_type, package_specific_id, sender, build_trigger }) => {
                    assert_eq!(package_type, PackageType::Docker);
                    assert_eq!(package_specific_id, cloned_random_package_specific_id);
                    assert_eq!(build_trigger, BuildTrigger::FromSource);
                    let _ = sender.send(Ok(String::from("ok")));
                },
                _ => panic!("Command must match BuildEvent::Start")
            }
        }
    }

    #[tokio::test]
    async fn test_verify_build() {
        let (client, mut receiver) = test_util::tests::create_build_event_client();

        let requestor = PeerId::random();
        let random_package_specific_id = test_util::tests::random_string(30);
        let cloned_random_package_specific_id = random_package_specific_id.clone();

        tokio::spawn(async move {
            client
                .verify_build(requestor, PackageType::Docker, &random_package_specific_id)
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(BuildEvent::Start { package_type, package_specific_id, sender, build_trigger }) => {
                    assert_eq!(package_type, PackageType::Docker);
                    assert_eq!(package_specific_id, cloned_random_package_specific_id);
                    assert_eq!(build_trigger, BuildTrigger::Verification(requestor));
                    let _ = sender.send(Ok(String::from("ok")));
                },
                _ => panic!("Command must match BuildEvent::Start")
            }
        }
    }

    #[tokio::test]
    async fn test_get_build_status() {
        let (client, mut receiver) = test_util::tests::create_build_event_client();

        let random_build_id = test_util::tests::random_string(30);
        let cloned_random_build_id = random_build_id.clone();

        tokio::spawn(async move { client.get_build_status(&random_build_id).await });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(BuildEvent::Status { build_id, sender }) => {
                    assert_eq!(build_id, cloned_random_build_id);
                    let _ = sender.send(Ok(BuildStatus::Running));
                },
                _ => panic!("Command must match BuildEvent::Status")
            }
        }
    }

    #[tokio::test]
    async fn test_build_succeeded() {
        let (client, mut receiver) = test_util::tests::create_build_event_client();

        let random_build_id = test_util::tests::random_string(30);
        let cloned_random_build_id = random_build_id.clone();
        let random_package_specific_id = test_util::tests::random_string(30);
        let cloned_random_package_specific_id = random_package_specific_id.clone();
        let artifact_urls = vec![String::from("url_1"), String::from("url_2")];
        let cloned_artifact_urls = artifact_urls.clone();

        tokio::spawn(async move {
            client
                .build_succeeded(
                    &random_build_id,
                    PackageType::Docker,
                    &random_package_specific_id,
                    BuildTrigger::FromSource,
                    artifact_urls,
                )
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(BuildEvent::Succeeded { build_id, package_type, package_specific_id, build_trigger, artifact_urls }) => {
                    assert_eq!(build_id, cloned_random_build_id);
                    assert_eq!(package_type, PackageType::Docker);
                    assert_eq!(package_specific_id, cloned_random_package_specific_id);
                    assert_eq!(build_trigger, BuildTrigger::FromSource);
                    assert_eq!(artifact_urls, cloned_artifact_urls);
                },
                _ => panic!("Command must match BuildEvent::Succeeded")
            }
        }
    }

    #[tokio::test]
    async fn test_build_failed() {
        let (client, mut receiver) = test_util::tests::create_build_event_client();

        let random_build_id = test_util::tests::random_string(30);
        let cloned_random_build_id = random_build_id.clone();

        tokio::spawn(async move {
            client
                .build_failed(
                    &random_build_id,
                    BuildError::MappingServiceEndpointFailure(StatusCode::BAD_REQUEST),
                )
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(BuildEvent::Failed { build_id, build_error }) => {
                    assert_eq!(build_id, cloned_random_build_id);
                    match build_error {
                        BuildError::MappingServiceEndpointFailure(status_code) => assert_eq!(status_code, StatusCode::BAD_REQUEST),
                        e => panic!("Invalid Error encountered: {:?}", e),
                    }
                },
                _ => panic!("Command must match BuildEvent::Failed")
            }
        }
    }

    #[tokio::test]
    async fn test_build_result() {
        let (client, mut receiver) = test_util::tests::create_build_event_client();

        let random_build_id = test_util::tests::random_string(30);
        let cloned_random_build_id = random_build_id.clone();
        let random_package_specific_id = test_util::tests::random_string(30);
        let cloned_random_package_specific_id = random_package_specific_id.clone();
        let artifacts = vec![BuildResultArtifact {
            artifact_specific_id: "artifact_specific_id_1".to_owned(),
            artifact_location: PathBuf::from("/tmp/build/artifact1.bin"),
            artifact_hash: "artifact_hash_1".to_owned(),
        }];
        let cloned_artifacts = artifacts.clone();

        tokio::spawn(async move {
            client
                .build_result(
                    &random_build_id,
                    BuildTrigger::FromSource,
                    BuildResult {
                        package_type: PackageType::Docker,
                        package_specific_id: random_package_specific_id.to_owned(),
                        artifacts,
                    },
                )
                .await
        });

        tokio::select! {
            command = receiver.recv() => match command {
                Some(BuildEvent::Result { build_id, build_trigger, build_result }) => {
                    assert_eq!(build_id, cloned_random_build_id);
                    assert_eq!(build_trigger, BuildTrigger::FromSource);
                    assert_eq!(build_result.package_type, PackageType::Docker);
                    assert_eq!(build_result.package_specific_id, cloned_random_package_specific_id);
                    assert_eq!(build_result.artifacts, cloned_artifacts);
                },
                _ => panic!("Command must match BuildEvent::Succeeded")
            }
        }
    }
}
