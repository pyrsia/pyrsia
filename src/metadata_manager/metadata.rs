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

use crate::document_store::document_store::{DocumentStore, IndexSpec};
use std::collections::HashMap;
use std::fmt::Debug;

use super::model::namespace::Namespace;
use super::model::package::Package;
use super::model::package_type::{PackageType, PackageTypeBuilder, PackageTypeName};
use super::model::package_version::{PackageVersion, PackageVersionBuilder};
use anyhow::{bail, Result};
use log::{error, info, warn};
use maplit::hashmap;
use serial_test::serial;
use signed::signed::Signed;

// create package version

pub trait MetadataApi {
    /// Create a new package type with the information specified in the `pkg_type` parameter.
    ///
    /// Returns an error if `pkg_type` does not have any valid signatures or i any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    ///
    /// Also returns an error if there is already package_type with the same name.
    fn create_package_type(&mut self, pkg_type: &PackageType) -> Result<()>;

    /// Return a PackageType struct that describes the named package type.
    fn get_package_type(&mut self, name: PackageTypeName) -> Result<Option<PackageType>>;

    /// Define the package version described by the given `PackageVersion` struct.
    ///
    /// Returns an error if there is already a package version with the same id or the same
    /// combination of package_type, namespace_id, package_name and version.
    ///
    /// Returns an error if `package_version` does not have any valid signatures or if any of the valid
    /// signatures are associated with a public key that does not identify an identity in the blockchain.
    fn create_package_version(&mut self, package_version: &PackageVersion) -> Result<()>;

    /// Get the package_version that matches the given namespace_id, package_name and version.
    fn get_package_version(
        &mut self,
        namespace_id: &str,
        package_name: &str,
        version: &str,
    ) -> Result<Option<PackageVersion>>;
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
// Each set of definitions has some const declarations followed by one or two function definitions.
// The first const is for the name of the document store. The names of these follow the pattern
// DS_documentStoreName. For example the const for the name of the document store named
// "package_versions" is DS_PACKAGE_VERSIONS.
//
// Each document store will have at least one index. Each index has a name. The pattern for const
// names that define the name of an index is IX_documentStoreName_indexName. For example, the name
// of the const for an index named "ids" for a document store named "package_versions" is
// IX_PACKAGE_VERSION_IDS. There will be one of these IX consts for each index that a document store
// will have.
//
// Since we need to specify which fields an index will refer to, there are also const names defined
// for each field that is covered by an index. The pattern for these names is
// FLD_documentStoreName_fieldName. For example the const name for the field named "name" in the
// document store named "package_versions" is FLD_PACKAGE_VERSIONS_NAME.
//
// For each document store to be created, there is a function named with the pattern
// ix_documentStoreName. For example, for the document store named "package_versions" the name of
// this function is ix_package_versions. This function uses the const values defined for the
// document store to create and return a Vec of index definitions.
//
// For some of the document stores there is a second function named with the pattern
// init_documentStoreName. For example, there is one of these functions for the document store named
// "package_types". The name of the function is init_package_types. This method creates a Vec of
// JSON strings that the document store will be pre-populated with.
//
////////////////////////////////////////////////////////////////////////////////////////////////////

// Definitions for package types
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

fn init_empty() -> Vec<String> {
    vec![]
}

// Definitions for package-versions
const DS_PACKAGE_VERSIONS: &str = "package_versions";
const IX_PACKAGE_VERSIONS_ID: &str = "id";
const IX_PACKAGE_VERSIONS_VERSION: &str = "version";
const FLD_PACKAGE_VERSIONS_ID: &str = "id";
const FLD_PACKAGE_VERSIONS_NAMESPACE_ID: &str = "namespace_id";
const FLD_PACKAGE_VERSIONS_NAME: &str = "name";
const FLD_PACKAGE_VERSIONS_VERSION: &str = "version";
fn ix_package_versions() -> Vec<IndexSpec> {
    vec![
        IndexSpec::new(
            String::from(IX_PACKAGE_VERSIONS_ID),
            vec![String::from(FLD_PACKAGE_VERSIONS_ID)],
        ),
        IndexSpec::new(
            String::from(IX_PACKAGE_VERSIONS_VERSION),
            vec![
                String::from(FLD_PACKAGE_VERSIONS_NAMESPACE_ID),
                String::from(FLD_PACKAGE_VERSIONS_NAME),
                String::from(FLD_PACKAGE_VERSIONS_VERSION),
            ],
        ),
    ]
}

////////////////////////////////////////////////////////////////////////////////////////////////////
// End of definitions to support creation of document stores
////////////////////////////////////////////////////////////////////////////////////////////////////

pub struct Metadata {
    //TODO Add a trust manager to decide if metadata is trust-worthy
    package_type_docs: DocumentStore,
    package_version_docs: DocumentStore,
}

impl Metadata {
    pub fn new() -> Result<Metadata, anyhow::Error> {
        info!("Creating new instance of metadata manager");
        let package_type_docs =
            open_document_store(DS_PACKAGE_TYPES, ix_package_types, init_package_types)?;
        let package_version_docs =
            open_document_store(DS_PACKAGE_VERSIONS, ix_package_versions, init_empty)?;
        Ok(Metadata {
            package_type_docs,
            package_version_docs,
        })
    }
}

// Open the specified document store. If that fails, try creating it.
// * `ds_name` ― The name of the document store.
// * `index_specs` ― If creating the document store, call this method to get a description of the indexes it will have.
// * `initial_records` ― When creating the document store, call this method to get JSON strings to inserted as a records.
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
                Ok(mut ds) => {
                    info!("Created document store {}", ds_name);
                    populate_with_initial_records(&mut ds, initial_records)?;
                    Ok(ds)
                }
                Err(error) => failed_to_create_document_store(ds_name, error),
            }
        }
    }
}

// Most types of metadata come from the Pyrsia network or the node's clients. However, a few types
// of metadata such as package type will need to be at partially pre-populated in new nodes.
fn populate_with_initial_records(
    ds: &mut DocumentStore,
    initial_records: fn() -> Vec<String>,
) -> Result<()> {
    for record in initial_records() {
        if let Err(error) = ds.insert(&record) {
            error!(
                "Failed to insert initial record into document store {}: {}\nError was {}",
                ds.name(),
                record,
                error
            );
            todo!("If an attempt to insert an initial record into document store fails, then we need to do something so that we will know that the document store is missing necessary information")
        }
    }
    Ok(())
}

fn failed_to_create_document_store(ds_name: &str, error: anyhow::Error) -> Result<DocumentStore> {
    let msg = format!(
        "Failed to create document store {} due to error {}",
        ds_name, error
    );
    error!("{}", msg);
    bail!(msg)
}

impl MetadataApi for Metadata {
    fn create_package_type(&mut self, pkg_type: &PackageType) -> anyhow::Result<()> {
        insert_metadata(&mut self.package_type_docs, pkg_type)
    }

    fn get_package_type(&mut self, name: PackageTypeName) -> anyhow::Result<Option<PackageType>> {
        let name_as_string = format!("{}", name.to_string());
        let filter = hashmap! {
            FLD_PACKAGE_TYPES_NAME => name_as_string.as_str()
        };
        match self.package_type_docs.fetch(IX_PACKAGE_TYPES_NAMES, filter) {
            Err(error) => bail!("Error fetching package type: {}", error),
            Ok(Some(json)) => Ok(Some(PackageType::from_json_string(&json)?)),
            Ok(None) => Ok(None),
        }
    }

    fn create_package_version(&mut self, package_version: &PackageVersion) -> anyhow::Result<()> {
        insert_metadata(&mut self.package_version_docs, package_version)
    }

    fn get_package_version(
        &mut self,
        namespace_id: &str,
        package_name: &str,
        version: &str,
    ) -> Result<Option<PackageVersion>> {
        let filter = hashmap! {
            FLD_PACKAGE_VERSIONS_NAMESPACE_ID => namespace_id,
            FLD_PACKAGE_VERSIONS_NAME => package_name,
            FLD_PACKAGE_VERSIONS_VERSION => version,
        };
        fetch_package_version(self, IX_PACKAGE_VERSIONS_VERSION, filter)
    }
}

fn fetch_package_version(
    md: &mut Metadata,
    index_name: &str,
    filter: HashMap<&str, &str>,
) -> anyhow::Result<Option<PackageVersion>> {
    match md.package_version_docs.fetch(index_name, filter) {
        Err(error) => bail!("Error fetching package version: {}", error),
        Ok(Some(json)) => Ok(Some(PackageVersion::from_json_string(&json)?)),
        Ok(None) => Ok(None),
    }
}

fn insert_metadata<'a, T: Signed<'a> + Debug>(
    ds: &mut DocumentStore,
    signed: &T,
) -> anyhow::Result<()> {
    match signed.json() {
        Some(json) => match ds.insert(&json) {
            Ok(_) => Ok(()),
            Err(error) => bail!(
                "Failed to create package_type record: {:?}\nError is {}",
                signed,
                error.to_string()
            ),
        },
        None => bail!(
            "A supposedly trusted metadata struct is missing its JSON: {:?}",
            signed
        ),
    }
}

fn untrusted_metadata_error<'a, T: Signed<'a>>(signed: &T, error: &str) -> anyhow::Result<()> {
    bail!(
        "New metadata is not trusted: JSON is {}\nError is{}",
        signed.json().unwrap_or_else(|| "None".to_string()),
        error
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifact_manager::HashAlgorithm;
    use crate::node_manager::model::artifact::ArtifactBuilder;
    use crate::node_manager::model::package_version::LicenseTextMimeType;
    use rand::Rng;
    use serde_json::{Map, Value};
    use signed::signed;
    use std::path::Path;
    use std::{env, fs};

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
            let mut metadata = Metadata::new()?;
            info!("Created metadata instance");

            let mut package_type = PackageTypeBuilder::default()
                .name(PackageTypeName::Docker)
                .description("docker packages".to_string())
                .build()?;
            let algorithm = signed::JwsSignatureAlgorithms::RS384;
            let key_pair = signed::create_key_pair(algorithm)?;
            package_type.sign_json(algorithm, &key_pair.private_key, &key_pair.public_key);
            metadata.create_package_type(&package_type)?;
            let package_type2 = metadata.get_package_type(PackageTypeName::Docker)?.unwrap();
            assert_eq!(package_type2, package_type);
            Ok(())
        })
    }

    #[test]
    #[serial]
    fn package_version_test() -> Result<()> {
        let mut metadata = Metadata::new()?;

        let hash1: Vec<u8> = vec![
            0xa3, 0x3f, 0x49, 0x64, 0x00, 0xa5, 0x67, 0xe1, 0xb4, 0xe5, 0xbe, 0x4c, 0x81, 0x30,
            0xd7, 0xd3, 0x5f, 0x67, 0x7a, 0x41, 0xff, 0xca, 0x25, 0xe5, 0x5c, 0x66, 0xde, 0xbf,
            0x42, 0xfe, 0xc5, 0xc0,
        ];
        let name1 = "roadRunner".to_string();
        let creation_time1 = signed::now_as_iso8601_string();
        let url1 = "https://example.com".to_string();
        let size1: u64 = 12345678;
        let mime_type1 = "application/binary".to_string();
        let source1 = "https://info.com".to_string();
        let artifacts = vec![ArtifactBuilder::default()
            .hash(hash1)
            .algorithm(HashAlgorithm::SHA256)
            .name(name1)
            .creation_time(creation_time1)
            .url(url1)
            .size(size1)
            .mime_type(mime_type1)
            .metadata(Map::new())
            .source_url(source1)
            .build()?];

        let id = "wi238rugs".to_string();
        let namespace_id = "asd928374".to_string();
        let name = "acme".to_string();
        let package_type = PackageTypeName::Docker;
        let version = "1.0".to_string();
        let license_text = "Do as you will.".to_string();
        let license_text_mimetype = LicenseTextMimeType::Text;
        let license_url = "https://example.com".to_string();
        let pv_metadata: serde_json::Map<String, Value> = serde_json::Map::new();
        let creation_time = signed::now_as_iso8601_string();
        let modified_time = signed::now_as_iso8601_string();
        let tags: Vec<String> = vec![];
        let description = "Roses are red".to_string();

        let mut package_version: PackageVersion = PackageVersionBuilder::default()
            .id(id)
            .namespace_id(namespace_id.clone())
            .name(name.clone())
            .pkg_type(package_type)
            .version(version.clone())
            .license_text(license_text)
            .license_text_mimetype(license_text_mimetype)
            .license_url(license_url)
            .metadata(pv_metadata)
            .creation_time(creation_time)
            .modified_time(modified_time)
            .tags(tags)
            .description(description)
            .artifacts(artifacts)
            .build()?;
        let algorithm = signed::JwsSignatureAlgorithms::RS384;
        let key_pair = signed::create_key_pair(algorithm)?;
        package_version.sign_json(algorithm, &key_pair.private_key, &key_pair.public_key);

        metadata.create_package_version(&package_version)?;
        let package_version2 = metadata.get_package_version(&namespace_id, &name, &version)?;
        assert!(package_version2.is_some());
        assert_eq!(package_version, package_version2.unwrap());

        Ok(())
    }
}
