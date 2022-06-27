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
use crate::transparency_log::log::Transaction;

#[derive(Debug, Error)]
pub enum BuildError {}

pub struct BuildResult {}

/// The build service is a component used by authorized nodes only. It is
/// the entrypoint to the authorized node's build pipeline infrastructure.
pub struct BuildService {}

impl BuildService {
    /// Starts a new build for the specified package.
    pub async fn start_build(
        &self,
        _package_type: PackageType,
        _package_type_id: &str,
        _sender: oneshot::Sender<Result<Vec<BuildResult>, BuildError>>,
    ) -> Result<(), BuildError> {
        Ok(())
    }

    /// Verify a build for the specified transaction. This method is
    /// used to be able to reach consensus about a built artifact
    /// between authorized nodes.
    pub async fn verify_build(
        &self,
        _add_build_transaction: Transaction,
        _sender: oneshot::Sender<Result<(), BuildError>>,
    ) -> Result<(), BuildError> {
        Ok(())
    }
}
