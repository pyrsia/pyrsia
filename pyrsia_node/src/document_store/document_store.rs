// module for the document store

use log::{debug, info};
use serde_json::{json, Value};
use std::collections::HashMap;
use std::fmt;
use std::str;
use unqlite::{Error, UnQLite, KV};

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

#[derive(Debug, Clone)]
pub struct DocumentStoreError {
    message: String,
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
        write!(f, "DocumentStoreError: {}", self.message)
    }
}

impl Key {
    pub fn new() -> Key {
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

fn map_to_document_store_error(e: Error) -> DocumentStoreError {
    DocumentStoreError {
        message: e.to_string(),
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
    pub fn store(&mut self, db_name: &str, document: &str) -> Result<(), DocumentStoreError> {
        let key = Key::new();

        if let Ok(json_document) = serde_json::from_str::<Value>(document) {
            if let Some(db) = self.dbs.get(db_name) {
                if let Err(e) = db.kv_store(&key, document) {
                    return Err(DocumentStoreError {
                        message: e.to_string(),
                    });
                }
                debug!("Document stored!");
            }

            if let Some(index_db) = self.indices.get(db_name) {
                for index in index_db {
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
                                .map_err(map_to_document_store_error);
                        }
                    }
                }
            } else {
                debug!("DONT Store fields!");
            }
        } else {
            return Err(DocumentStoreError {
                message: String::from("Document contains invalid JSON"),
            });
        }

        Ok(())
    }

    // Fetch a document from the database with the provided
    // name that has an index that maps with the provided
    // JSON filter
    pub fn fetch(
        &mut self,
        db_name: &str,
        filter: &String,
    ) -> Result<Option<String>, DocumentStoreError> {
        let db = self.dbs.get(db_name).unwrap();

        if let Ok(json_filter) = serde_json::from_str::<Value>(filter) {
            if let Some(conditions) = json_filter.as_object() {
                for (key, value) in conditions.iter() {
                    debug!("key = {}, value = {}", key, value);
                    let mut index_name = String::from(db_name);
                    index_name.push('_');
                    index_name.push_str(key);
                    debug!("this belongs to index db {}", index_name);
                    if let Some(field_db) = self.fields.get(&index_name) {
                        if let Ok(key) = field_db.kv_fetch(value.as_str().unwrap()) {
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
            }
        } else {
            return Err(DocumentStoreError {
                message: String::from("Filter contains invalid JSON"),
            });
        }

        Ok(None)
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
        let mut doc_store = DocumentStore::new();
        let name: &str = "test_create_db";
        let idx = vec![];
        doc_store.create_db(name, idx);
    }

    // Create a database with a name and an index list with 2 elements
    #[test]
    fn test_create_db_with_index() {
        let mut doc_store = DocumentStore::new();
        let name: &str = "test_create_db_with_index";
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
        let mut doc_store = DocumentStore::new();
        let name: &str = "test_store";
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
        let mut doc_store = DocumentStore::new();
        let name: &str = "test_fetch";
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
        let flt = json!({
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        let res: String = doc_store
            .fetch(name, &flt.to_string())
            .expect("Should have fetched without error.")
            .expect("Should have been found!");
        info!("Got result: {}", res);
        assert_eq!(doc.to_string(), res);
    }

    #[test]
    fn test_fetch_not_found() {
        let mut doc_store = DocumentStore::new();
        let name: &str = "test_fetch_not_found";
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
        let flt = json!({
            "mostSignificantField": "msf2",
            "leastSignificantField": "12"
        });
        let res = doc_store.fetch(name, &flt.to_string());
        debug!("Got result: {:?}", res);
        assert_eq!(
            true,
            res.expect("Should have fetched without error.").is_none()
        );
    }

    #[test]
    fn test_fetch_invalid_json() {
        let mut doc_store = DocumentStore::new();
        let name: &str = "test_fetch_invalid_json";
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
        let flt = "{\"mostSignificantField\": \"msf2\"";
        doc_store
            .fetch(name, &String::from(flt))
            .expect_err("Should not store invalid json.");
    }
}
