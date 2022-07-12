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

use thiserror::Error;
use tokio::sync::oneshot;

use crate::artifact_service::service::PackageType;

#[derive(Debug, Error)]
pub enum BuildError {}

pub struct BuildResult {}

/// The build service is a component used by authorized nodes only. It is
/// the entrypoint to the authorized node's build pipeline infrastructure.
#[derive(Default)]
pub struct BuildService {}

impl BuildService {
    /// Starts a new build for the specified package.
    pub async fn start_build(
        &self,
        _package_type: PackageType,
        _package_specific_id: &str,
        _sender: oneshot::Sender<Result<Vec<BuildResult>, BuildError>>,
    ) -> Result<(), BuildError> {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_start_build() {
        let package_type = PackageType::Docker;
        let package_specific_id = "alpine:3.15.2";

        let (sender, _) = oneshot::channel();

        let build_service = BuildService::default();
        let build_result = build_service
            .start_build(package_type, package_specific_id, sender)
            .await;

        assert!(build_result.is_ok());
    }
}
