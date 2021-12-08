// module for the document store

use log::{debug, info};
use std::collections::HashMap;
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

fn split_key_value(source: &str) -> (&str, &str) {
    let keyvalue: Vec<&str> = source.split(':').collect();
    let key = keyvalue[0].get(1..keyvalue[0].len() - 1).unwrap();
    let value = keyvalue[1].get(1..keyvalue[1].len() - 1).unwrap();
    (key, value)
}

fn get_value_for_key(source: &str, searchkey: &str) -> String {
    let entries: Vec<&str> = source
        .get(1..source.len() - 1)
        .unwrap()
        .split(',')
        .collect();
    for entry in entries {
        let (key, value) = split_key_value(entry);
        debug!("GVFK, key = {} and searchKey = {}", key, searchkey);
        if key.eq(searchkey) {
            debug!("GVFK, key = {} and searchKey = {}", key, searchkey);
            return value.to_string();
        }
    }
    "ok".to_string()
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

    // Store the provided document in the database with
    // the provided name
    pub fn store(&mut self, db_name: &str, document: &str) -> Result<(), Error> {
        let key = Key::new();

        if let Some(db) = self.dbs.get(db_name) {
            if let Err(e) = db.kv_store(&key, document) {
                return Err(e);
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
                    let value = get_value_for_key(document, &index.name);
                    debug!("store value {} with key {:?}", value, &key);
                    return field_db.kv_store(value, &key);
                }
            }
        } else {
            debug!("DONT Store fields!");
        }

        Ok(())
    }

    // Fetch a document from the database with the provided
    // name that has an index that maps with the provided
    // filter
    pub fn fetch(&mut self, db_name: &str, filter: &str) -> Option<String> {
        let db = self.dbs.get(db_name).unwrap();

        let filter_length = filter.len();
        let json = filter.get(1..filter_length - 1).unwrap();

        // split filter in a number of conditions
        let conditions: Vec<&str> = json.split(',').collect();
        for condition in &conditions {
            debug!("condition = {}", condition);
            let (key, value) = split_key_value(condition);
            debug!("key = {}, value = {}", key, value);
            let mut index_name = String::from(db_name);
            index_name.push('_');
            index_name.push_str(key);
            debug!("this belongs to index db {}", index_name);
            if let Some(field_db) = self.fields.get(&index_name) {
                if let Ok(key) = field_db.kv_fetch(value.to_string()) {
                    debug!("Exists! key = {:?}", key);
                    if let Ok(raw_document) = db.kv_fetch(key) {
                        debug!("raw document = {:?}", raw_document);
                        return Some(String::from_utf8(raw_document).unwrap());
                    }
                } else {
                    debug!("Does not exist!");
                }
            }
        }
        None
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
        let doc: &str = "{\"mostSignificantField\":\"msf1\",\"leastSignificantField\":\"12\"}";
        doc_store.store(name, doc).expect("empty value");
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
        let doc: &str =
            "{\"foo\":\"bar\",\"mostSignificantField\":\"msf1\",\"leastSignificantField\":\"12\"}";
        doc_store.store(name, doc).expect("empty value");
        let flt: &str = "{\"mostSignificantField\":\"msf1\",\"leastSignificantField\":\"12\"}";
        let res: String = doc_store.fetch(name, flt).expect("Should have been found!");
        info!("Got result: {}", res);
        assert_eq!(doc, res);
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
        let doc: &str =
            "{\"foo\":\"bar\",\"mostSignificantField\":\"msf1\",\"leastSignificantField\":\"12\"}";
        doc_store.store(name, doc).expect("empty value");
        let flt: &str = "{\"mostSignificantField\":\"msf2\",\"leastSignificantField\":\"12\"}";
        let res = doc_store.fetch(name, flt);
        debug!("Got result: {:?}", res);
        assert_eq!(true, res.is_none());
    }
}
