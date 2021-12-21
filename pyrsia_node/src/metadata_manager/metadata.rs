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
extern crate maplit;

use super::model::namespace::Namespace;
use super::model::package::Package;
use super::model::package_type::{PackageType, PackageTypeName};
use super::model::package_version::PackageVersion;
use anyhow::anyhow;
use anyhow::Result;
use log::{debug, error, info, warn};
use maplit::hashmap;
use pyrsia_client_lib::iso8601;
use pyrsia_client_lib::signed::{Attestation, Signed};
use pyrsia_node::document_store::document_store::{DocumentStore, IndexSpec};
use serde::de::Unexpected::Str;
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
    fn create_package_type(&self, pkg_type: &PackageType) -> Result<()>;

    /// Return a PackageType struct that describes the named package type.
    fn get_package_type(&self, name: PackageTypeName) -> Result<Option<PackageType>>;

    /// Define the namespace described by the given `Namespace` struct.
    ///
    /// Returns an error if there is already a namespace with the same id or the same package_type and namespace_path.
    ///
    /// There may be rules associated with some package types about what a valid namespace path can be. If the namespace
    /// path violates such rules, an error will be returned.
    ///
    /// Returns an error if `namespace` does not have any valid signatures or if any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    fn create_namespace(&self, namespace: &Namespace) -> Result<()>;

    /// Get the namespace identified by the given package type and namespace path.
    fn get_namespace(
        &self,
        package_type: PackageTypeName,
        namespace_path: &str,
    ) -> Result<Option<Namespace>>;

    /// Get the namespace identified by the given id.
    fn get_namespace_by_id(&self, id: &str) -> Result<Option<Namespace>>;

    /// Get an iterator over the namespaces associated with the specified package type.
    fn get_namespaces_by_package_type(
        &self,
        package_type: PackageTypeName,
    ) -> Result<NamespaceIterator>;

    /// Define the package described by the given `Package` struct.
    ///
    /// Returns an error if there is already a package with the same package_type, namespace and
    /// package_name.
    ///
    /// Returns an error if `package` does not have any valid signatures or if any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    fn create_package(&self, package: &Package) -> Result<()>;

    /// Get the package identified by the combination of the given package type, namespace id and
    /// package name.
    fn get_package(
        &self,
        package_type: PackageTypeName,
        namespace_id: &str,
        package_name: &str,
    ) -> Result<Option<Package>>;

    /// Get the package identified by the combination of the given package type, namespace path and
    /// package name.
    fn get_package_by_namespace_path(
        &self,
        package_type: PackageTypeName,
        namespace_path: &[&str],
        package_name: &str,
    ) -> Result<Option<Package>>;

    /// Get an iterator over the packages associated with the namespace identified by the given
    /// namespace ID.
    fn get_packages_by_namespace_id(&self, namespace_id: &str) -> Result<PackageIterator>;

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
    fn update_package(&self, package: &Package, previous_signature: &str) -> Result<()>;

    /// Define the package version described by the given `PackageVersion` struct.
    ///
    /// Returns an error if there is already a package version with the same id or the same
    /// combination of package_type, namespace_id, package_name and version.
    ///
    /// Returns an error if `package_version` does not have any valid signatures or if any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    fn create_package_version(&self, package_version: &PackageVersion) -> Result<()>;

    /// Get the package_version that matches the given namespace_id, package_name and version.
    fn get_package_version(
        &self,
        namespace_id: &str,
        package_name: &str,
        version: &str,
    ) -> Result<Option<PackageVersion>>;

    /// Get the package_version that has the given id.
    fn get_package_version_by_id(&self, id: &str) -> Result<Option<PackageVersion>>;
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

////////////////////////////////////////////////////////////////////////////////////////////////////
//
// The first time that a node is run, its metadata manager needs to create the document stores that
// it will used to hold metadata. Below are sets of definitions used to create the document stores.
//
// Each set of definitions has some const declarations follows by one or two function definitions.
// The first const is for the name of the document store. The names of these follow the pattern
// DS_documentStoreName. For example the const for the name of the document store named
// "package_types" is DS_PACKAGE_TYPES.
//
// Each document store will have at least one index. Each index has a name. The pattern for const
// names that define the name of an index is IX_documentStoreName_indexName. For example, the name
// of the const for an index named "names" for a document store named "package_types" is
// IX_PACKAGE_TYPES_NAMES. There will be one of these IX consts for each index that a document store
// will have.
//
// Since we need to specify which fields an index will refer to, there are also const names defined
// for each field that is covered by an index. The patter for these names is
// FLD_documentStoreName_fieldName. For example the const name for the field named "name" in the
// document store named "package_types" is FLD_PACKAGE_TYPES_NAME.
//
// For each document store to be created, there is a function named with the pattern
// ix_documentStoreName. For example, for the document store named "package_types" the name of this
// function is ix_package_types. This function uses the const values defined for the document store
// to create and return a Vec of index definitions.
//
// For some of the document stores there is a second function named with the pattern
// init_documentStoreName. For example, there is one of these functions for the document store named
// "package_types". The name of the function is init_package_types. This method creates a Vec of
// JSON strings that the document store will be pre-populated with.
//
////////////////////////////////////////////////////////////////////////////////////////////////////
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

const DS_PACKAGES: &str = "packages";
const IX_PACKAGES_NAME: &str = "names";
const FLD_PACKAGES_PACKAGE_TYPE: &str = "package_type";
const FLD_PACKAGES_NAMESPACE_ID: &str = "namespace_id";
const FLD_PACKAGES_NAME: &str = "name";

fn ix_packages() -> Vec<IndexSpec> {
    vec![IndexSpec::new(
        String::from(IX_PACKAGES_NAME),
        vec![
            String::from(FLD_PACKAGES_PACKAGE_TYPE),
            String::from(FLD_PACKAGES_NAMESPACE_ID),
            String::from(FLD_PACKAGES_NAME),
        ],
    )]
}

fn init_empty() -> Vec<String> {
    vec![]
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// End of definitions to support creation of document stores
////////////////////////////////////////////////////////////////////////////////////////////////////

#[derive(Debug)]
pub struct Metadata<'a> {
    trust_manager: &'a dyn TrustManager,
    package_type_docs: DocumentStore,
    namespace_docs: DocumentStore,
    package_docs: DocumentStore,
}

impl<'a> Metadata<'a> {
    pub fn new() -> Result<Metadata<'a>> {
        info!("Creating new instance of metadata manager");
        let package_type_docs =
            open_document_store(DS_PACKAGE_TYPES, ix_package_types, init_package_types)?;
        let namespace_docs = open_document_store(DS_NAMESPACES, ix_namespaces, init_empty)?;
        let package_docs = open_document_store(DS_PACKAGES, ix_packages, init_empty)?;
        Ok(Metadata {
            trust_manager: &DefaultTrustManager {},
            package_type_docs,
            namespace_docs,
            package_docs,
        })
    }
}

/// Open the specified document store. If that fails, try creating it.
/// * `ds_name` ― The name of the document store.
/// * `index_specs` ― If the document store need to be created, this method is called to get a
///                   a description of the indexes it will have. When we create a new document store
///                   we need to define the names of the indexes it will use and the names of the
///                   fields that each index will be based on.
/// * `initial_records` ― If the document store needs to be created, this method is called to get
///                       JSON string that will each be inserted as a record into the document
///                       store. While most document stores will initially be empty, some need to be
///                       pre-populated with some records.
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

// Most types of metadata will come from the Pyrsia network or the node's clients. However there are
// a few type of metadata that will need to be at least partially pre-populated in new nodes. For
// example, package-type should be pre-populated with one record for each type of package that the
// pyrsia node supports.
//
// When the metadata manager is setting up its local document stores to store metadata, it calls
// this function to pre-populate each document store with any records that it should have.
// The parameters are
// ds - A document store to pre-populate
// initial_records - a function that returns a Vec of JSON strings that are to be stored as the
//                   initial content of the document store. Most document stores will not need to be
//                   pre-populated with anything. For these, the function passed will return an
//                   empty Vec.
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

fn failed_to_create_document_store(ds_name: &str, error: Box<dyn Error>) -> Result<DocumentStore> {
    let msg = format!(
        "Failed to create document store {} due to error {}",
        ds_name, error
    );
    error!("{}", msg);
    return Err(anyhow!(msg));
}

impl MetadataApi for Metadata<'_> {
    fn create_package_type(&self, pkg_type: &PackageType) -> anyhow::Result<()> {
        match self.trust_manager.trust_package_type(pkg_type) {
            Ok(_) => insert_metadata(&self.package_type_docs, pkg_type),
            Err(error) => untrusted_metadata_error(pkg_type, &error.to_string()),
        }
    }

    fn get_package_type(&self, name: PackageTypeName) -> anyhow::Result<Option<PackageType>> {
        let name_as_string = name.to_string();
        let filter = hashmap! {
            FLD_PACKAGE_TYPES_NAME => name_as_string.as_str()
        };
        match self.package_type_docs.fetch(IX_PACKAGE_TYPES_NAMES, filter) {
            Err(error) => Err(anyhow!("Error fetching package type: {}", error)),
            Ok(Some(json)) => Ok(Some(PackageType::from_json_string(&json)?)),
            Ok(None) => Ok(None),
        }
    }

    fn create_namespace(&self, namespace: &Namespace) -> anyhow::Result<()> {
        match self.trust_manager.trust_namespace(namespace) {
            Ok(_) => insert_metadata(&self.namespace_docs, namespace),
            Err(error) => untrusted_metadata_error(namespace, &error.to_string()),
        }
    }

    fn get_namespace(
        &self,
        package_type: PackageTypeName,
        namespace_path: &str,
    ) -> anyhow::Result<Option<Namespace>> {
        let package_type_as_string = package_type.to_string();
        let filter = hashmap! {
            FLD_NAMESPACES_PKG_TYPE => package_type_as_string.as_str(),
            FLD_NAMESPACES_PATH => namespace_path
        };
        fetch_namespace(self, IX_NAMESPACES_PATH, filter)
    }

    fn get_namespace_by_id(&self, id: &str) -> anyhow::Result<Option<Namespace>> {
        let filter = hashmap! {
            FLD_NAMESPACES_ID => id
        };
        fetch_namespace(self, IX_NAMESPACES_ID, filter)
    }

    fn get_namespaces_by_package_type(
        &self,
        _package_type: PackageTypeName,
    ) -> anyhow::Result<NamespaceIterator> {
        todo!() // Requires range search support from the document store.
    }

    fn create_package(&self, _package: &Package) -> anyhow::Result<()> {
        todo!()
    }

    fn get_package(
        &self,
        _package_type: PackageTypeName,
        _namespace_id: &str,
        _package_name: &str,
    ) -> anyhow::Result<Option<Package>> {
        todo!()
    }

    fn get_package_by_namespace_path(
        &self,
        _package_type: PackageTypeName,
        _namespace_path: &[&str],
        _package_name: &str,
    ) -> anyhow::Result<Option<Package>> {
        todo!()
    }

    fn get_packages_by_namespace_id(&self, _namespace_id: &str) -> anyhow::Result<PackageIterator> {
        todo!()
    }

    fn update_package(&self, _package: &Package, _previous_signature: &str) -> anyhow::Result<()> {
        todo!()
    }

    fn create_package_version(&self, _package_version: &PackageVersion) -> anyhow::Result<()> {
        todo!()
    }

    fn get_package_version(
        &self,
        _namespace_id: &str,
        _package_name: &str,
        _version: &str,
    ) -> Result<Option<PackageVersion>> {
        todo!()
    }

    fn get_package_version_by_id(&self, _id: &str) -> anyhow::Result<Option<PackageVersion>> {
        todo!()
    }
}

fn fetch_namespace(
    md: &Metadata,
    index_name: &str,
    filter: HashMap<&str, &str>,
) -> anyhow::Result<Option<Namespace>> {
    match md.namespace_docs.fetch(index_name, filter) {
        Err(error) => Err(anyhow!("Error fetching namespace: {}", error)),
        Ok(Some(json)) => Ok(Some(Namespace::from_json_string(&json)?)),
        Ok(None) => Ok(None),
    }
}

fn insert_metadata<'a, T: Signed<'a> + Debug>(
    ds: &DocumentStore,
    signed: &T,
) -> anyhow::Result<()> {
    match signed.json() {
        Some(json) => match ds.insert(&json) {
            Ok(_) => Ok(()),
            Err(error) => Err(anyhow!(
                "Failed to create package_type record: {:?}\nError is {}",
                signed,
                error.to_string()
            )),
        },
        None => Err(anyhow!(
            "A supposedly trusted metadata struct is missing its JSON: {:?}",
            signed
        )),
    }
}

fn untrusted_metadata_error<'a, T: Signed<'a>>(signed: &T, error: &str) -> anyhow::Result<()> {
    Err(anyhow!(
        "New metadata is not trusted: JSON is {}\nError is{}",
        signed.json().unwrap_or("None".to_string()),
        error
    ))
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
        common_trust_logic(pkg_type)
    }

    fn trust_namespace(&self, namespace: &Namespace) -> Result<()> {
        common_trust_logic(namespace)
    }

    fn trust_package(&self, package: &Package) -> Result<()> {
        common_trust_logic(package)
    }

    fn trust_package_version(self, package_version: &PackageVersion) -> Result<()> {
        common_trust_logic(package_version)
    }
}

fn common_trust_logic<'a, T: Signed<'a>>(signed: &T) -> anyhow::Result<()> {
    match signed.json() {
        Some(_) => match signed.verify_signature() {
            Ok(attestations) => process_attestations(attestations),
            Err(error) => Err(error),
        },
        None => Err(anyhow!("Unsigned metadata")),
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

    // Used to run a test in a randomly named directory and then clean up by deleting the directory.
    fn do_in_temp_directory(runner: fn() -> anyhow::Result<()>) -> anyhow::Result<()> {
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

    #[test]
    #[serial]
    fn namespace_test() -> Result<()> {
        do_in_temp_directory(|| {
            let metadata = Metadata::new()?;
            info!("Created metadata instance: {:?}", metadata);

            let key_pair = create_key_pair(JwsSignatureAlgorithms::RS512)?;
            info!("Created key pair");

            let id = "asd928374".to_string();
            let path = "all/or/nothing".to_string();
            let timestamp = Some(iso8601::now_as_utc_iso8601_string());
            let mut namespace = Namespace::new(
                id.clone(),
                PackageTypeName::Docker,
                path.clone(),
                vec![],
                timestamp.clone(),
                timestamp.clone(),
            );

            namespace.sign_json(
                JwsSignatureAlgorithms::RS512,
                &key_pair.private_key,
                &key_pair.public_key,
            )?;
            metadata.create_namespace(&namespace)?;
            let namespace2 = metadata
                .get_namespace(PackageTypeName::Docker, &path)?
                .unwrap();
            assert_eq!(namespace2, namespace);
            let namespace3 = metadata.get_namespace_by_id(&id)?.unwrap();
            assert_eq!(namespace3, namespace2);
            assert!(metadata
                .get_namespace(PackageTypeName::Docker, &"BoGuS")?
                .is_none());
            assert!(metadata.get_namespace_by_id(&"BoGuS")?.is_none());
            Ok(())
        })
    }
}
