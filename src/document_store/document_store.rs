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

//! The DocumentStore is capable of storing documents in the form
//! of JSON objects. The DocumentStore is backed by a data store
//! where these documents are persistently written to.
//!
//! A DocumentStore is defined by a name and one or more index
//! specifications. An index specification is defined again by a
//! name and a list of fields that make up the index. Together, this
//! is known as the catalog.
//!
//! To create and persist the catalog, we call [`DocumentStore::create`].
//!
//! A DocumentStore can be get via its name by calling [`DocumentStore::get`].
//!
//! To store a document, use [`DocumentStore.insert`]. Fetching a document
//! via an index can be done with [`DocumentStore.fetch`].
//!

use bincode;
use log::{debug, error, info, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::str;
use thiserror::Error;
use unqlite::{Transaction, UnQLite, KV};

/// Defines the sorting order when storing the values associated
/// with an index.
/// Note: This is currently not yet implemented and at the moment,
/// all IndexSpec will default to IndexOrder::Asc.
#[derive(Debug, Deserialize, Serialize, PartialEq, Clone)]
pub enum IndexOrder {
    /// Fetching a range of values using the index returns
    /// the lowest value first and the highest value last.
    Asc,
    /// Fetching a range of values using the index returns
    /// the highest value first and the lowest value last.
    Desc,
}

/// The definition of an index in the document store.
#[derive(Debug, Deserialize, Serialize, Clone, PartialEq)]
pub struct IndexSpec {
    pub name: String,
    pub field_names: Vec<String>,
    direction: IndexOrder,
}

/// The document store is able to store and fetch documents
/// with a list of predefined indexes.
pub struct DocumentStore {
    unqlite: UnQLite,
    catalog: Catalog,
}

impl DocumentStore {
    /// Open/create a DocumentStore for a collection of records with the given `name` and a Vec of
    /// `indexes` that is used to create.
    pub fn open(
        name: &str,
        indexes: Vec<IndexSpec>,
    ) -> anyhow::Result<DocumentStore, DocumentStoreError> {
        info!(
            "Opening DocumentStore with name {} and indexes {:?}",
            name, indexes
        );
        check_index_specs_valid(&indexes)?;
        let document_store = create_document_store(name, &indexes);
        match get_catalog_record(&document_store) {
            Ok(_) => info!(
                "Opened document store collection: {}",
                document_store.catalog.name
            ),
            Err(error) => {
                info!("Failed to get catalog record. Assuming that collection {} is new. Creating catalog record. Error was {}", document_store.catalog.name, error.to_string());
                let serialized_catalog = bincode::serialize(&document_store.catalog)?;
                document_store
                    .unqlite
                    .kv_store(serialized_catalog_key()?, serialized_catalog)
                    .map_err(DocumentStoreError::UnQLite)?;
                debug!("Created catalog record for {}", document_store.catalog.name)
            }
        };
        Ok(document_store)
    }

    pub fn name(&self) -> &str {
        &self.catalog.name
    }
}

fn create_document_store(name: &str, indexes: &[IndexSpec]) -> DocumentStore {
    let index_count_u16 = match u16::try_from(indexes.len()) {
        Ok(count) => count,
        Err(_) => panic!(
            "Requested document store \"{}\" is configured for more than {} indexes",
            name,
            u16::MAX
        ),
    };
    DocumentStore {
        catalog: Catalog {
            name: name.to_string(),
            indexes: (0..index_count_u16).zip(indexes.iter().cloned()).collect(),
        },
        unqlite: UnQLite::create(collection_name_to_file_name(name)),
    }
}

fn check_index_specs_valid(indexes: &[IndexSpec]) -> anyhow::Result<(), DocumentStoreError> {
    if indexes.is_empty() {
        Err(DocumentStoreError::Custom(
            "At least one index specification is required when creating a DocumentStore."
                .to_string(),
        ))
    } else {
        Ok(())
    }
}

fn collection_name_to_file_name(name: &str) -> String {
    let mut s = name.to_string();
    s.push_str(".db");
    s
}

fn get_catalog_record(
    document_store: &DocumentStore,
) -> anyhow::Result<Catalog, DocumentStoreError> {
    let raw_key = serialized_catalog_key()?;
    let raw_doc_store = document_store
        .unqlite
        .kv_fetch(raw_key)
        .map_err(DocumentStoreError::UnQLite)?;
    let catalog: Catalog = bincode::deserialize(&raw_doc_store)?;
    if catalog != document_store.catalog {
        warn!("Stored catalog fof document store collection {} is different than expected. This may cause future errors.", catalog.name)
    }
    Ok(catalog)
}

fn serialized_catalog_key() -> anyhow::Result<Vec<u8>, DocumentStoreError> {
    bincode::serialize(&CatalogKey::new())
        .map_err(|err| DocumentStoreError::KeyCreation(err.to_string()))
}

// A description of a collection of documents and how they are indexed.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Catalog {
    name: String,
    indexes: Vec<(u16, IndexSpec)>,
}

const KEYTYPE_CATALOG: u8 = 0b00000001;
const KEYTYPE_DATA: u8 = 0b00000010;
const KEYTYPE_INDEX: u8 = 0b00000011;

// A key that is associated with the metadata of the
// document store. It is identified by [KEYTYPE_CATALOG].
#[derive(Debug, Deserialize, Serialize)]
struct CatalogKey {
    key_type: u8,
}

// A key that is associated with the a document in the
// document store. It is identified by [KEYTYPE_DATA].
#[derive(Debug, Deserialize, Serialize)]
struct DataKey {
    key_type: u8,
    number: u128,
}

// A key that is associated with the a stored index in
#[derive(Debug, Deserialize, Serialize, Clone)]
struct IndexKey {
    key_type: u8,
    index: u16,
    values: Vec<String>,
}

impl IndexSpec {
    /// Creates a new index specification.
    pub fn new<I, T>(name: T, field_names: I) -> IndexSpec
    where
        I: IntoIterator<Item = T>,
        T: Into<String>,
    {
        IndexSpec {
            name: name.into(),
            field_names: field_names.into_iter().map(Into::into).collect(),
            direction: IndexOrder::Asc, // hardcode to Asc until implemented
        }
    }

    // Extracts the index values from the provided JSON object.
    fn get_index_values(
        &self,
        json_document: &Value,
    ) -> anyhow::Result<Vec<String>, DocumentStoreError> {
        let mut values: Vec<String> = vec![];

        for field_name in &self.field_names {
            if let Some(value) = json_document.get(&field_name) {
                if let Some(json_string) = value.as_str() {
                    values.push(json_string.to_string())
                } else {
                    return Err(DocumentStoreError::KeyValueIsNotAString {
                        collection_name: self.name.to_string(),
                        field_name: field_name.to_string(),
                        value: value.clone(),
                    });
                }
            } else {
                return Err(DocumentStoreError::MissingKeyField {
                    collection_name: self.name.to_string(),
                    field_name: field_name.to_string(),
                });
            }
        }

        Ok(values)
    }
}

/// The DocumentStoreError acts as a wrapper around all types of
/// errors that can occur while working with a document store.
#[derive(Debug, Error)]
pub enum DocumentStoreError {
    #[error("Bincode Error: {0}")]
    Bincode(bincode::Error),
    #[error("Json Error: {0}")]
    Json(serde_json::Error),
    #[error("UnQLite Error: {0}")]
    UnQLite(unqlite::Error),
    #[error("DocumentStore Error: {0}")]
    Custom(String),
    #[error("Attempted to insert duplicate of record {0}")]
    DuplicateRecord(String),
    #[error("Failed to create the key for the catalog record {0}")]
    KeyCreation(String),
    #[error("Document has required index key {collection_name}.{field_name}, but is not a JSON string: {value}.")]
    KeyValueIsNotAString {
        collection_name: String,
        field_name: String,
        value: Value,
    },
    #[error("Document is missing required index key {collection_name}.{field_name}")]
    MissingKeyField {
        collection_name: String,
        field_name: String,
    },
    #[error("DocumentStore found an index entry for {values:?} in index {index_name} using raw index key {raw_index_key:?} pointing to record {raw_data_key:?}, but failed to find Document with key {document_key}.")]
    IndexOrphan {
        values: Vec<String>,
        index_name: String,
        raw_index_key: Vec<u8>,
        raw_data_key: Vec<u8>,
        document_key: u128,
    },
    #[error("Document string at {0:?} is not valid UTF8: {1}")]
    NotUtf8(Vec<u8>, String),
    #[error("Failed to serialize index key {0:?}")]
    UnserializableIndexKey(Vec<String>),
}

impl From<bincode::Error> for DocumentStoreError {
    fn from(err: bincode::Error) -> DocumentStoreError {
        DocumentStoreError::Bincode(err)
    }
}

impl From<serde_json::Error> for DocumentStoreError {
    fn from(err: serde_json::Error) -> DocumentStoreError {
        DocumentStoreError::Json(err)
    }
}

impl From<unqlite::Error> for DocumentStoreError {
    fn from(err: unqlite::Error) -> DocumentStoreError {
        DocumentStoreError::UnQLite(err)
    }
}

impl CatalogKey {
    fn new() -> CatalogKey {
        CatalogKey {
            key_type: KEYTYPE_CATALOG,
        }
    }
}

impl DataKey {
    fn new(number: u128) -> DataKey {
        DataKey {
            key_type: KEYTYPE_DATA,
            number,
        }
    }
}

impl IndexKey {
    fn new(index: u16, values: Vec<String>) -> IndexKey {
        IndexKey {
            key_type: KEYTYPE_INDEX,
            index,
            values,
        }
    }
}

impl DocumentStore {
    /// Insert the provided JSON document in the DocumentStore. The JSON document
    /// must be a JSON Object. The document will be written together with the
    /// document's index values. Each field for every index that is specified in
    /// the DocumentStore must be present in the document.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] in the following cases:
    ///
    ///  * `document` is not a JSON Object
    ///  * the JSON object doesn't contain all the specified index fields
    ///  * the document or its index values could not be persisted in the data store
    pub fn insert(&self, document: &str) -> anyhow::Result<(), DocumentStoreError> {
        let json_document = serde_json::from_str::<Value>(document)?;
        if !json_document.is_object() {
            return Err(DocumentStoreError::Custom(
                "Provided JSON document must represent a JSON Object rather than an array or other type of JSON data.".to_string(),
            ));
        }
        self.unqlite.begin().map_err(DocumentStoreError::UnQLite)?;

        match store_document(self, document) {
            Ok(raw_data_key) => {
                for index in &self.catalog.indexes {
                    if let Err(e) =
                        process_index(&self.unqlite, &json_document, &raw_data_key, index)
                    {
                        self.unqlite
                            .rollback()
                            .map_err(DocumentStoreError::UnQLite)?;
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                self.unqlite
                    .rollback()
                    .map_err(DocumentStoreError::UnQLite)?;
                return Err(e);
            }
        }

        self.unqlite.commit().map_err(DocumentStoreError::UnQLite)?;

        Ok(())
    }

    /// Fetches a document from the database by searching on the index
    /// with the specified `index_name` and `filter`. The filter is a
    /// map where the keys represent each field name of the specified
    /// index and the values represent the associated values of the keys
    /// in the document to fetch.
    pub fn fetch(
        &self,
        index_name: &str,
        filter: HashMap<&str, &str>,
    ) -> anyhow::Result<Option<String>, DocumentStoreError> {
        let index_to_use = find_index(self, index_name)?;
        let compound_key = build_compound_key(index_name, filter, index_to_use.1)?;

        let index_key = IndexKey::new(index_to_use.0, compound_key);
        fetch_indexed_record(index_name, &self.unqlite, &index_key)
    }
}

fn find_index<'a>(
    ds: &'a DocumentStore,
    index_name: &str,
) -> anyhow::Result<(u16, &'a IndexSpec), DocumentStoreError> {
    match ds
        .catalog
        .indexes
        .iter()
        .find(|index| index.1.name == index_name)
    {
        Some((position, index_to_use)) => Ok((*position, index_to_use)),
        None => Err(DocumentStoreError::Custom(format!(
            "DocumentStore {} has no index with given name: {}",
            ds.catalog.name, index_name
        ))),
    }
}

// Store the provided JSON `document` in the provided data store
// and return the raw data key that was used as the key that
// uniquely identifies the document.
fn store_document(
    ds: &DocumentStore,
    document: &str,
) -> anyhow::Result<Vec<u8>, DocumentStoreError> {
    let mut rng = rand::thread_rng();

    loop {
        let raw_data_key = bincode::serialize(&DataKey::new(rng.gen()))?;
        if !ds.unqlite.kv_contains(&raw_data_key) {
            ds.unqlite
                .kv_store(&raw_data_key, document)
                .map_err(DocumentStoreError::UnQLite)?;
            debug!("Document stored!");
            return Ok(raw_data_key);
        }
    }
}

fn build_compound_key(
    index_name: &str,
    filter: HashMap<&str, &str>,
    index_to_use: &IndexSpec,
) -> anyhow::Result<Vec<String>, DocumentStoreError> {
    let mut compound_key: Vec<String> = vec![];
    for field_name in &index_to_use.field_names {
        if let Some(value) = filter.get(field_name as &str) {
            compound_key.push(value.to_string());
        } else {
            return Err(DocumentStoreError::Custom(format!(
                "Filter is missing value for field {} required by index {}.",
                field_name, index_name
            )));
        }
    }
    Ok(compound_key)
}

fn fetch_indexed_record(
    index_name: &str,
    unqlite: &UnQLite,
    index_key: &IndexKey,
) -> anyhow::Result<Option<String>, DocumentStoreError> {
    let raw_index_key = bincode::serialize(&index_key)?;
    if let Ok(raw_data_key) = unqlite.kv_fetch(&raw_index_key) {
        fetch_json_record(index_name, unqlite, index_key, raw_index_key, raw_data_key)
    } else {
        Ok(None)
    }
}

fn fetch_json_record(
    index_name: &str,
    unqlite: &UnQLite,
    index_key: &IndexKey,
    raw_index_key: Vec<u8>,
    raw_data_key: Vec<u8>,
) -> Result<Option<String>, DocumentStoreError> {
    if let Ok(raw_document) = unqlite.kv_fetch(&raw_data_key) {
        let document = bytes_to_utf8(raw_data_key, raw_document)?;
        Ok(Some(document))
    } else {
        let data_key: DataKey = bincode::deserialize(&raw_data_key)?;
        let err = DocumentStoreError::IndexOrphan {
            values: index_key.values.clone(),
            index_name: index_name.to_string(),
            raw_index_key,
            raw_data_key,
            document_key: data_key.number,
        };
        error!("{:?}", err);
        Err(err)
    }
}

fn bytes_to_utf8(raw_key: Vec<u8>, raw_document: Vec<u8>) -> Result<String, DocumentStoreError> {
    String::from_utf8(raw_document.clone()).map_err(move |_| {
        let lossy = String::from_utf8_lossy(&raw_document).to_string();
        warn!(
            "Contents of record with key {:?} is not valid utf8: {:?}",
            raw_key, &lossy
        );
        DocumentStoreError::NotUtf8(raw_key, lossy)
    })
}

// Process the `index` by parsing the index values from the
// `json_document` and store these index values in the provided data
// store.
fn process_index(
    data_store: &UnQLite,
    json_document: &Value,
    raw_data_key: &[u8],
    index: &(u16, IndexSpec),
) -> anyhow::Result<(), DocumentStoreError> {
    let index_values = index.1.get_index_values(json_document)?;
    store_index(data_store, raw_data_key, index.0, index_values)?;
    Ok(())
}

// Store the `raw_data_key` (that points to a document) associated
// with the `index_values` into the provided data store. The key
// will be an `IndexKey` that is built from the `index` and
// `index_values`. The value will be the `raw_data_key` itself.
fn store_index(
    unqlite: &UnQLite,
    raw_data_key: &[u8],
    index: u16,
    index_values: Vec<String>,
) -> anyhow::Result<(), DocumentStoreError> {
    let index_key: IndexKey = IndexKey::new(index, index_values);
    let raw_index_key = bincode::serialize(&index_key)
        .map_err(|_| DocumentStoreError::UnserializableIndexKey(index_key.values))?;
    match unqlite.kv_fetch(&raw_index_key) {
        Ok(record_key) => {
            // handle a duplicate
            let raw_record = match unqlite.kv_fetch(record_key.clone()) {
                Ok(raw_record) => raw_record,
                Err(error) => {
                    error!("While handling an attempt to insert a record, discovered it would create a duplicate index record. Attempted to retrieve the JSON record referenced by the index record, but failed: {}", error);
                    "*** JSON record referenced by duplicate record could not be fetched ***"
                        .as_bytes()
                        .to_owned()
                }
            };
            let record = bytes_to_utf8(record_key, raw_record)?;
            Err(DocumentStoreError::DuplicateRecord(record))
        }
        Err(_) => store_new_index_record(unqlite, raw_data_key, &raw_index_key),
    }
}

fn store_new_index_record(
    unqlite: &UnQLite,
    raw_data_key: &[u8],
    raw_index_key: &[u8],
) -> Result<(), DocumentStoreError> {
    unqlite
        .kv_store(&raw_index_key, raw_data_key)
        .map_err(DocumentStoreError::UnQLite)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use anyhow::bail;
    use rand::RngCore;
    use serde_json::json;
    use test_log::test;

    // Create a database with a name and an empty index list
    #[test]
    fn test_create_without_indexes() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_create_without_indexes");
        let name = path.to_str().unwrap();
        assert!(
            DocumentStore::open(name, vec![]).is_err(),
            "should not have been created"
        );
    }

    // Create a database with a name and two index specifications
    #[test]
    fn test_create_with_indexes() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_create_with_indexes");
        let name = path.to_str().unwrap();
        let index_one = "index_one";
        let index_two = "index_two";
        let field1 = "mostSignificantField";
        let field2 = "leastSignificantField";
        let idx1 = IndexSpec::new(index_one, vec![field1]);
        let idx2 = IndexSpec::new(index_two, vec![field2]);
        let indexes = vec![idx1, idx2];

        let doc_store = DocumentStore::open(name, indexes).expect("should not result in error");
        assert_eq!(doc_store.catalog.indexes[0].1.name, "index_one".to_string());
        assert_eq!(
            doc_store.catalog.indexes[0].1.field_names,
            vec![field1.to_string()]
        );
        assert_eq!(doc_store.catalog.indexes[0].1.direction, IndexOrder::Asc);
        assert_eq!(doc_store.catalog.indexes[1].1.name, "index_two".to_string());
        assert_eq!(
            doc_store.catalog.indexes[1].1.field_names,
            vec![field2.to_string()]
        );
        assert_eq!(doc_store.catalog.indexes[1].1.direction, IndexOrder::Asc);
    }

    #[test]
    fn test_insert() -> anyhow::Result<()> {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_insert");
        let name = path.to_str().unwrap();
        let forward_index = "forwards";
        let backwards_index = "backwards";
        let field1 = "mostSignificantField";
        let field2 = "leastSignificantField";
        let i1 = IndexSpec::new(forward_index, vec![field1, field2]);
        let i2 = IndexSpec::new(backwards_index, vec![field2, field1]);
        let indexes = vec![i1, i2];

        let doc_store = DocumentStore::open(name, indexes).expect("should not result in error");

        let doc1 = json!({
            "mostSignificantField": "msf1",
            "leastSignificantField": "12",
            "insignificant": 0
        });
        let doc2 = json!({
            "mostSignificantField": "msf1",
            "leastSignificantField": "12",
            "insignificant": 2
        });
        let doc1_string = doc1.to_string();
        doc_store.insert(&doc1_string)?;
        let mut filter = HashMap::new();
        filter.insert(field1, "msf1");
        filter.insert(field2, "12");
        let fetched = doc_store.fetch(forward_index, filter)?.unwrap();
        assert_eq!(doc1_string, fetched);
        match doc_store.insert(&doc2.to_string()) {
            Ok(_) => bail!("Attempt to add a duplicate record succeeded."),
            Err(DocumentStoreError::DuplicateRecord(idx)) => assert_eq!(doc1_string, idx),
            Err(other) => bail!(
                "Unexpected error from inserting a duplicate record {:?}",
                other
            ),
        }
        Ok(())
    }

    #[test]
    fn test_store_missing_index_field() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store_invalid_json");
        let name = path.to_str().unwrap();
        let doc_store =
            DocumentStore::open(name, vec![IndexSpec::new("index", vec!["index_field"])])
                .expect("should not result in error");

        let doc = json!({
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        doc_store
            .insert(&doc.to_string())
            .expect_err("should not store with missing index fields.");
    }

    #[test]
    fn test_store_invalid_json() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store_invalid_json");
        let name = path.to_str().unwrap();
        let doc_store = DocumentStore::open(name, vec![IndexSpec::new("index", vec!["field"])])
            .expect("should not result in error");

        doc_store
            .insert(&String::from("{\"mostSignificantField\":\"value\""))
            .expect_err("should not store invalid json.");
    }

    #[test]
    fn test_store_non_json_object() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store_non_json_object");
        let name = path.to_str().unwrap();
        let doc_store = DocumentStore::open(name, vec![IndexSpec::new("index", vec!["field"])])
            .expect("should not result in error");

        doc_store
            .insert(&String::from("[{\"mostSignificantField\":\"value\"}]"))
            .expect_err("should not store non json object.");
    }

    #[test]
    fn test_fetch() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_fetch");
        let name = path.to_str().unwrap();
        let index = "index";
        let field = "mostSignificantField";
        let i = IndexSpec::new(index, vec![field]);
        let indexes = vec![i];

        let doc_store = DocumentStore::open(name, indexes).expect("should not result in error");

        let doc = json!({
            "foo": "bar",
            "mostSignificantField": "msf1"
        });
        doc_store.insert(&doc.to_string()).expect("empty value");

        let mut filter = HashMap::new();
        filter.insert("mostSignificantField", "msf1");
        let res: String = doc_store
            .fetch(index, filter)
            .expect("Should have fetched without error.") // expect Ok
            .expect("Should have found a document."); // expect Some
        assert_eq!(doc.to_string(), res);
    }

    #[test]
    fn test_fetch_not_found() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_fetch_not_found");
        let name = path.to_str().unwrap();
        let index = "index";
        let field = "mostSignificantField";
        let i = IndexSpec::new(index, vec![field]);
        let indexes = vec![i];

        let doc_store = DocumentStore::open(name, indexes).expect("should not result in error");

        let doc = json!({
            "foo": "bar",
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        doc_store.insert(&doc.to_string()).expect("empty value");

        let mut filter = HashMap::new();
        filter.insert("mostSignificantField", "msf2");
        let res: Option<String> = doc_store
            .fetch(index, filter)
            .expect("Should have fetched without error."); // expect Ok
        assert!(res.is_none());
    }

    #[test]
    fn test_fetch_multiple_indexes() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_fetch_multiple_indexes");
        let name = path.to_str().unwrap();
        let index1 = "index_one";
        let index2 = "index_two";
        let doc_store = DocumentStore::open(
            name,
            vec![
                IndexSpec::new(index1, vec!["index1_field"]),
                IndexSpec::new(index2, vec!["index2_field"]),
            ],
        )
        .expect("should not result in error");

        let fld1 = append_random("msf1_");
        let fld2 = append_random("msf2_");
        let doc = json!({
            "index1_field": fld1,
            "index2_field": fld2
        });
        doc_store
            .insert(&doc.to_string())
            .expect("should not store with missing index fields.");

        let mut filter1 = HashMap::new();
        filter1.insert("index1_field", fld1.as_str());
        let mut filter2 = HashMap::new();
        filter2.insert("index2_field", fld2.as_str());
        let res: String = doc_store
            .fetch(index1, filter1)
            .expect("Should have fetched without error.") // expect Ok
            .expect("Should have found a document."); // expect Some
        assert_eq!(doc.to_string(), res);
        let res: String = doc_store
            .fetch(index2, filter2)
            .expect("Should have fetched without error.") // expect Ok
            .expect("Should have found a document."); // expect Some
        assert_eq!(doc.to_string(), res);
    }

    fn append_random(name: &str) -> String {
        let mut rng = rand::thread_rng();
        format!("{}{}", name, rng.next_u32())
    }
}
