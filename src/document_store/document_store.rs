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

use anyhow::{anyhow, bail, Context};
use bincode;
use log::{debug, error, info, warn};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::fmt;
use std::str;
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
    pub fn open(name: &str, indexes: Vec<IndexSpec>) -> anyhow::Result<DocumentStore> {
        info!("Opening DocumentStore with name {}", name);
        check_index_specs_valid(&indexes)?;
        let document_store = create_document_store(name, &indexes);
        match get_catalog_record(&document_store) {
            Ok(_) => info!(
                "Opened existing document store collection: {}",
                document_store.catalog.name
            ),
            Err(error) => {
                info!("Failed to get catalog record. Assuming that collection {} is new. Creating catalog record. Error was {}", document_store.catalog.name, error);
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

fn create_document_store(name: &str, indexes: &Vec<IndexSpec>) -> DocumentStore {
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
            indexes: (0..index_count_u16)
                .zip(indexes.iter().map(|ix| ix.clone()))
                .collect(),
        },
        unqlite: UnQLite::create(collection_name_to_file_name(name)),
    }
}

fn check_index_specs_valid(indexes: &Vec<IndexSpec>) -> anyhow::Result<()> {
    if indexes.is_empty() {
        bail!("At least one index specification is required when creating a DocumentStore.")
    } else {
        Ok(())
    }
}

fn collection_name_to_file_name(name: &str) -> String {
    let mut s = name.to_string();
    s.push_str(".db");
    s
}

fn get_catalog_record(document_store: &DocumentStore) -> anyhow::Result<Catalog> {
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

fn serialized_catalog_key() -> anyhow::Result<Vec<u8>> {
    bincode::serialize(&CatalogKey::new())
        .context("Failed to create the key for the catalog record")
}

// A description of a collection of documents and how they are indexed.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
struct Catalog {
    name: String,
    indexes: Vec<(u16, IndexSpec)>,
}

/// The DocumentStoreError acts as a wrapper around all types of
/// errors that can occur while working with a document store.
#[derive(Debug)]
pub enum DocumentStoreError {
    /// Errors mapped to [bincode::Error]
    Bincode(bincode::Error),
    /// Errors mapped to [serde_json::Error]
    Json(serde_json::Error),
    /// Errors mapped to [unqlite::Error]
    UnQLite(unqlite::Error),
    /// Custom errors specific to the document store
    Custom(String),
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
// the document store. It is identified by [KEYTYPE_INDEX].
#[derive(Debug, Deserialize, Serialize)]
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
    fn get_index_values(&self, json_document: &Value) -> anyhow::Result<Vec<String>> {
        let mut values: Vec<String> = vec![];

        for field_name in &self.field_names {
            if let Some(value) = json_document.get(&field_name) {
                if let Some(json_string) = value.as_str() {
                    values.push(json_string.to_string());
                } else {
                    return bail!(
                        "Document has required index key {}.{}, but is not a JSON string: {}.",
                        self.name, field_name, value
                    );
                }
            } else {
                return bail!(
                    "Document is missing required index key: {}.{}",
                    self.name,
                    field_name
                );
            }
        }

        Ok(values)
    }
}

impl fmt::Display for DocumentStoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            DocumentStoreError::Bincode(ref err) => write!(f, "Bincode Error: {}", err),
            DocumentStoreError::Json(ref err) => write!(f, "Json Error: {}", err),
            DocumentStoreError::UnQLite(ref err) => write!(f, "UnQLite Error: {}", err),
            DocumentStoreError::Custom(ref message) => {
                write!(f, "DocumentStore Error: {}", message)
            }
        }
    }
}

/// Implementation of the [std::error::Error] trait for the
/// DocumentStoreError.
impl std::error::Error for DocumentStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            DocumentStoreError::Bincode(ref err) => Some(err),
            DocumentStoreError::Json(ref err) => Some(err),
            DocumentStoreError::UnQLite(ref err) => match *err {
                unqlite::Error::Custom(ref custom) => Some(custom),
                unqlite::Error::Other(ref other) => Some(other.as_ref()),
            },
            _ => None,
        }
    }
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
    pub fn insert(&self, document: &str) -> anyhow::Result<()> {
        let json_document = serde_json::from_str::<Value>(&document)?;
        if !json_document.is_object() {
            return Err(From::from(DocumentStoreError::Custom(
                "Provided JSON document must represent a JSON Object".to_string(),
            )));
        }
        self.unqlite.begin().map_err(DocumentStoreError::UnQLite)?;

        match store_document(self, document) {
            Ok(raw_data_key) => {
                for index in &self.catalog.indexes {
                    if let Err(e) = process_index(
                        &self.unqlite,
                        &json_document,
                        &raw_data_key,
                        index.0,
                        &index.1,
                    ) {
                        self.unqlite
                            .rollback()
                            .map_err(DocumentStoreError::UnQLite)?;
                        return Err(anyhow!("{}", e.to_string()));
                    }
                }
            }
            Err(e) => {
                self.unqlite
                    .rollback()
                    .map_err(DocumentStoreError::UnQLite)?;
                return Err(anyhow!("{}", e.to_string()));
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
    ) -> anyhow::Result<Option<String>> {
        let index_to_use = find_index(self, index_name)?;
        let compound_key = build_compound_key(index_name, filter, index_to_use.1)?;

        let index_key = IndexKey::new(index_to_use.0, compound_key);
        fetch_indexed_record(index_name, &self.unqlite, &index_key)
    }
}

fn find_index<'a>(ds: &'a DocumentStore, index_name: &str) -> anyhow::Result<(u16, &'a IndexSpec)> {
    match ds
        .catalog
        .indexes
        .iter()
        .find(|index| index.1.name == index_name)
    {
        Some((position, index_to_use)) => Ok((*position, index_to_use)),
        None => Err(From::from(DocumentStoreError::Custom(format!(
            "DocumentStore {} has no index with given name: {}",
            ds.catalog.name, index_name
        )))),
    }
}

// Store the provided JSON `document` in the provided data store
// and return the raw data key that was used as the key that
// uniquely identifies the document.
fn store_document(ds: &DocumentStore, document: &str) -> anyhow::Result<Vec<u8>> {
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
) -> anyhow::Result<Vec<String>> {
    let mut compound_key: Vec<String> = vec![];
    for field_name in &index_to_use.field_names {
        if let Some(value) = filter.get(&field_name as &str) {
            compound_key.push(value.to_string());
        } else {
            return Err(From::from(DocumentStoreError::Custom(format!(
                "Filter is missing value for field {} required by index {}.",
                field_name, index_name
            ))));
        }
    }
    Ok(compound_key)
}

fn fetch_indexed_record(
    index_name: &str,
    unqlite: &UnQLite,
    index_key: &IndexKey,
) -> anyhow::Result<Option<String>> {
    let raw_index_key = bincode::serialize(&index_key)?;
    if let Ok(raw_data_key) = unqlite.kv_fetch(&raw_index_key) {
        if let Ok(raw_document) = unqlite.kv_fetch(&raw_data_key) {
            debug!("Found raw document: {:?}", raw_document);
            let document = String::from_utf8(raw_document)?;
            Ok(Some(document))
        } else {
            let data_key: DataKey = bincode::deserialize(&raw_data_key)?;
            let message = format!("DocumentStore found an index entry for {:?} in index {} using raw index key {:?} pointing to record {:?}, but failed to find Document with key {}.",
                                  index_key, index_name, raw_index_key, raw_data_key, data_key.number);
            error!("{}", message);
            Err(anyhow!("{}", message))
        }
    } else {
        Ok(None)
    }
}

// Process the `index` by parsing the index values from the
// `json_document` and store these index values in the provided data
// store.
fn process_index(
    data_store: &UnQLite,
    json_document: &Value,
    raw_data_key: &Vec<u8>,
    index: u16,
    index_spec: &IndexSpec,
) -> anyhow::Result<()> {
    let index_values = index_spec.get_index_values(json_document)?;
    store_index(data_store, raw_data_key, index, index_values)?;
    Ok(())
}

// Store the `raw_data_key` (that points to a document) associated
// with the `index_values` into the provided data store. The key
// will be an `IndexKey` that is built from the `index` and
// `index_values`. The value will be the `raw_data_key` itself.
fn store_index(
    unqlite: &UnQLite,
    raw_data_key: &Vec<u8>,
    index: u16,
    index_values: Vec<String>,
) -> anyhow::Result<()> {
    let index_key: IndexKey = IndexKey::new(index, index_values);
    let raw_index_key = bincode::serialize(&index_key).context("Failed to serialize index key")?;
    if unqlite.kv_fetch_length(&raw_index_key).is_ok() {
        return bail!("Attempted to insert duplicate record in index {}", index);
    }
    unqlite
        .kv_store(&raw_index_key, raw_data_key)
        .map_err(DocumentStoreError::UnQLite)
        .with_context(|| format!("Failed to store index record for index {}", index))?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
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
        let path = tmp_dir.path().join("test_store");
        let name = path.to_str().unwrap();
        let index_one = "index_one";
        let index_two = "index_two";
        let field1 = "mostSignificantField";
        let field2 = "leastSignificantField";
        let i1 = IndexSpec::new(index_one, vec![field1]);
        let i2 = IndexSpec::new(index_two, vec![field2]);
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
        doc_store.insert(&doc1.to_string())?;
        if doc_store.insert(&doc2.to_string()).is_ok() {
            bail!("Attempt to add a duplicate record succeeded.")
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

        let doc = json!({
            "index1_field": "msf1",
            "index2_field": "msf2"
        });
        doc_store
            .insert(&doc.to_string())
            .expect("should not store with missing index fields.");

        let mut filter1 = HashMap::new();
        filter1.insert("index1_field", "msf1");
        let mut filter2 = HashMap::new();
        filter2.insert("index2_field", "msf2");
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
}
