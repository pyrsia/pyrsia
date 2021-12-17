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

use bincode;
use log::{debug, error, info};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;
use std::str;
use unqlite::{Transaction, UnQLite, KV};

// Defines the sorting order when storing the values associated
// with an index.
// Note: This is currently not yet implemented and at the moment,
// all IndexSpec will default to IndexOrder::Asc.
#[derive(Debug, Deserialize, Serialize, PartialEq)]
enum IndexOrder {
    // Fetching a range of values using the index returns
    // the lowest value first and the highest value last.
    Asc,
    // Fetching a range of values using the index returns
    // the highest value first and the lowest value last.
    Desc,
}

/// The definition of an index in the document store.
#[derive(Debug, Deserialize, Serialize)]
pub struct IndexSpec {
    name: String,
    field_names: Vec<String>,
    direction: IndexOrder,
}

/// The document store is able to store and fetch documents
/// with a list of predefined indexes.
#[derive(Debug, Deserialize, Serialize)]
pub struct DocumentStore {
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
    /// Errors mapped to [UnQLite::Error]
    UnQLite(unqlite::Error),
    /// Custom errors specific to the document store
    Custom(String),
}

const KEYTYPE_CATALOG: u8 = 0b00000001;
const KEYTYPE_DATA: u8 = 0b00000010;
const KEYTYPE_INDEX: u8 = 0b00000011;

/// A key that is associated with the metadata of the
/// document store. It is identified by [KEYTYPE_CATALOG].
#[derive(Debug, Deserialize, Serialize)]
struct CatalogKey {
    d: u8,
}

/// A key that is associated with the a document in the
/// document store. It is identified by [KEYTYPE_DATA].
#[derive(Debug, Deserialize, Serialize)]
struct DataKey {
    d: u8,
    number: u128,
}

/// A key that is associated with the a stored index in
/// the document store. It is identified by [KEYTYPE_INDEX].
#[derive(Debug, Deserialize, Serialize)]
struct IndexKey {
    d: u8,
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
        CatalogKey { d: KEYTYPE_CATALOG }
    }
}

impl DataKey {
    fn new(number: u128) -> DataKey {
        DataKey {
            d: KEYTYPE_DATA,
            number,
        }
    }
}

impl IndexKey {
    fn new(index: u16, values: Vec<String>) -> IndexKey {
        IndexKey {
            d: KEYTYPE_INDEX,
            index,
            values,
        }
    }
}

impl DocumentStore {
    /// Creates a new DocumentStore
    fn new(name: &str, indexes: Vec<(u16, IndexSpec)>) -> DocumentStore {
        DocumentStore {
            name: name.to_string(),
            indexes,
        }
    }

    // ping
    pub fn ping(&self) {
        debug!("DocumentStore is alive");
    }

    fn get_db(db_name: &str) -> UnQLite {
        UnQLite::create(format!("{}.unql", &db_name))
    }

    fn open_db(db_name: &str) -> UnQLite {
        UnQLite::open_mmap(format!("{}.unql", &db_name))
    }

    /// Creates an UnQLite database with the specified `db_name` and
    /// the provided list of index definitions. The metadata of the
    /// document store is then added to this database.
    pub fn create(
        db_name: &str,
        indexes: Vec<IndexSpec>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Creating DB with name {}", db_name);

        let db = DocumentStore::get_db(db_name);

        let mut rng = rand::thread_rng();

        let mut doc_store_indexes: Vec<(u16, IndexSpec)> = vec![];
        for index in indexes {
            doc_store_indexes.push((rng.gen(), IndexSpec::new(index.name, index.field_names)));
        }
        let doc_store = DocumentStore::new(db_name, doc_store_indexes);

        let raw_key = bincode::serialize(&CatalogKey::new())?;
        let raw_doc_store = bincode::serialize(&doc_store)?;
        db.kv_store(raw_key, raw_doc_store)
            .map_err(DocumentStoreError::UnQLite)?;

        Ok(())
    }

    /// Gets a document store that was [created](DocumentStore::create) previously.
    ///
    /// # Errors
    ///
    /// Returns [`Err`] if no document store could be found with the specified name.
    pub fn get(db_name: &str) -> Result<DocumentStore, Box<dyn std::error::Error>> {
        let db = DocumentStore::get_db(db_name);

        let raw_key = bincode::serialize(&CatalogKey::new())?;
        let raw_doc_store = db.kv_fetch(raw_key).map_err(DocumentStoreError::UnQLite)?;
        let doc_store = bincode::deserialize(&raw_doc_store)?;

        Ok(doc_store)
    }

    /// Store the provided JSON document in the document store.
    pub fn store(&self, document: &str) -> Result<(), Box<dyn std::error::Error>> {
        let db = DocumentStore::get_db(&self.name);

        let json_document = serde_json::from_str::<Value>(&document)?;
        if !json_document.is_object() {
            return Err(From::from(DocumentStoreError::Custom(
                "Provided JSON document must represent a JSON Object".to_string(),
            )));
        }

        self.validate_required_indexes(&json_document)?;

        db.begin().map_err(DocumentStoreError::UnQLite)?;

        let mut rng = rand::thread_rng();

        let mut raw_data_key = bincode::serialize(&DataKey::new(rng.gen()))?;
        loop {
            if !db.kv_contains(&raw_data_key) {
                db.kv_store(&raw_data_key, document)
                    .map_err(DocumentStoreError::UnQLite)?;
                debug!("Document stored!");
                break;
            }
            raw_data_key = bincode::serialize(&DataKey::new(rng.gen()))?;
        }

        for index in &self.indexes {
            let mut values: Vec<String> = vec![];

            for field_name in &index.1.field_names {
                let value = json_document.get(&field_name).unwrap().as_str().unwrap();
                values.push(value.to_string());
            }

            let index_key: IndexKey = IndexKey::new(index.0, values);
            match bincode::serialize(&index_key) {
                Ok(raw_index_key) => {
                    if let Err(e) = db.kv_store(raw_index_key, &raw_data_key) {
                        db.rollback().map_err(DocumentStoreError::UnQLite)?;
                        return Err(From::from(DocumentStoreError::UnQLite(e)));
                    }
                }
                Err(e) => {
                    db.rollback().map_err(DocumentStoreError::UnQLite)?;
                    return Err(e);
                }
            }
        }

        db.commit().map_err(DocumentStoreError::UnQLite)?;

        Ok(())
    }

    /// Fetches one or more documents from the database by searching
    /// on the index with the specified `index_name` and `filter`. The
    /// filter is a map of which the keys represent each field name
    /// of the specified index. The values represent the associated
    /// values of the keys in the documents to fetch.
    ///
    /// # Examples
    ///
    /// ```
    /// // create a document store
    /// let indexes = vec![
    ///     IndexSpec::new("index", vec!["field"]),
    /// ];
    /// DocumentStore::create("sample", indexes);
    /// let doc_store = DocumentStore::get("sample");
    ///
    /// // store a document
    /// let document = json!({"field": "value"});
    /// doc_store.store(&document.to_string());
    ///
    /// let mut filter = HashMap::new();
    /// filter.insert("field", "value");
    /// if let Ok(documents) = doc_store.fetch("index", filter) {
    ///     println!("Found documents on index 'index': {}", documents);
    /// }
    /// ```
    pub fn fetch(
        &self,
        index_name: &str,
        filter: HashMap<&str, &str>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        if let Some(index_to_use) = self.indexes.iter().find(|index| index.1.name == index_name) {
            let mut results: Vec<String> = vec![];
            let mut values: Vec<String> = vec![];

            for field_name in &index_to_use.1.field_names {
                if let Some(value) = filter.get(&field_name as &str) {
                    values.push(value.to_string());
                } else {
                    return Err(From::from(DocumentStoreError::Custom(format!(
                        "Filter is missing required index key: {}.{}",
                        index_name, field_name
                    ))));
                }
            }

            let db = DocumentStore::open_db(&self.name);
            debug!("Opened db with name {}", &self.name);
            let index_key = IndexKey::new(index_to_use.0, values);
            let raw_index_key = bincode::serialize(&index_key)?;
            if let Ok(raw_data_key) = db.kv_fetch(raw_index_key) {
                if let Ok(raw_data) = db.kv_fetch(&raw_data_key) {
                    debug!("Found raw data: {:?}", raw_data);
                    results.push(String::from_utf8(raw_data).unwrap());
                } else {
                    let data_key: DataKey = bincode::deserialize(&raw_data_key)?;
                    error!(
                        "DocumentStore failed to find Document with key {} on index {}.",
                        data_key.number, index_name
                    );
                }
            }

            return Ok(results);
        } else {
            return Err(From::from(DocumentStoreError::Custom(format!(
                "DocumentStore has no index with given name: {}",
                index_name
            ))));
        }
    }

    fn validate_required_indexes(&self, json_document: &Value) -> Result<(), DocumentStoreError> {
        for index in &self.indexes {
            for field_name in &index.1.field_names {
                if let None = json_document.get(field_name) {
                    return Err(DocumentStoreError::Custom(format!(
                        "Document is missing required index key: {}.{}",
                        index.1.name, field_name
                    )));
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Create a database with a name and an empty index list
    #[test]
    fn test_create() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_create");
        let name = path.to_str().unwrap();

        let result = DocumentStore::create(name, vec![]).expect("should not result in error");
        assert_eq!(result, ());

        let doc_store = DocumentStore::get(name).expect("should not result in error");
        assert_eq!(doc_store.name, name);
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
        doc_store.store(&doc.to_string()).expect("empty value");
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
            .store(&doc.to_string())
            .expect_err("should not store with missing index fields.");
    }

    #[test]
    fn test_store_invalid_json() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store_invalid_json");
        let name = path.to_str().unwrap();
        DocumentStore::create(name, vec![]).expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

        doc_store
            .store(&String::from("{\"mostSignificantField\":\"value\""))
            .expect_err("should not store invalid json.");
    }

    #[test]
    fn test_store_non_json_object() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store_non_json_object");
        let name = path.to_str().unwrap();
        DocumentStore::create(name, vec![]).expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

        doc_store
            .store(&String::from("[{\"mostSignificantField\":\"value\"}]"))
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
        doc_store.store(&doc.to_string()).expect("empty value");

        let mut filter = HashMap::new();
        filter.insert("mostSignificantField", "msf1");
        let res: Vec<String> = doc_store
            .fetch(index, filter)
            .expect("Should have fetched without error.");
        assert_eq!(1, res.len());
        assert_eq!(doc.to_string(), res[0]);
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
        doc_store.store(&doc.to_string()).expect("empty value");

        let mut filter = HashMap::new();
        filter.insert("mostSignificantField", "msf2");
        let res: Vec<String> = doc_store
            .fetch(index, filter)
            .expect("Should have fetched without error.");
        assert_eq!(0, res.len());
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
            .store(&doc.to_string())
            .expect("should not store with missing index fields.");

        let mut filter1 = HashMap::new();
        filter1.insert("index1_field", "msf1");
        let mut filter2 = HashMap::new();
        filter2.insert("index2_field", "msf2");
        let res: Vec<String> = doc_store
            .fetch(index1, filter1)
            .expect("Should have fetched without error.");
        assert_eq!(1, res.len());
        assert_eq!(doc.to_string(), res[0]);
        let res: Vec<String> = doc_store
            .fetch(index2, filter2)
            .expect("Should have fetched without error.");
        assert_eq!(1, res.len());
        assert_eq!(doc.to_string(), res[0]);
    }
}
