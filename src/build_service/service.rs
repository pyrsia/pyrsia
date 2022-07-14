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

use tokio::sync::oneshot;

use super::error::BuildError;
use super::mapping::service::MappingService;
use super::model::{BuildInfo, BuildResult};
use super::pipeline::service::PipelineService;
use crate::artifact_service::model::PackageType;
use std::path::{Path, PathBuf};

/// The build service is a component used by authorized nodes only. It is
/// the entrypoint to the authorized node's build pipeline infrastructure.
pub struct BuildService {
    _repository_path: PathBuf,
    mapping_service: MappingService,
    pipeline_service: PipelineService,
}

impl BuildService {
    pub fn new<P: AsRef<Path>>(
        repository_path: P,
        mapping_service_endpoint: &str,
        pipeline_service_endpoint: &str,
    ) -> Result<Self, anyhow::Error> {
        let repository_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        Ok(BuildService {
            _repository_path: repository_path,
            mapping_service: MappingService::new(mapping_service_endpoint),
            pipeline_service: PipelineService::new(pipeline_service_endpoint),
        })
    }

    /// Starts a new build for the specified package.
    pub async fn start_build(
        &self,
        package_type: PackageType,
        package_specific_id: &str,
        _sender: oneshot::Sender<Result<Vec<BuildResult>, BuildError>>,
    ) -> Result<BuildInfo, BuildError> {
        let mapping_info = self
            .mapping_service
            .get_mapping(package_type, package_specific_id)
            .await?;

        self.pipeline_service.start_build(mapping_info).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::build_service::mapping::model::MappingInfo;
    use crate::build_service::model::BuildStatus;
    use crate::util::test_util;
    use httptest::{matchers, responders, Expectation, Server};

    #[tokio::test]
    async fn test_start_build() {
        let tmp_dir = test_util::tests::setup();

        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.2";

        let (sender, _) = oneshot::channel();

        let mapping_info = MappingInfo {
            package_type: PackageType::Docker,
            package_specific_id: "alpine:3.15.2".to_owned(),
            source_repository: None,
            build_spec_url: None,
        };

        let build_info = BuildInfo {
            id: uuid::Uuid::new_v4().to_string(),
            status: BuildStatus::Running,
        };

        let http_server = Server::run();
        http_server.expect(
            Expectation::matching(matchers::all_of!(
                matchers::request::method_path("PUT", "/build"),
                matchers::request::body(matchers::json_decoded(matchers::eq(serde_json::json!(
                    &mapping_info
                ))))
            ))
            .respond_with(responders::json_encoded(&build_info)),
        );

        let mapping_service_endpoint = "https://mapping-service.pyrsia.io/";
        let pipeline_service_endpoint = &http_server.url_str("/");

        let build_service = BuildService::new(
            &tmp_dir,
            mapping_service_endpoint,
            pipeline_service_endpoint,
        )
        .unwrap();
        let build_info_result = build_service
            .start_build(package_type, package_specific_id, sender)
            .await
            .unwrap();

        assert_eq!(build_info_result, build_info);

        test_util::tests::teardown(tmp_dir);
    }
}
