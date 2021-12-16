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

use log::{debug, info};
use std::collections::HashMap;
use std::fmt;
use std::str;
use serde_json::Value;
use unqlite::{UnQLite, KV};

pub struct DocumentStore {
    dbs: HashMap<String, UnQLite>,
    fields: HashMap<String, UnQLite>,
    indices: HashMap<String, Vec<Index>>,
}

pub struct Index {
    name: String,
    _itype: String,
    _order: bool,
}

#[derive(Debug)]
pub enum DocumentStoreError {
    Json(serde_json::Error),
    UnQLite(unqlite::Error),
    Custom { message: String },
}

#[derive(Debug)]
pub struct Key {
    elements: Vec<u8>,
}

impl Index {
    pub fn new(name: String, itype: String, order: bool) -> Index {
        Index {
            name,
            _itype: itype,
            _order: order,
        }
    }
}

impl fmt::Display for DocumentStoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            DocumentStoreError::Json(ref err) => write!(f, "Json Error: {}", err),
            DocumentStoreError::UnQLite(ref err) => write!(f, "UnQLite Error: {}", err),
            DocumentStoreError::Custom { message } => write!(f, "DocumentStore Error: {}", message),
        }
    }
}

impl Key {
    fn new() -> Key {
        let mut v: Vec<u8> = Vec::new();
        for _i in [0..7] {
            let rr = rand::random::<u8>();
            v.push(rr);
        }

        Key { elements: v }
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
        &self.elements
    }
}

impl std::error::Error for DocumentStoreError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match *self {
            DocumentStoreError::Json(ref err) => Some(err),
            DocumentStoreError::UnQLite(ref err) => match *err {
                unqlite::Error::Custom(ref cstm) => Some(cstm),
                unqlite::Error::Other(ref other) => Some(other.as_ref()),
            },
            _ => None,
        }
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

impl DocumentStore {
    // Creates a new DocumentStore
    pub fn new() -> DocumentStore {
        DocumentStore {
            dbs: HashMap::new(),
            fields: HashMap::new(),
            indices: HashMap::new(),
        }
    }

    // ping
    pub fn ping(&self) {
        debug!("DocumentStore is alive");
    }

    // Creates an UnQLite database with the specified name and
    // the provided list of fields to be indexed and adds it
    // to the document store
    pub fn create_db(&mut self, db_name: &str, indices: Vec<Index>) {
        info!("Creating DB with name {}", db_name);

        let db = UnQLite::create(format!("{}.unql", &db_name));
        let mut all_indices = vec![];
        self.dbs.insert(String::from(db_name), db);

        for index in indices {
            let mut field_name = String::from(db_name);
            field_name.push('_');
            field_name.push_str(&index.name);
            debug!("Creating Field DB with name {}", field_name);
            let field_db = UnQLite::create(format!("{}.unql", &field_name));
            self.fields.insert(field_name, field_db);

            all_indices.push(index);
        }
        self.indices.insert(String::from(db_name), all_indices);
    }

    // Store the provided JSON document in the database with
    // the provided name
    pub fn store(
        &mut self,
        db_name: &str,
        document: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let db = self.get_db(db_name)?;

        let json_document = serde_json::from_str::<Value>(document)?;

        let key = Key::new();
        db.kv_store(&key, document)
            .map_err(DocumentStoreError::UnQLite)?;
        debug!("Document stored!");

        let indices = self
            .indices
            .get(db_name)
            .ok_or(DocumentStoreError::Custom {
                message: String::from("No indices found for DocumentStore"),
            })?;
        for index in indices {
            debug!("got an index for {}", index.name);
            let mut index_name = String::from(db_name);
            index_name.push('_');
            index_name.push_str(&index.name);
            debug!("this belongs to db {}", index_name);
            if let Some(field_db) = self.fields.get(&index_name) {
                if let Some(value) = json_document[&index.name].as_str() {
                    debug!("store value {} with key {:?}", value, &key);
                    return field_db
                        .kv_store(value, &key)
                        .map_err(|e| From::from(DocumentStoreError::UnQLite(e)));
                }
            }
        }

        Ok(())
    }

    // Fetch a document from the database with the provided
    // name that has an index that maps with the provided
    // indices
    pub fn fetch(
        &mut self,
        db_name: &str,
        indices: Vec<(&str, &str)>,
    ) -> Result<Option<String>, Box<dyn std::error::Error>> {
        let db = self.get_db(db_name)?;

        for (key, value) in indices.iter() {
            debug!("key = {}, value = {}", key, value);
            let mut index_name = String::from(db_name);
            index_name.push('_');
            index_name.push_str(key);
            debug!("this belongs to index db {}", index_name);
            if let Some(field_db) = self.fields.get(&index_name) {
                if let Ok(key) = field_db.kv_fetch(value) {
                    debug!("Exists! key = {:?}", key);
                    if let Ok(raw_document) = db.kv_fetch(key) {
                        debug!("raw document = {:?}", raw_document);
                        return Ok(Some(String::from_utf8(raw_document).unwrap()));
                    }
                } else {
                    debug!("Does not exist!");
                }
            }
        }

        Ok(None)
    }

    fn get_db(&self, db_name: &str) -> Result<&UnQLite, Box<dyn std::error::Error>> {
        self.dbs.get(db_name).ok_or_else(|| {
            From::from(DocumentStoreError::Custom {
                message: format!("No DocumentStore found for name {}", db_name),
            })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dummy() {
        assert_eq!(2 + 2, 4);
    }

    // Create a database, which requires a name and a list of indices
    // Create a database with a name and an empty index list
    #[test]
    fn test_create_db() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_create_db");
        let name = path.to_str().unwrap();
        let mut doc_store = DocumentStore::new();
        let idx = vec![];
        doc_store.create_db(name, idx);
    }

    // Create a database with a name and an index list with 2 elements
    #[test]
    fn test_create_db_with_index() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_create_db_with_index");
        let name = path.to_str().unwrap();
        let mut doc_store = DocumentStore::new();
        let n1 = "mostSignificantField".to_string();
        let t1 = "string".to_string();
        let n2 = "leastSignificantField".to_string();
        let t2 = "number".to_string();
        let i1 = Index::new(n1, t1, true);
        let i2 = Index::new(n2, t2, false);
        let idx = vec![i1, i2];
        doc_store.create_db(name, idx);
    }

    #[test]
    fn test_store() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store");
        let name = path.to_str().unwrap();
        let mut doc_store = DocumentStore::new();
        let n1 = "mostSignificantField".to_string();
        let t1 = "string".to_string();
        let n2 = "leastSignificantField".to_string();
        let t2 = "number".to_string();
        let i1 = Index::new(n1, t1, true);
        let i2 = Index::new(n2, t2, false);
        let idx = vec![i1, i2];
        doc_store.create_db(name, idx);
        let doc = json!({
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        doc_store
            .store(name, &doc.to_string())
            .expect("empty value");
    }

    #[test]
    fn test_store_invalid_json() {
        let mut doc_store = DocumentStore::new();
        let name: &str = "test_store_invalid_json";
        doc_store.create_db(name, vec![]);
        doc_store
            .store(name, &String::from("{\"mostSignificantField\":\"value\""))
            .expect_err("Should not store invalid json.");
    }

    #[test]
    fn test_fetch() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_fetch");
        let name = path.to_str().unwrap();
        let mut doc_store = DocumentStore::new();
        let n1 = "mostSignificantField".to_string();
        let t1 = "string".to_string();
        let n2 = "leastSignificantField".to_string();
        let t2 = "string".to_string();
        let i1 = Index::new(n1, t1, true);
        let i2 = Index::new(n2, t2, false);
        let idx = vec![i1, i2];
        doc_store.create_db(name, idx);
        let doc = json!({
            "foo": "bar",
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        doc_store
            .store(name, &doc.to_string())
            .expect("empty value");
        let res: String = doc_store
            .fetch(name, vec![("mostSignificantField", "msf1")])
            .expect("Should have fetched without error.")
            .expect("Should have been found!");
        info!("Got result: {}", res);
        assert_eq!(doc.to_string(), res);
    }

    #[test]
    fn test_fetch_not_found() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_not_found");
        let name = path.to_str().unwrap();
        let mut doc_store = DocumentStore::new();
        let n1 = "mostSignificantField".to_string();
        let t1 = "string".to_string();
        let n2 = "leastSignificantField".to_string();
        let t2 = "string".to_string();
        let i1 = Index::new(n1, t1, true);
        let i2 = Index::new(n2, t2, false);
        let idx = vec![i1, i2];
        doc_store.create_db(name, idx);
        let doc = json!({
            "foo": "bar",
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        doc_store
            .store(name, &doc.to_string())
            .expect("empty value");
        let res = doc_store.fetch(name, vec![("mostSignificantField", "msf2")]);
        debug!("Got result: {:?}", res);
        assert_eq!(
            true,
            res.expect("Should have fetched without error.").is_none()
        );
    }
}
