// module for the document store

use bincode;
use log::{debug, info};
use rand::Rng;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::fmt;
use std::str;
use unqlite::{Transaction, UnQLite, KV};

#[derive(Debug, Deserialize, Serialize, PartialEq)]
pub enum IndexOrder {
    Asc,
    Desc,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct IndexSpec {
    name: String,
    field_names: Vec<String>,
    direction: IndexOrder,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DocumentStore {
    name: String,
    indexes: Vec<(u16, IndexSpec)>,
}

#[derive(Debug)]
pub enum DocumentStoreError {
    Bincode(bincode::Error),
    Json(serde_json::Error),
    UnQLite(unqlite::Error),
    Custom { message: String },
}

const KEYTYPE_CATALOG: u8 = 0b00000001;
const KEYTYPE_DATA: u8 = 0b00000010;
const KEYTYPE_INDEX: u8 = 0b00000100;

#[derive(Debug, Deserialize, Serialize)]
enum Key {
    Catalog { d: u8 },
    Data { d: u8, n: u128 },
    Index { d: u8, i: u16, v: Vec<String> },
}

impl IndexSpec {
    pub fn new(name: String, field_names: Vec<String>, direction: IndexOrder) -> IndexSpec {
        IndexSpec {
            name,
            field_names,
            direction,
        }
    }
}

impl fmt::Display for DocumentStoreError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match &*self {
            DocumentStoreError::Bincode(ref err) => write!(f, "Bincode Error: {}", err),
            DocumentStoreError::Json(ref err) => write!(f, "Json Error: {}", err),
            DocumentStoreError::UnQLite(ref err) => write!(f, "UnQLite Error: {}", err),
            DocumentStoreError::Custom { message } => write!(f, "DocumentStore Error: {}", message),
        }
    }
}

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

impl Key {
    fn catalog() -> Key {
        Key::Catalog { d: KEYTYPE_CATALOG }
    }

    fn data(number: u128) -> Key {
        Key::Data { d: KEYTYPE_DATA, n: number }
    }

    fn index(index: u16, values: Vec<String>) -> Key {
        Key::Index { d: KEYTYPE_INDEX, i: index, v: values }
    }
}

impl DocumentStore {
    // Creates a new DocumentStore
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

    // Creates an UnQLite database with the specified name and
    // the provided list of fields to be indexed and adds it
    // to the document store
    pub fn create(
        db_name: &str,
        indexes: Vec<IndexSpec>,
    ) -> Result<(), Box<dyn std::error::Error>> {
        info!("Creating DB with name {}", db_name);

        let db = DocumentStore::get_db(db_name);

        let mut rng = rand::thread_rng();

        let mut doc_store_indexes: Vec<(u16, IndexSpec)> = vec![];
        for index in indexes {
            doc_store_indexes.push((rng.gen(), IndexSpec::new(index.name, index.field_names, index.direction)));
        }
        let doc_store = DocumentStore::new(db_name, doc_store_indexes);

        let raw_key = bincode::serialize(&Key::catalog()).map_err(DocumentStoreError::Bincode)?;
        let raw_doc_store = bincode::serialize(&doc_store).map_err(DocumentStoreError::Bincode)?;
        db.kv_store(raw_key, raw_doc_store)
            .map_err(DocumentStoreError::UnQLite)?;

        Ok(())
    }

    pub fn get(db_name: &str) -> Result<DocumentStore, Box<dyn std::error::Error>> {
        let db = DocumentStore::get_db(db_name);

        let raw_key = bincode::serialize(&Key::catalog()).map_err(DocumentStoreError::Bincode)?;
        let raw_doc_store = db
            .kv_fetch(raw_key)
            .map_err(DocumentStoreError::UnQLite)?;
        let doc_store =
            bincode::deserialize(&raw_doc_store).map_err(DocumentStoreError::Bincode)?;

        Ok(doc_store)
    }

    // Store the provided JSON document in the database with
    // the provided name
    pub fn store(
        &self,
        db_name: &str,
        document: &str,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let db = DocumentStore::get_db(db_name);

        let json_document = serde_json::from_str::<Value>(document)?;
        if !json_document.is_object() {
            return Err(From::from(DocumentStoreError::Custom {
                message: "Provided JSON document must represent a JSON Object".to_string(),
            }));
        }

        self.validate_required_indexes(&json_document)?;

        db.begin().map_err(DocumentStoreError::UnQLite)?;

        let mut rng = rand::thread_rng();

        let mut raw_data_key = bincode::serialize(&Key::data(rng.gen())).map_err(DocumentStoreError::Bincode)?;
        loop {
            if !db.kv_contains(&raw_data_key) {
                db.kv_store(&raw_data_key, document)
                    .map_err(DocumentStoreError::UnQLite)?;
                debug!("Document stored!");
                break;
            }
            raw_data_key = bincode::serialize(&Key::data(rng.gen())).map_err(DocumentStoreError::Bincode)?;
        }

        for index in &self.indexes {
            let mut values: Vec<String> = vec![];

            for field_name in &index.1.field_names {
                let value = json_document.get(&field_name).unwrap().as_str().unwrap();
                values.push(value.to_string());
            }

            let index_key: Key = Key::index(index.0, values);
            let raw_index_key = bincode::serialize(&index_key).map_err(DocumentStoreError::Bincode)?;
            db.kv_store(raw_index_key, &raw_data_key)
                .map_err(DocumentStoreError::UnQLite)?;
        }

        Ok(())
    }

    // Fetch a document from the database with the provided
    // name that has an index that maps with the provided
    // indices
    pub fn fetch(
        &mut self,
        db_name: &str,
        indexes: Vec<(&str, &str)>,
    ) -> Result<Vec<String>, Box<dyn std::error::Error>> {
        let db = DocumentStore::get_db(db_name);

        let mut results: Vec<String> = vec![];

        Ok(results)
    }

    fn validate_required_indexes(
        &self,
        json_document: &Value,
    ) -> Result<(), DocumentStoreError> {
        for index in &self.indexes {
            for field_name in &index.1.field_names {
                if let None = json_document.get(field_name) {
                    return Err(DocumentStoreError::Custom {
                        message: format!(
                            "Document is missing required index key: {}.{}",
                            index.1.name, field_name
                        ),
                    });
                }
            }
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dummy() {
        assert_eq!(2 + 2, 4);
    }

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
        let index_one = "index_one".to_string();
        let index_two = "index_two".to_string();
        let field1 = "mostSignificantField";
        let field2 = "leastSignificantField";
        let idx1 = IndexSpec::new(index_one, vec![field1.to_string()], IndexOrder::Asc);
        let idx2 = IndexSpec::new(index_two, vec![field2.to_string()], IndexOrder::Desc);
        let idxs = vec![idx1, idx2];

        DocumentStore::create(name, idxs).expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");
        assert_eq!(doc_store.indexes[0].1.name, "index_one".to_string());
        assert_eq!(doc_store.indexes[0].1.field_names, vec![field1.to_string()]);
        assert_eq!(doc_store.indexes[0].1.direction, IndexOrder::Asc);
        assert_eq!(doc_store.indexes[1].1.name, "index_two".to_string());
        assert_eq!(doc_store.indexes[1].1.field_names, vec![field2.to_string()]);
        assert_eq!(doc_store.indexes[1].1.direction, IndexOrder::Desc);
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
        let index_one = "index_one".to_string();
        let index_two = "index_two".to_string();
        let field1 = "mostSignificantField".to_string();
        let field2 = "leastSignificantField".to_string();
        let i1 = IndexSpec::new(index_one, vec![field1], IndexOrder::Asc);
        let i2 = IndexSpec::new(index_two, vec![field2], IndexOrder::Asc);
        let idxs = vec![i1, i2];

        DocumentStore::create(name, idxs).expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

        let doc = json!({
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        doc_store
            .store(name, &doc.to_string())
            .expect("empty value");
    }

    #[test]
    fn test_store_missing_index_field() {
        let tmp_dir = tempfile::tempdir().unwrap();
        let path = tmp_dir.path().join("test_store_invalid_json");
        let name = path.to_str().unwrap();
        DocumentStore::create(
            name,
            vec![IndexSpec::new(
                "index".to_string(),
                vec!["index_field".to_string()],
                IndexOrder::Asc,
            )],
        )
        .expect("should not result in error");

        let doc_store = DocumentStore::get(name).expect("should not result in error");

        let doc = json!({
            "mostSignificantField": "msf1",
            "leastSignificantField": "12"
        });
        doc_store
            .store(name, &doc.to_string())
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
            .store(name, &String::from("{\"mostSignificantField\":\"value\""))
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
            .store(name, &String::from("[{\"mostSignificantField\":\"value\"}]"))
            .expect_err("should not store non json object.");
    }
}
