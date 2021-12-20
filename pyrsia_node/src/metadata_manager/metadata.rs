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
extern crate log;

use super::model::namespace::Namespace;
use super::model::package::Package;
use super::model::package_type::{PackageType, PackageTypeName};
use super::model::package_version::PackageVersion;
use anyhow::anyhow;
use anyhow::Result;
use log::{debug, error, info, warn};
use pyrsia_client_lib::signed::{Attestation, Signed};
use pyrsia_node::document_store::document_store::{DocumentStore, IndexSpec};
use std::collections::HashMap;
use std::env;
use std::error::Error;
use std::fmt::Debug;
use std::fs;

/// This trait is implemented by structs that provide the MetadataApi.
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

// Names of document stores and their indexes
const DS_PACKAGE_TYPES: &str = "package_types";
const IX_PACKAGE_TYPES_NAMES: &str = "names";
const FLD_PACKAGE_TYPES_NAME: &str = "name";
fn ix_package_types() -> Vec<IndexSpec> {
    vec![IndexSpec::new(
        String::from(IX_PACKAGE_TYPES_NAMES),
        vec![String::from(FLD_PACKAGE_TYPES_NAME)],
    )]
}
fn init_package_types() -> Vec<String> {
    init_empty() // TODO This should be replaced by code to insert a signed json string that defines the Docker package type.
}

const DS_NAMESPACES: &str = "namespaces";
const IX_NAMESPACES_ID: &str = "ids";
const IX_NAMESPACES_PATH: &str = "paths";
const FLD_NAMESPACES_ID: &str = "id";
const FLD_NAMESPACES_PKG_TYPE: &str = "package_type";
const FLD_NAMESPACES_PATH: &str = "namespace_path";
fn ix_namespaces() -> Vec<IndexSpec> {
    vec![
        IndexSpec::new(
            String::from(IX_NAMESPACES_ID),
            vec![String::from(FLD_NAMESPACES_ID)],
        ),
        IndexSpec::new(
            String::from(IX_NAMESPACES_PATH),
            vec![
                String::from(FLD_NAMESPACES_PKG_TYPE),
                String::from(FLD_NAMESPACES_PATH),
            ],
        ),
    ]
}

fn init_empty() -> Vec<String> {
    vec![]
}

#[derive(Debug)]
pub struct Metadata<'a> {
    trust_manager: &'a dyn TrustManager,
    package_type_docs: DocumentStore,
    namespace_docs: DocumentStore,
}

impl<'a> Metadata<'a> {
    pub fn new() -> Result<Metadata<'a>, anyhow::Error> {
        info!("Creating new instance of metadata manager");
        let package_type_docs =
            open_document_store(DS_PACKAGE_TYPES, ix_package_types, init_package_types)?;
        let namespace_docs = open_document_store(DS_NAMESPACES, ix_namespaces, init_empty)?;
        Ok(Metadata {
            trust_manager: &DefaultTrustManager {},
            package_type_docs,
            namespace_docs,
        })
    }
}

/// Open the specified document store. If that fails, try creating it.
/// * `ds_name` ― The name of the document store.
/// * `index_specs` ― If the document store need to be created, this method is called to get a
///                   a description of the indexes it will have.
/// * `initial_records` ― If the document store needs to be created, this method is called to get
///                       JSON string that will each be inserted as a record into the document store.
fn open_document_store(
    ds_name: &str,
    index_specs: fn() -> Vec<IndexSpec>,
    initial_records: fn() -> Vec<String>,
) -> anyhow::Result<DocumentStore> {
    info!("Opening document store: {}", ds_name);
    match DocumentStore::get(ds_name) {
        Ok(ds) => Ok(ds),
        Err(error) => {
            warn!(
                "Failed to open document store {}; error was {}",
                ds_name, error
            );
            info!("Attempting to create document store {}", ds_name);
            match DocumentStore::create(ds_name, index_specs()) {
                Ok(ds) => {
                    info!("Created document store {}", ds_name);
                    populate_with_initial_records(&ds, initial_records)?;
                    Ok(ds)
                }
                Err(error) => failed_to_create_document_store(ds_name, error),
            }
        }
    }
}

fn populate_with_initial_records(
    ds: &DocumentStore,
    initial_records: fn() -> Vec<String>,
) -> Result<()> {
    for record in initial_records() {
        if let Err(error) = ds.insert(&record) {
            error!(
                "Failed to insert initial record into document store {}: {}\nError was {}",
                ds.name, record, error
            );
            todo!("If an attempt to insert an initial record into document store fails, then we need to do something so that we will know that the document store is missing necessary information")
        }
    }
    Ok(())
}

fn failed_to_create_document_store(
    ds_name: &str,
    error: Box<dyn Error>,
) -> Result<DocumentStore, anyhow::Error> {
    let msg = format!(
        "Failed to create document store {} due to error {}",
        ds_name, error
    );
    error!("{}", msg);
    return Err(anyhow!(msg));
}

impl MetadataApi for Metadata<'_> {
    fn create_package_type(&self, pkg_type: &PackageType) -> anyhow::Result<(), anyhow::Error> {
        match self.trust_manager.trust_package_type(pkg_type) {
            Ok(_) => {
                if let Err(error) = self.package_type_docs.insert(&pkg_type.json().unwrap()) {
                    error!("{}", error);
                    Err(anyhow!(
                        "Failed to create package_type record: {:?}",
                        pkg_type
                    ))
                } else {
                    Ok(())
                }
            }
            Err(error) => Err(anyhow!(
                "New package type is not trusted: JSON is {}\nError is{}",
                pkg_type.json().unwrap_or("None".to_string()),
                error
            )),
        }
    }

    fn get_package_type(
        &self,
        name: PackageTypeName,
    ) -> anyhow::Result<Option<PackageType>, anyhow::Error> {
        let mut filter = HashMap::new();
        let name_as_string = name.to_string();
        filter.insert(FLD_PACKAGE_TYPES_NAME, name_as_string.as_str());
        match self.package_type_docs.fetch(IX_PACKAGE_TYPES_NAMES, filter) {
            Err(error) => Err(anyhow!("Error fetching package type: {}", error)),
            Ok(Some(json)) => Ok(Some(PackageType::from_json_string(&json)?)),
            Ok(None) => Ok(None),
        }
    }

    fn create_namespace(&self, namespace: &Namespace) -> anyhow::Result<(), anyhow::Error> {
        match self.trust_manager.trust_namespace(namespace) {
            Ok(_) => {
                if let Err(error) = self.namespace_docs.insert(&namespace.json().unwrap()) {
                    error!("{}", error);
                    Err(anyhow!(
                        "Failed to create namespace record: {:?}",
                        namespace
                    ))
                } else {
                    Ok(())
                }
            }
            Err(error) => Err(anyhow!("New namespace is not trusted: {}", error)),
        }
    }

    fn get_namespace(
        &self,
        _package_type: PackageTypeName,
        _namespace_path: &[&str],
    ) -> anyhow::Result<Option<Namespace>, anyhow::Error> {
        todo!()
    }

    fn get_namespace_by_id(&self, _id: &str) -> anyhow::Result<Option<Namespace>, anyhow::Error> {
        todo!()
    }

    fn get_namespaces_by_package_type(
        &self,
        _package_type: PackageTypeName,
    ) -> anyhow::Result<NamespaceIterator, anyhow::Error> {
        todo!()
    }

    fn create_package(&self, _package: &Package) -> anyhow::Result<(), anyhow::Error> {
        todo!()
    }

    fn get_package(
        &self,
        _package_type: PackageTypeName,
        _namespace_id: &str,
        _package_name: &str,
    ) -> anyhow::Result<Option<Package>, anyhow::Error> {
        todo!()
    }

    fn get_package_by_namespace_path(
        &self,
        _package_type: PackageTypeName,
        _namespace_path: &[&str],
        _package_name: &str,
    ) -> anyhow::Result<Option<Package>, anyhow::Error> {
        todo!()
    }

    fn get_packages_by_namespace_id(
        &self,
        _namespace_id: &str,
    ) -> anyhow::Result<PackageIterator, anyhow::Error> {
        todo!()
    }

    fn update_package(
        &self,
        _package: &Package,
        _previous_signature: &str,
    ) -> anyhow::Result<(), anyhow::Error> {
        todo!()
    }

    fn create_package_version(
        &self,
        _package_version: &PackageVersion,
    ) -> anyhow::Result<(), anyhow::Error> {
        todo!()
    }

    fn get_package_version(
        &self,
        _namespace_id: &str,
        _package_name: &str,
        _version: &str,
    ) -> Result<Option<PackageVersion>, anyhow::Error> {
        todo!()
    }

    fn get_package_version_by_id(
        &self,
        _id: &str,
    ) -> Result<Option<PackageVersion>, anyhow::Error> {
        todo!()
    }
}

// TODO move trust manager trait and its default implementation to a separate module
trait TrustManager: Debug {
    fn trust_package_type(&self, pkg_type: &PackageType) -> Result<()>;
    fn trust_namespace(&self, namespace: &Namespace) -> Result<()>;
    fn trust_package(&self, package: &Package) -> Result<()>;
    fn trust_package_version(self, package_version: &PackageVersion) -> Result<()>;
}

#[derive(Debug)]
struct DefaultTrustManager {}

impl TrustManager for DefaultTrustManager {
    fn trust_package_type(&self, pkg_type: &PackageType) -> anyhow::Result<()> {
        let json = pkg_type.json();
        match json {
            Some(_) => match pkg_type.verify_signature() {
                Ok(attestations) => {
                    process_attestations(attestations)
                }
                Err(error) => Err(error),
            },
            None => Err(anyhow!("Unsigned package type")),
        }
    }

    fn trust_namespace(&self, _namespace: &Namespace) -> Result<()> {
        todo!()
    }

    fn trust_package(&self, _package: &Package) -> Result<()> {
        todo!()
    }

    fn trust_package_version(self, _package_version: &PackageVersion) -> Result<()> {
        todo!()
    }
}

fn process_attestations(attestations: Vec<Attestation>) -> Result<()> {
    debug!(
        "Found {} valid signatures out of {}",
        attestations
            .iter()
            .filter(|attestation| attestation.signature_is_valid())
            .count(),
        attestations.len()
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use pyrsia_client_lib::signed::{create_key_pair, JwsSignatureAlgorithms};
    use rand::Rng;
    use serial_test::serial;
    use std::path::Path;

    const DIR_PREFIX: &str = "metadata_test_";

    fn do_in_temp_directory(
        runner: fn() -> Result<(), anyhow::Error>,
    ) -> Result<(), anyhow::Error> {
        let mut rng = rand::thread_rng();
        let n: u32 = rng.gen();
        let mut dir_name = String::from(DIR_PREFIX);
        dir_name.push_str(&n.to_string());
        let dir_path = Path::new(&dir_name);
        let prev_dir = env::current_dir()?;
        info!("Creating temp directory {}", dir_path.to_str().unwrap());
        fs::create_dir_all(dir_path)?;
        env::set_current_dir(dir_path)?;
        let result = runner();
        env::set_current_dir(prev_dir)?;
        info!("Removing temp directory {}", dir_path.to_str().unwrap());
        fs::remove_dir_all(dir_path)?;
        result
    }

    #[test]
    #[serial]
    fn package_type_test() -> Result<()> {
        do_in_temp_directory(|| {
            info!("Creating metadata instance");
            let metadata = Metadata::new()?;
            info!("Created metadata instance: {:?}", metadata);

            let key_pair = create_key_pair(JwsSignatureAlgorithms::RS512)?;
            info!("Created key pair");

            let mut package_type =
                PackageType::new(PackageTypeName::Docker, String::from("docker packages"));

            package_type.sign_json(
                JwsSignatureAlgorithms::RS512,
                &key_pair.private_key,
                &key_pair.public_key,
            )?;
            info!("creating signed package type");
            metadata.create_package_type(&package_type)?;
            let package_type2 = metadata.get_package_type(PackageTypeName::Docker)?.unwrap();
            assert_eq!(package_type2.name(), package_type.name());
            assert_eq!(package_type2.description(), package_type.description());
            Ok(())
        })
    }

    #[test]
    #[serial]
    fn unsigned_package_type() -> Result<()> {
        let metadata = Metadata::new()?;
        let package_type =
            PackageType::new(PackageTypeName::Docker, String::from("docker packages"));
        if let Ok(_) = metadata.create_package_type(&package_type) {
            return Err(anyhow!(
                "create_package_type accepted an unsigned package type"
            ));
        };
        Ok(())
    }
}
