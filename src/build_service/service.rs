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

use super::error::BuildError;
use super::event::BuildEventClient;
use super::mapping::service::MappingService;
use super::model::{BuildResult, BuildResultArtifact, BuildStatus, BuildTrigger};
use super::pipeline::service::PipelineService;
use crate::artifact_service::model::PackageType;
use bytes::Buf;
use log::{debug, error, warn};
use multihash::Hasher;
use std::fs;
use std::io;
use std::path::{Path, PathBuf};

/// The build service is a component used by authorized nodes only. It is
/// the entrypoint to the authorized node's build pipeline infrastructure.
pub struct BuildService {
    repository_path: PathBuf,
    build_event_client: BuildEventClient,
    mapping_service: MappingService,
    pipeline_service: PipelineService,
}

impl BuildService {
    pub fn new<P: AsRef<Path>>(
        repository_path: P,
        build_event_client: BuildEventClient,
        mapping_service_endpoint: &str,
        pipeline_service_endpoint: &str,
    ) -> Result<Self, anyhow::Error> {
        let repository_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        Ok(BuildService {
            repository_path,
            build_event_client,
            mapping_service: MappingService::new(mapping_service_endpoint),
            pipeline_service: PipelineService::new(pipeline_service_endpoint),
        })
    }

    /// Starts a new build for the specified package.
    pub async fn start_build(
        &self,
        package_type: PackageType,
        package_specific_id: String,
        build_trigger: BuildTrigger,
    ) -> Result<String, BuildError> {
        debug!(
            "Starting build for package type {:?} and specific ID {:}",
            package_type, package_specific_id
        );

        let mapping_info = self
            .mapping_service
            .get_mapping(package_type, &package_specific_id)
            .await?;

        let build_id = self.pipeline_service.start_build(mapping_info).await?;
        let mut interval = tokio::time::interval(tokio::time::Duration::from_secs(5));
        let pipeline_service = self.pipeline_service.clone();
        let build_event_client = self.build_event_client.clone();
        let build_id_result = build_id.clone();
        tokio::spawn(async move {
            loop {
                interval.tick().await;

                match pipeline_service.get_build_status(&build_id).await {
                    Ok(latest_build_info) => {
                        debug!("Updated build info: {:?}", &latest_build_info);

                        match latest_build_info.status {
                            BuildStatus::Running => continue,
                            BuildStatus::Success { artifact_urls } => {
                                build_event_client
                                    .build_succeeded(
                                        &build_id,
                                        package_type,
                                        package_specific_id,
                                        build_trigger,
                                        artifact_urls,
                                    )
                                    .await;
                                break;
                            }
                            BuildStatus::Failure(build_error) => {
                                build_event_client
                                    .build_failed(
                                        &build_id,
                                        BuildError::Failure(latest_build_info.id, build_error),
                                    )
                                    .await;
                                break;
                            }
                        }
                    }
                    Err(build_error) => {
                        build_event_client
                            .build_failed(&build_id, build_error)
                            .await;
                        break;
                    }
                }
            }
        });

        Ok(build_id_result)
    }

    pub async fn handle_successful_build(
        &self,
        build_id: &str,
        package_type: PackageType,
        package_specific_id: String,
        build_trigger: BuildTrigger,
        artifact_urls: Vec<String>,
    ) {
        let build_path = &self.get_build_path(build_id);
        if let Err(build_error) = fs::create_dir_all(build_path)
            .map_err(|e| BuildError::Failure(build_id.to_owned(), e.to_string()))
        {
            self.build_event_client
                .build_failed(build_id, build_error)
                .await;
        } else {
            match self
                .process_artifact_urls(
                    build_id,
                    package_type,
                    package_specific_id,
                    artifact_urls,
                    build_path,
                )
                .await
            {
                Ok(build_result) => {
                    debug!("Successfully handled build {}.", build_id);

                    self.build_event_client
                        .build_result(build_id, build_trigger, build_result)
                        .await;
                }
                Err(build_error) => {
                    error!("Failed to handle build {}: {:?}", build_id, build_error);
                    self.build_event_client
                        .build_failed(build_id, build_error)
                        .await;
                    self.clean_up_build(build_id);
                }
            }
        }
    }

    async fn process_artifact_urls(
        &self,
        build_id: &str,
        package_type: PackageType,
        package_specific_id: String,
        artifact_urls: Vec<String>,
        build_path: &Path,
    ) -> Result<BuildResult, BuildError> {
        let mut artifacts = vec![];

        for artifact_url in artifact_urls {
            debug!("Handle built artifact with url: {}", artifact_url);
            let artifact = self
                .pipeline_service
                .download_artifact(&artifact_url)
                .await?;
            let (artifact_location, artifact_hash) = hash_and_store_data(build_path, &artifact)
                .map_err(|e| BuildError::Failure(build_id.to_owned(), e.to_string()))?;

            let artifact_specific_id = match package_type {
                PackageType::Docker => {
                    if artifact_url.ends_with("/manifest") {
                        package_specific_id.to_owned()
                    } else {
                        let artifact_filename = match artifact_url.rfind('/') {
                            Some(position_slash) => {
                                String::from(&artifact_url[position_slash + 1..])
                            }
                            None => artifact_url,
                        };
                        match artifact_filename.rfind('.') {
                            Some(position_dot) => String::from(&artifact_filename[..position_dot]),
                            None => artifact_filename,
                        }
                    }
                }
                PackageType::Maven2 => {
                    let prefix = package_specific_id.replace(':', "/");
                    let artifact_filename = match artifact_url.rfind('/') {
                        Some(position) => String::from(&artifact_url[position + 1..]),
                        None => artifact_url,
                    };
                    format!("{}/{}", prefix, artifact_filename)
                }
            };

            debug!(
                "Handled artifact into artifact specific id {}",
                artifact_specific_id
            );

            artifacts.push(BuildResultArtifact {
                artifact_specific_id,
                artifact_location,
                artifact_hash,
            });
        }

        debug!(
            "Handling build {} resulted in {} artifacts.",
            build_id,
            artifacts.len()
        );

        Ok(BuildResult {
            package_type,
            package_specific_id,
            artifacts,
        })
    }

    pub fn clean_up_build(&self, build_id: &str) {
        let build_path = self.get_build_path(build_id);
        if let Err(error) = fs::remove_dir_all(&build_path) {
            warn!(
                "Could not remove temporary build directory {:?}. Failed with error: {:?}",
                build_path, error
            )
        }
    }

    fn get_build_path(&self, build_id: &str) -> PathBuf {
        self.repository_path.clone().join("builds").join(build_id)
    }
}

fn hash_and_store_data(build_path: &Path, bytes: &[u8]) -> Result<(PathBuf, String), io::Error> {
    let hash = calculate_hash(bytes);
    let mut data_location = PathBuf::from(build_path);
    data_location.push(&hash);
    let mut file = fs::File::create(&data_location)?;
    io::copy(&mut bytes.reader(), &mut file)?;

    Ok((data_location, hash))
}

fn calculate_hash(bytes: &[u8]) -> String {
    let mut sha256 = multihash::Sha2_256::default();
    sha256.update(bytes);
    hex::encode(sha256.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_service::mapping::model::MappingInfo;
    use crate::util::test_util;
    use httptest::{matchers, responders, Expectation, Server};
    use tokio::sync::mpsc;

    #[tokio::test]
    async fn test_start_build() {
        let tmp_dir = test_util::tests::setup();

        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.2";

        let (sender, _) = mpsc::channel(1);

        let build_event_client = BuildEventClient::new(sender);

        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id: "alpine:3.15.2".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let build_id = uuid::Uuid::new_v4().to_string();

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::all_of!(
                matchers::request::method_path("PUT", "/build"),
                matchers::request::body(matchers::json_decoded(matchers::eq(serde_json::json!(
                    &mapping_info
                ))))
            ))
            .respond_with(responders::json_encoded(&build_id)),
        );

        let mapping_service_endpoint = "https://mapping-service.pyrsia.io/";
        let pipeline_service_endpoint = &http_server.url_str("/");

        let build_service = BuildService::new(
            &tmp_dir,
            build_event_client,
            mapping_service_endpoint,
            pipeline_service_endpoint,
        )
        .unwrap();
        let build_id_result = build_service
            .start_build(
                package_type,
                package_specific_id.to_owned(),
                BuildTrigger::FromSource,
            )
            .await
            .unwrap();

        assert_eq!(build_id_result, build_id);

        test_util::tests::teardown(tmp_dir);
    }
}
