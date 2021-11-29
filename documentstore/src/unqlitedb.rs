extern crate unqlite;

// use unqlite::{UnQLite, Config, KV, Cursor};
use unqlite::{UnQLite, KV};
use std::collections::HashMap;
use std::str;


pub struct UnQLiteDB {
    database_map: HashMap<String, UnQLite>,
    field_database_map: HashMap<String, UnQLite>,
    database_index: HashMap<String, Vec<Index>>,
}

pub struct Index {
    name: String,
    itype: String,
    order:bool,
}

pub struct Key {
    // low: u64,
    // high: u64,
    elements: Vec<u8>,
}

impl Index {
    pub fn new(name: String, itype: String, order: bool) -> Index {
        Index {
            name: name,
            itype: itype,
            order: order,
        }
    }
}

impl Key {
    pub fn new() -> Key {
        // let x = rand::random::<u64>();
        // let y = rand::random::<u64>();
        let mut v : Vec<u8> = Vec::new();
        for i in [0..7] {
           let rr = rand::random::<u8>();
           v.push(rr);
        }

        Key {
            // low: x,
            // high: y,
            elements: v
        }
    }
}

impl AsRef<[u8]> for Key {
    fn as_ref(&self) -> &[u8] {
		&self.elements
    }
}

pub fn split_key_value(source:&str) -> (&str, &str) {
    let keyvalue: Vec<&str> = source.split(':').collect();
    let key = keyvalue[0].get(1..keyvalue[0].len()-1).unwrap();
    let value = keyvalue[1].get(1..keyvalue[1].len()-1).unwrap();
    (key, value)
}

pub fn get_value_for_key(source:&str, searchkey:&str) -> String {
    let entries: Vec<&str> = source.get(1..source.len()-1).unwrap().split(',').collect();
    for entry in entries {
        let (key, value) = split_key_value(entry);
        println!("GVFK, key = {} and searchKey = {}", key, searchkey);
        if (key.eq(searchkey)) {
            println!("GVFK, key = {} and searchKey = {}", key, searchkey);
            return value.to_string();
        }
    }
    return "ok".to_string();
}

impl UnQLiteDB {
    pub fn new() -> UnQLiteDB {
        println!("Creating UnQLiteDB!");
        UnQLiteDB {
            database_map: HashMap::new(),
            field_database_map: HashMap::new(),
            database_index: HashMap::new()
        }
    }

    pub fn create_db(&mut self, name: &str, idxs: Vec<Index>) {
        println!("Creating DB with name {}", name);

        let unqlite = UnQLite::create(&name);
        let mut index = vec![];
        self.database_map.insert(String::from(name), unqlite);

        for idx in idxs {
            let mut fname = String::from(name);
            fname.push_str("_");
            fname.push_str(&idx.name);
            println!("Creating Index with name {}", fname);
            let fielddb = UnQLite::create(&fname);
            index.push(idx);
            self.field_database_map.insert(fname, fielddb);
        }
        self.database_index.insert(String::from(name), index);
    }

    pub fn store(&mut self, name: &str, data: &str) {
        let x = Key::new();
        if let Some(_db) = self.database_map.get(name) {
            let res =_db.kv_store(&x, data);
            println!("Stored entry!");
        } else {
            println!("DONT Store entry!");
        }
        if let Some(_fdbi) = self.database_index.get(name) {
            println!("Store fields!");
            for ix in _fdbi {
                println!("got an index for {}", ix.name);
                let mut idxname = String::from(name);
                idxname.push_str("_");
                idxname.push_str(&ix.name);
                println!("this belongs to db {}", idxname);
                if let Some(_fdb) = self.field_database_map.get(&idxname) {
                    let value = get_value_for_key(data, &ix.name);
println!("store value {}", value);
                    let res = _fdb.kv_store(value.to_string(), &x).unwrap();
                    // let res = _fdb.kv_store(value, &x).unwrap();
                }
            }
        } else {
            println!("DONT Store fields!");
        }
    }


    pub fn fetch(&mut self, name: &str, filter: &str) -> String {
        let _db = self.database_map.get(name).unwrap();
        let flen = filter.len();
        let json = filter.get(1..flen-1).unwrap();
        // split filter in a number of conditions
        let conditions: Vec<&str> = json.split(',').collect();
        for subcond in &conditions {
            println!("sc = {}", subcond);
            let (key, value) = split_key_value(subcond);
            println!("key = {}, value = {}", key, value);
            let mut idxname = String::from(name);
            idxname.push_str("_");
            idxname.push_str(&key);
            println!("this belongs to db {}", idxname);
            if let Some(_fdb) = self.field_database_map.get(&idxname) {
                let res = _fdb.kv_fetch(value.to_string());
                // let res = _fdb.kv_fetch(value);
                let exists = res.ok();
                if (exists.is_some()) {
                    println!("Exists!");
                    let key = exists.unwrap();
                    println!("key = {:?}", key);
                    let answer_bytes =_db.kv_fetch(key).unwrap();
                    println!("answer = {:?}", answer_bytes);
                    let answer = String::from_utf8(answer_bytes).unwrap();
                    return answer;
                } else {
                    println!("Doesn't exists!");
                }
            }
        }
        return "notfound".to_string();
    }
}
