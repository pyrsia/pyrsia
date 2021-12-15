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

use crate::node_manager::model::namespace::Namespace;
use crate::node_manager::model::package::Package;
use crate::node_manager::model::package_type::{PackageType, PackageTypeName};

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
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    ///
    /// Also returns an error if there is already package_type with the same name.
    fn create_package_type(&self, pkg_type: &PackageType) -> Result<(), anyhow::Error>;

    /// Return a PackageType struct that describes the named package type.
    fn get_package_type(&self, name: PackageTypeName) -> Result<Option<PackageType>, anyhow::Error>;

    /// Define the namespace described by the given `Namespace` struct.
    ///
    /// Returns an error if there is already a namespace with the same id or the same package_type and namespace_path.
    ///
    /// There may be rules associated with some package types about what a valid namespace path can be. If the namespace
    /// path violates such rules, an error will be returned.
    ///
    /// Returns an error if `namespace` does not have any valid signatures of it any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    fn create_namespace(&self, namespace: &Namespace) -> Result<(), anyhow::Error>;

    /// Get the namespace identified by the given package type and namespace path.
    fn get_namespace(&self, package_type: PackageTypeName, namespace_path: &str) -> Result<Option<Namespace>, anyhow::Error>;

    /// Get the namespace identified by the given id.
    fn get_namespace_by_id(&self, id: &str) -> Result<Namespace, anyhow::Error>;

    /// Get an iterator over the namespaces associated with the specified package type.
    fn get_namespace_by_package_type(&self, package_type: PackageTypeName) -> Result<NamespaceIterator, anyhow::Error>;

    fn create_package(&self, package: &Package) -> Result<(), anyhow::Error>;
}

/// Used to iterate over collection of namespaces without requiring the collection to fit in memory.
pub struct NamespaceIterator {}

impl Iterator for NamespaceIterator {
    type Item = Namespace;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
