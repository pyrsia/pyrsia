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
//! To store a document, use [`DocumentStore::store`]. Fetching a document
//! via an index can be done with [`DocumentStore::fetch`].
//!

use bincode;
use log::{debug, error, info};
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
#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum IndexOrder {
    /// Fetching a range of values using the index returns
    /// the lowest value first and the highest value last.
    Asc,
    /// Fetching a range of values using the index returns
    /// the highest value first and the lowest value last.
    Desc,
}

/// The definition of an index in the document store.
#[derive(Debug, Deserialize, Serialize)]
pub struct IndexSpec {
    pub name: String,
    pub field_names: Vec<String>,
    direction: IndexOrder,
}

/// The document store is able to store and fetch documents
/// with a list of predefined indexes.
#[derive(Debug, Deserialize, Serialize)]
pub struct DocumentStore {
    pub name: String,
    pub indexes: Vec<(u16, IndexSpec)>,
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
    fn get_index_values(
        &self,
        json_document: &Value,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let mut values: Vec<String> = vec![];

        for field_name in &self.field_names {
            if let Some(value) = json_document.get(&field_name) {
                if let Some(json_string) = value.as_str() {
                    values.push(json_string.to_string());
                } else {
                    return Err(From::from(DocumentStoreError::Custom(format!(
                        "Document has required index key {}.{}, but is not a JSON string: {}.",
                        self.name, field_name, value
                    ))));
                }
            } else {
                return Err(From::from(DocumentStoreError::Custom(format!(
                    "Document is missing required index key: {}.{}",
                    self.name, field_name
                ))));
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
                unqlite::Error::Custom(ref cstm) => Some(cstm),
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
    // Creates a new DocumentStore
    fn new(
        name: &str,
        indexes: Vec<(u16, IndexSpec)>,
    ) -> Result<DocumentStore, DocumentStoreError> {
        if indexes.is_empty() {
            return Err(DocumentStoreError::Custom(
                "At least one index specification is required when creating a DocumentStore."
                    .to_string(),
            ));
        }

        Ok(DocumentStore {
            name: name.to_string(),
            indexes,
        })
    }

    // ping
    pub fn ping(&self) {
        debug!("DocumentStore is alive");
    }

    fn get_data_store(name: &str) -> UnQLite {
        UnQLite::create(format!("{}.unql", name))
    }

    fn open_data_store(name: &str) -> UnQLite {
        UnQLite::open_mmap(format!("{}.unql", &name))
    }

    /// Creates the persistent data store for a DocumentStore and
    /// initializes it with the provided metadata. The metadata
    /// currently consists of the `name` of the DocumentStore and
    /// a vec of `indexes`.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if the metadata could not be stored in the
    /// persistent data store.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pyrsia_node::document_store::document_store::{DocumentStore, IndexSpec};
    /// # let tmp_dir = tempfile::tempdir().unwrap();
    /// # let path = tmp_dir.path().join("docs_code_create");
    /// let name = "document_store";
    /// # let name = path.to_str().unwrap();
    /// let doc_store = DocumentStore::create(name, vec![
    ///     IndexSpec::new("index", vec!["field_one", "field_two"])
    /// ]);
    /// assert!(doc_store.is_ok());
    /// ```
    pub fn create(
        name: &str,
        indexes: Vec<IndexSpec>,
    ) -> Result<DocumentStore, Box<dyn std::error::Error>> {
        info!("Creating DataStore for DocumentStore with name {}", name);

        let raw_key = bincode::serialize(&CatalogKey::new())?;

        let mut doc_store_indexes: Vec<(u16, IndexSpec)> = vec![];
        let mut pos = 1;
        for index in indexes {
            doc_store_indexes.push((pos, IndexSpec::new(index.name, index.field_names)));
            pos += 1;
        }

        let doc_store = DocumentStore::new(name, doc_store_indexes)?;
        let raw_doc_store = bincode::serialize(&doc_store)?;

        let data_store = DocumentStore::get_data_store(name);
        data_store
            .kv_store(raw_key, raw_doc_store)
            .map_err(DocumentStoreError::UnQLite)?;

        Ok(doc_store)
    }

    /// Gets a document store that will use the persistent data store
    /// [created](DocumentStore::create) previously.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if no DocumentStore could be found with the specified `name`.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pyrsia_node::document_store::document_store::DocumentStore;
    /// # let tmp_dir = tempfile::tempdir().unwrap();
    /// # let path = tmp_dir.path().join("docs_code_create");
    /// let name = "document_store";
    /// # let name = path.to_str().unwrap();
    /// if let Ok(doc_store) = DocumentStore::get(name) {
    ///     println!("Found the DocumentStore with name {}", doc_store.name);
    /// }
    /// ```
    pub fn get(name: &str) -> Result<DocumentStore, Box<dyn std::error::Error>> {
        let raw_key = bincode::serialize(&CatalogKey::new())?;

        let data_store = DocumentStore::get_data_store(name);
        let raw_doc_store = data_store
            .kv_fetch(raw_key)
            .map_err(DocumentStoreError::UnQLite)?;
        let doc_store = bincode::deserialize(&raw_doc_store)?;

        Ok(doc_store)
    }

    /// Store the provided JSON document in the DocumentStore. The JSON document
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
    ///
    /// # Examples
    ///
    /// ```
    /// # use pyrsia_node::document_store::document_store::{DocumentStore, IndexSpec};
    /// # use serde_json::json;
    /// # let tmp_dir = tempfile::tempdir().unwrap();
    /// # let path = tmp_dir.path().join("docs_code_store");
    /// let name = "document_store";
    /// # let name = path.to_str().unwrap();
    /// # DocumentStore::create(name, vec![
    /// #    IndexSpec::new("index", vec!["field_one", "field_two"])
    /// # ]);
    /// if let Ok(doc_store) = DocumentStore::get(name) {
    ///     let document = json!({
    ///         "field_one": "foo",
    ///         "field_two": "bar"
    ///     });
    ///     let res = doc_store.insert(&document.to_string());
    ///     assert!(res.is_ok());
    /// }
    /// ```
    pub fn insert(&self, document: &str) -> Result<(), Box<dyn std::error::Error>> {
        let data_store = DocumentStore::get_data_store(&self.name);

        let json_document = serde_json::from_str::<Value>(&document)?;
        if !json_document.is_object() {
            return Err(From::from(DocumentStoreError::Custom(
                "Provided JSON document must represent a JSON Object".to_string(),
            )));
        }

        data_store.begin().map_err(DocumentStoreError::UnQLite)?;

        match self.store_document(&data_store, document) {
            Ok(raw_data_key) => {
                for index in &self.indexes {
                    if let Err(e) = self.process_index(
                        &data_store,
                        &json_document,
                        &raw_data_key,
                        index.0,
                        &index.1,
                    ) {
                        data_store.rollback().map_err(DocumentStoreError::UnQLite)?;
                        return Err(e);
                    }
                }
            }
            Err(e) => {
                data_store.rollback().map_err(DocumentStoreError::UnQLite)?;
                return Err(e);
            }
        }

        data_store.commit().map_err(DocumentStoreError::UnQLite)?;

        Ok(())
    }

    // Store the provided JSON `document` in the provided data store
    // and return the raw data key that was used as the key that
    // uniquely identifies the document.
    fn store_document(
        &self,
        data_store: &UnQLite,
        document: &str,
    ) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
        let mut rng = rand::thread_rng();

        let mut raw_data_key;
        loop {
            raw_data_key = bincode::serialize(&DataKey::new(rng.gen()))?;
            if !data_store.kv_contains(&raw_data_key) {
                data_store
                    .kv_store(&raw_data_key, document)
                    .map_err(DocumentStoreError::UnQLite)?;
                debug!("Document stored!");
                break;
            }
        }

        Ok(raw_data_key)
    }

    // Process the `index` by parsing the index values from the
    // `json_document` and store these index values in the provided data
    // store.
    fn process_index(
        &self,
        data_store: &UnQLite,
        json_document: &Value,
        raw_data_key: &Vec<u8>,
        index: u16,
        index_spec: &IndexSpec,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let index_values = index_spec.get_index_values(json_document)?;
        self.store_index(data_store, raw_data_key, index, index_values)?;

        Ok(())
    }

    // Store the `raw_data_key` (that points to a document) associated
    // with the `index_values` into the provided data store. The key
    // will be an `IndexKey` that is built from the `index` and
    // `index_values`. The value will be the `raw_data_key` itself.
    fn store_index(
        &self,
        data_store: &UnQLite,
        raw_data_key: &Vec<u8>,
        index: u16,
        index_values: Vec<String>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let index_key: IndexKey = IndexKey::new(index, index_values);
        let raw_index_key = bincode::serialize(&index_key)?;
        data_store
            .kv_store(&raw_index_key, raw_data_key)
            .map_err(DocumentStoreError::UnQLite)?;

        Ok(())
    }

    /// Fetches a document from the database by searching on the index
    /// with the specified `index_name` and `filter`. The filter is a
    /// map where the keys represent each field name of the specified
    /// index and the values represent the associated values of the keys
    /// in the document to fetch.
    ///
    /// # Examples
    ///
    /// ```
    /// # use pyrsia_node::document_store::document_store::{DocumentStore, IndexSpec};
    /// # use serde_json::json;
    /// # use std::collections::HashMap;
    /// # let tmp_dir = tempfile::tempdir().unwrap();
    /// # let path = tmp_dir.path().join("docs_code_fetch");
    /// let name = "document_store";
    /// # let name = path.to_str().unwrap();
    /// # DocumentStore::create(name, vec![
    /// #    IndexSpec::new("index", vec!["field_one", "field_two"])
    /// # ]);
    /// if let Ok(doc_store) = DocumentStore::get(name) {
    ///     # let document = json!({
    ///     #     "field_one": "foo",
    ///     #     "field_two": "bar"
    ///     # });
    ///     # doc_store.insert(&document.to_string()).expect("should have stored");
    ///     let mut filter = HashMap::new();
    ///     filter.insert("field_one", "foo");
    ///     filter.insert("field_two", "bar");
    ///     if let Ok(document) = doc_store.fetch("index", filter) {
    ///         assert!(document.is_some());
    ///     }
    /// }
    /// ```
    pub fn fetch(
        &self,
        index_name: &str,
        filter: HashMap<&str, &str>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        if let Some(index_to_use) = self.indexes.iter().find(|index| index.1.name == index_name) {
            let mut values: Vec<String> = vec![];
            for field_name in &index_to_use.1.field_names {
                if let Some(value) = filter.get(&field_name as &str) {
                    values.push(value.to_string());
                } else {
                    return Err(From::from(DocumentStoreError::Custom(format!(
                        "Filter is missing value for field {} required by index {}.",
                        field_name, index_name
                    ))));
                }
            }

            let data_store = DocumentStore::open_data_store(&self.name);
            debug!("Opened db with name {}", &self.name);
            let index_key = IndexKey::new(index_to_use.0, values);
            let raw_index_key = bincode::serialize(&index_key)?;
            if let Ok(raw_data_key) = data_store.kv_fetch(raw_index_key) {
                if let Ok(raw_document) = data_store.kv_fetch(&raw_data_key) {
                    debug!("Found raw document: {:?}", raw_document);
                    let document = String::from_utf8(raw_document)?;
                    return Ok(Some(document));
                } else {
                    let data_key: DataKey = bincode::deserialize(&raw_data_key)?;
                    error!(
                        "DocumentStore failed to find Document with key {} on index {}.",
                        data_key.number, index_name
                    );
                }
            }

            Ok(None)
        } else {
            Err(From::from(DocumentStoreError::Custom(format!(
                "DocumentStore {} has no index with given name: {}",
                self.name, index_name
            ))))
        }
    }
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

        DocumentStore::create(name, vec![]).expect_err("should not have been created");
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
        let idxs = vec![idx1, idx2];

        DocumentStore::create(name, idxs).expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");
        assert_eq!(doc_store.indexes[0].1.name, "index_one".to_string());
        assert_eq!(doc_store.indexes[0].1.field_names, vec![field1.to_string()]);
        assert_eq!(doc_store.indexes[0].1.direction, IndexOrder::Asc);
        assert_eq!(doc_store.indexes[1].1.name, "index_two".to_string());
        assert_eq!(doc_store.indexes[1].1.field_names, vec![field2.to_string()]);
        assert_eq!(doc_store.indexes[1].1.direction, IndexOrder::Asc);
    }

    #[test]
    fn test_get_unknown_name() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_get_unknown_name");
        let name = path.to_str().unwrap();

        DocumentStore::get(name).expect_err("should not have been found");
    }

    #[test]
    fn test_store() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store");
        let name = path.to_str().unwrap();
        let index_one = "index_one";
        let index_two = "index_two";
        let field1 = "mostSignificantField";
        let field2 = "leastSignificantField";
        let i1 = IndexSpec::new(index_one, vec![field1]);
        let i2 = IndexSpec::new(index_two, vec![field2]);
        let idxs = vec![i1, i2];

        DocumentStore::create(name, idxs).expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

        let doc = json!({
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        doc_store.insert(&doc.to_string()).expect("empty value");
    }

    #[test]
    fn test_store_missing_index_field() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store_invalid_json");
        let name = path.to_str().unwrap();
        DocumentStore::create(name, vec![IndexSpec::new("index", vec!["index_field"])])
            .expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

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
        DocumentStore::create(name, vec![IndexSpec::new("index", vec!["field"])])
            .expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

        doc_store
            .insert(&String::from("{\"mostSignificantField\":\"value\""))
            .expect_err("should not store invalid json.");
    }

    #[test]
    fn test_store_non_json_object() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store_non_json_object");
        let name = path.to_str().unwrap();
        DocumentStore::create(name, vec![IndexSpec::new("index", vec!["field"])])
            .expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

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
        let idxs = vec![i];

        DocumentStore::create(name, idxs).expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

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
        let idxs = vec![i];

        DocumentStore::create(name, idxs).expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

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
        DocumentStore::create(
            name,
            vec![
                IndexSpec::new(index1, vec!["index1_field"]),
                IndexSpec::new(index2, vec!["index2_field"]),
            ],
        )
        .expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

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
