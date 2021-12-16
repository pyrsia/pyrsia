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
use crate::node_manager::model::package_version::PackageVersion;

// create package version

pub trait MetadataApi {
    /// Create a new package type with the information specified in the `pkg_type` parameter.
    ///
    /// Returns an error if `pkg_type` does not have any valid signatures or i any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    ///
    /// Also returns an error if there is already package_type with the same name.
    fn create_package_type(&self, pkg_type: &PackageType) -> Result<(), anyhow::Error>;

    /// Return a PackageType struct that describes the named package type.
    fn get_package_type(&self, name: PackageTypeName)
        -> Result<Option<PackageType>, anyhow::Error>;

    /// Define the namespace described by the given `Namespace` struct.
    ///
    /// Returns an error if there is already a namespace with the same id or the same package_type and namespace_path.
    ///
    /// There may be rules associated with some package types about what a valid namespace path can be. If the namespace
    /// path violates such rules, an error will be returned.
    ///
    /// Returns an error if `namespace` does not have any valid signatures or if any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    fn create_namespace(&self, namespace: &Namespace) -> Result<(), anyhow::Error>;

    /// Get the namespace identified by the given package type and namespace path.
    fn get_namespace(
        &self,
        package_type: PackageTypeName,
        namespace_path: &[&str],
    ) -> Result<Option<Namespace>, anyhow::Error>;

    /// Get the namespace identified by the given id.
    fn get_namespace_by_id(&self, id: &str) -> Result<Option<Namespace>, anyhow::Error>;

    /// Get an iterator over the namespaces associated with the specified package type.
    fn get_namespaces_by_package_type(
        &self,
        package_type: PackageTypeName,
    ) -> Result<NamespaceIterator, anyhow::Error>;

    /// Define the package described by the given `Package` struct.
    ///
    /// Returns an error if there is already a package with the same package_type, namespace and
    /// package_name.
    ///
    /// Returns an error if `package` does not have any valid signatures or if any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    fn create_package(&self, package: &Package) -> Result<(), anyhow::Error>;

    /// Get the package identified by the combination of the given package type, namespace id and
    /// package name.
    fn get_package(
        &self,
        package_type: PackageTypeName,
        namespace_id: &str,
        package_name: &str,
    ) -> Result<Option<Package>, anyhow::Error>;

    /// Get the package identified by the combination of the given package type, namespace path and
    /// package name.
    fn get_package_by_namespace_path(
        &self,
        package_type: PackageTypeName,
        namespace_path: &[&str],
        package_name: &str,
    ) -> Result<Option<Package>, anyhow::Error>;

    /// Get an iterator over the packages associated with the namespace identified by the given
    /// namespace ID.
    fn get_packages_by_namespace_id(
        &self,
        namespace_id: &str,
    ) -> Result<PackageIterator, anyhow::Error>;

    /// Update the package described by the given `Package` struct with the information in the
    /// struct.
    ///
    /// The value of the `previous_signature` parameter must be equal to the contents of the
    /// `__signature` field of the json of the existing package record (available by calling the
    /// signed structs `signatures` method). If it is not, the update is assumed to be based on a
    /// stale version of the package record (someone else updated the package first) and an error is
    /// returned.
    ///
    /// The values of the `name`, `package_type`, `namespace_id` and `creation_time` fields must be
    /// the same as the values in the existing record. Updates to these fields are not allowed.
    ///
    /// The value of the `modified_time` field must be greater than or equal to the existing record.
    /// If the value of `modified_time` is greater than in the existing record, its value is updated
    /// to the new later value. If the value of the `modified_time` field is equal to the existing
    /// record, then this method updates the `modified_time` field with the current time.
    ///
    /// The `Vec` that is the value of the `versions` field must include all of the values in the
    /// existing record or an error is returned.
    ///
    /// Returns an error if `package` does not have any valid signatures or if any of the valid
    /// signatures are associated with a public key that does not identify an identity in the
    /// blockchain.
    ///
    /// If the values of the `administrators` field in the existing record is not an empty `Vec`,
    /// then the public key of at least one of the signers of this `Package` must be one of the
    /// public keys in the `administrators` field. Otherwise an error is returned.
    fn update_package(
        &self,
        package: &Package,
        previous_signature: &str,
    ) -> Result<(), anyhow::Error>;

    /// Define the package version described by the given `PackageVersion` struct.
    ///
    /// Returns an error if there is already a package version with the same id or the same
    /// combination of package_type, namespace_id, package_name and version.
    ///
    /// Returns an error if `package_version` does not have any valid signatures or if any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    fn create_package_version(&self, package_version: &PackageVersion)
        -> Result<(), anyhow::Error>;

    /// Get the package_version that matches the given namespace_id, package_name and version.
    fn get_package_version(
        &self,
        namespace_id: &str,
        package_name: &str,
        version: &str,
    ) -> Result<Option<PackageVersion>, anyhow::Error>;

    /// Get the package_version that has the given id.
    fn get_package_version_by_id(&self, id: &str) -> Result<Option<PackageVersion>, anyhow::Error>;
}

/// Used to iterate over a collection of namespaces without requiring the collection to fit in memory.
pub struct NamespaceIterator {}

impl Iterator for NamespaceIterator {
    type Item = Namespace;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}

/// Used to iterate over a collection of packages without requiring the collection to fit in memory.
pub struct PackageIterator {}

impl Iterator for PackageIterator {
    type Item = Package;

    fn next(&mut self) -> Option<Self::Item> {
        todo!()
    }
}
