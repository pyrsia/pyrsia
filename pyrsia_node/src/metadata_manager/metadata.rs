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
extern crate anyhow;

use super::super::node_manager::model::package_type;

// create namespace
// get namespace
// get namespaces by package type
// create package
// get package
// update package
// get packages by namespace
// update package
// create package version

pub trait MetadataApi {
    /// Create a new package type with the information specified in the `pkg_type` parameter.
    ///
    /// Returns an error if `pkg_type` does not have any valid signatures of it any of the valid
    /// signatures are associated with a public key that does not identify an identity in the block
    /// chain.
    fn create_package_type(pkg_type: package_type) -> Result<(), anyhow::Error>;
}
