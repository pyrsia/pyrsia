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

use crate::artifact_service::service::PackageType;
use crate::build_service::model::{BuildError, BuildInfo, BuildResult, BuildStatus};
use std::path::{Path, PathBuf};

/// The build service is a component used by authorized nodes only. It is
/// the entrypoint to the authorized node's build pipeline infrastructure.
#[derive(Default)]
pub struct BuildService {
    _repository_path: PathBuf,
}

impl BuildService {
    pub fn new<P: AsRef<Path>>(repository_path: P) -> Result<Self, anyhow::Error> {
        let repository_path = repository_path.as_ref().to_path_buf().canonicalize()?;
        Ok(BuildService {
            _repository_path: repository_path,
        })
    }

    /// Starts a new build for the specified package.
    pub async fn start_build(
        &self,
        _package_type: PackageType,
        _package_specific_id: &str,
        _sender: oneshot::Sender<Result<Vec<BuildResult>, BuildError>>,
    ) -> Result<BuildInfo, BuildError> {
        Ok(BuildInfo {
            id: uuid::Uuid::new_v4().to_string(),
            status: BuildStatus::Running,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::util::test_util;

    #[tokio::test]
    async fn test_start_build() {
        let tmp_dir = test_util::tests::setup();

        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.2";

        let (sender, _) = oneshot::channel();

        let build_service = BuildService::new(&tmp_dir).unwrap();
        let build_result = build_service
            .start_build(package_type, package_specific_id, sender)
            .await;

        assert!(build_result.is_ok());

        test_util::tests::teardown(tmp_dir);
    }
}
