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

use crate::document_store::document_store::{DocumentStore, DocumentStoreError, IndexSpec};
use std::collections::HashMap;
use std::fmt::Debug;

use super::model::namespace::Namespace;
use super::model::package_type::{PackageType, PackageTypeName};
use super::model::package_version::PackageVersion;
use anyhow::{bail, Result};
use log::{error, info};
use maplit::hashmap;
use uuid::Uuid;

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
fn init_package_types() -> Result<Vec<String>> {
    let package_type = PackageType {
        id: Uuid::new_v4().to_string(),
        name: PackageTypeName::Docker,
        description: "docker packages".to_string(),
    };
    Ok(vec![serde_json::to_string(&package_type).unwrap_or_else(
        |_| "package type for pre-installation somehow does not have JSON".to_string(),
    )])
}

// Definitions for name spaces
const DS_NAMESPACES: &str = "namespaces";
const IX_NAMESPACES_ID: &str = "ids";
const IX_NAMESPACES_PATH: &str = "path";
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
    namespace_docs: DocumentStore,
    package_version_docs: DocumentStore,
}

#[derive(Debug)]
pub enum MetadataCreationStatus {
    Created,
    Duplicate { json: String },
}

impl Metadata {
    pub fn new() -> Result<Metadata, anyhow::Error> {
        info!("Creating new instance of metadata manager");
        let package_type_docs = DocumentStore::open(DS_PACKAGE_TYPES, ix_package_types())?;
        let namespace_docs = DocumentStore::open(DS_NAMESPACES, ix_namespaces())?;
        let package_version_docs = DocumentStore::open(DS_PACKAGE_VERSIONS, ix_package_versions())?;
        let metadata = Metadata {
            package_type_docs,
            namespace_docs,
            package_version_docs,
        };
        populate_with_initial_records(&metadata.package_type_docs, init_package_types)?;
        Ok(metadata)
    }

    pub fn create_package_type(
        &self,
        pkg_type: &PackageType,
    ) -> anyhow::Result<MetadataCreationStatus> {
        insert_metadata(&self.package_type_docs, serde_json::to_string(pkg_type)?)
    }

    pub fn get_package_type(&self, name: PackageTypeName) -> anyhow::Result<Option<PackageType>> {
        let name_as_string = name.to_string();
        let filter = hashmap! {
            FLD_PACKAGE_TYPES_NAME => name_as_string.as_str()
        };
        match self.package_type_docs.fetch(IX_PACKAGE_TYPES_NAMES, filter) {
            Err(error) => bail!("Error fetching package type: {}", error.to_string()),
            Ok(Some(json)) => Ok(Some(serde_json::from_str(&json)?)),
            Ok(None) => Ok(None),
        }
    }

    pub fn create_namespace(
        &self,
        namespace: &Namespace,
    ) -> anyhow::Result<MetadataCreationStatus> {
        insert_metadata(&self.namespace_docs, serde_json::to_string(&namespace)?)
    }

    pub fn get_namespace(
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

    pub fn create_package_version(
        &self,
        package_version: &PackageVersion,
    ) -> anyhow::Result<MetadataCreationStatus> {
        insert_metadata(
            &self.package_version_docs,
            serde_json::to_string(package_version)?,
        )
    }

    pub fn get_package_version(
        &self,
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

// Most types of metadata come from the Pyrsia network or the node's clients. However, a few types
// of metadata such as package type will need to be at partially pre-populated in new nodes.
fn populate_with_initial_records(
    ds: &DocumentStore,
    initial_records: fn() -> Result<Vec<String>>,
) -> Result<()> {
    for record in initial_records()? {
        info!(
            "Inserting in collection {} pre-installed record {}",
            ds.name(),
            record
        );
        match ds.insert(&record) {
            Ok(_) | Err(DocumentStoreError::DuplicateRecord(_)) => (), // Duplicates are OK
            Err(error) => {
                error!(
                    "Failed to insert initial record into document store {}: {}\nError was {}",
                    ds.name(),
                    record,
                    error.to_string(),
                );
                todo!("If an attempt to insert an initial record into document store fails, then we need to do something so that we will know that the document store is missing necessary information")
            }
        }
    }
    Ok(())
}

fn fetch_namespace(
    md: &Metadata,
    index_name: &str,
    filter: HashMap<&str, &str>,
) -> anyhow::Result<Option<Namespace>> {
    match md.namespace_docs.fetch(index_name, filter) {
        Err(error) => bail!("Error fetching namespace: {:?}", error),
        Ok(Some(json)) => Ok(Some(serde_json::from_str(&json)?)),
        Ok(None) => Ok(None),
    }
}

fn fetch_package_version(
    md: &Metadata,
    index_name: &str,
    filter: HashMap<&str, &str>,
) -> anyhow::Result<Option<PackageVersion>> {
    match md.package_version_docs.fetch(index_name, filter) {
        Err(error) => bail!("Error fetching package version: {}", error.to_string()),
        Ok(Some(json)) => Ok(Some(serde_json::from_str(&json)?)),
        Ok(None) => Ok(None),
    }
}

fn insert_metadata(ds: &DocumentStore, signed: String) -> anyhow::Result<MetadataCreationStatus> {
    match ds.insert(&signed) {
        Ok(_) => Ok(MetadataCreationStatus::Created),
        Err(DocumentStoreError::DuplicateRecord(record)) => {
            Ok(MetadataCreationStatus::Duplicate { json: record })
        }
        Err(error) => bail!(
            "Failed to create package_type record: {:?}\nError is {}",
            signed,
            error.to_string()
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::artifacts_repository::hash_util::HashAlgorithm;
    use crate::model::namespace::NamespaceBuilder;
    use crate::node_manager::handlers::METADATA_MGR;
    use crate::node_manager::model::artifact::ArtifactBuilder;
    use crate::node_manager::model::package_version::LicenseTextMimeType;
    use rand::RngCore;
    use serde_json::{Map, Value};

    #[test]
    fn package_type_test() -> Result<()> {
        let metadata = &METADATA_MGR;
        let mut package_type = PackageType {
            id: Uuid::new_v4().to_string(),
            name: PackageTypeName::Docker,
            description: "docker packages".to_string(),
        };
        // Because the Docker package type is pre-installed, we expect an attempt to add one to
        // produce a duplicate result.
        match metadata.create_package_type(&package_type)? {
            MetadataCreationStatus::Created => bail!("Docker package type is supposed to be pre-installed, but we were just able to create it!"),
            MetadataCreationStatus::Duplicate{ json: _} => Ok(())
        }
    }

    #[test]
    fn namespace_test() -> Result<()> {
        let metadata = &METADATA_MGR;

        let id = Uuid::new_v4().to_string();
        let path = append_random("all/or/nothing");
        let mut namespace = Namespace {
            id,
            package_type: PackageTypeName::Docker,
            namespace_path: path,
        };
        match metadata.create_namespace(&namespace)? {
            MetadataCreationStatus::Created => {
                let namespace2 = metadata
                    .get_namespace(PackageTypeName::Docker, &path)?
                    .unwrap();
                assert_eq!(namespace2, namespace);
            }
            MetadataCreationStatus::Duplicate { json: _ } => (),
        }
        Ok(())
    }

    #[test]
    fn package_version_test() -> Result<()> {
        let metadata = &METADATA_MGR;
        info!("Got metadata instance");

        let hash1: Vec<u8> = vec![
            0xa3, 0x3f, 0x49, 0x64, 0x00, 0xa5, 0x67, 0xe1, 0xb4, 0xe5, 0xbe, 0x4c, 0x81, 0x30,
            0xd7, 0xd3, 0x5f, 0x67, 0x7a, 0x41, 0xff, 0xca, 0x25, 0xe5, 0x5c, 0x66, 0xde, 0xbf,
            0x42, 0xfe, 0xc5, 0xc0,
        ];
        let name1 = "roadRunner".to_string();
        let url1 = "https://example.com".to_string();
        let size1: u64 = 12345678;
        let mime_type1 = "application/binary".to_string();
        let source1 = "https://info.com".to_string();
        let artifacts = vec![ArtifactBuilder::default()
            .hash(hash1)
            .algorithm(HashAlgorithm::SHA256)
            .name(name1)
            .url(url1)
            .size(size1)
            .mime_type(mime_type1)
            .metadata(Map::new())
            .source_url(source1)
            .build()?];

        let id = append_random("id");
        let namespace_id = append_random("NS");
        let name = append_random("name");
        let package_type = PackageTypeName::Docker;
        let version = "1.0".to_string();
        let license_text = "Do as you will.".to_string();
        let license_text_mimetype = LicenseTextMimeType::Text;
        let license_url = "https://example.com".to_string();
        let pv_metadata: serde_json::Map<String, Value> = serde_json::Map::new();
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
            .tags(tags)
            .description(description)
            .artifacts(artifacts)
            .build()?;
        let key_pair = metadata.untrusted_key_pair();
        package_version.sign_json(algorithm, &key_pair.private_key, &key_pair.public_key)?;

        match metadata.create_package_version(&package_version)? {
            MetadataCreationStatus::Created => (),
            status => panic!(
                "Expected metadata status to be created but found {:?}",
                status
            ),
        }

        let fetched_package_version2 =
            metadata.get_package_version(&namespace_id, &name, &version)?;
        assert!(fetched_package_version2.is_some());
        assert_eq!(package_version, fetched_package_version2.unwrap());

        Ok(())
    }

    fn append_random(name: &str) -> String {
        let mut rng = rand::thread_rng();
        format!("{}{}", name, rng.next_u32())
    }
}
