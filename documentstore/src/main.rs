mod unqlitedb;

use unqlitedb::UnQLiteDB;
use unqlitedb::Index;

fn main() {
    println!("Hello, world!");

    let mut unqlitedb = UnQLiteDB::new();

    let name: &str = "rootdb";

    let a1 = "key1".to_string();
    let b1 = "string".to_string();
    let i1 = Index::new(a1, b1, true);
    let idx = vec![i1];

    unqlitedb.create_db(name, idx);
    unqlitedb.store(name, "{\"key1\":\"val1\"}");
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn dummy() {
        assert_eq!(2+2, 4);
    }

// Create a database, which requires a name and a list of indices
// Create a database with a name and an empty index list
    #[test]
    fn test_create_db() {
        let mut unqlitedb = UnQLiteDB::new();
        let name: &str = "rootdb";
        let idx = vec![];
        unqlitedb.create_db(name, idx);
    }

// Create a database with a name and an index list with 2 elements
    #[test]
    fn test_create_db_with_index() {
        let mut unqlitedb = UnQLiteDB::new();
        let name: &str = "rootdb";
        let n1 = "mostSignificatntField".to_string();
        let t1 = "string".to_string();
        let n2 = "leastSignificantField".to_string();
        let t2 = "number".to_string();
        let i1 = Index::new(n1, t1, true);
        let i2 = Index::new(n2, t2, false);
        let idx = vec![i1, i2];
        unqlitedb.create_db(name, idx);
    }

    // #[test]
    fn test_store() {
        let mut unqlitedb = UnQLiteDB::new();
        let name: &str = "rootdb";
        let n1 = "mostSignificantField".to_string();
        let t1 = "string".to_string();
        let n2 = "leastSignificantField".to_string();
        let t2 = "number".to_string();
        let i1 = Index::new(n1, t1, true);
        let i2 = Index::new(n2, t2, false);
        let idx = vec![i1, i2];
        unqlitedb.create_db(name, idx);
        let doc: &str = "{'mostSignificantField':'msf1','leastSignificantField':'12'}";
        unqlitedb.store(name, doc);
    }

    #[test]
    fn test_fetch() {
        let mut unqlitedb = UnQLiteDB::new();
        let name: &str = "rootdb";
        let n1 = "mostSignificantField".to_string();
        let t1 = "string".to_string();
        let n2 = "leastSignificantField".to_string();
        let t2 = "string".to_string();
        let i1 = Index::new(n1, t1, true);
        let i2 = Index::new(n2, t2, false);
        let idx = vec![i1, i2];
        unqlitedb.create_db(name, idx);
        let doc: &str = "{'foo':'bar','mostSignificantField':'msf1','leastSignificantField':'12'}";
        unqlitedb.store(name, doc);
        let flt: &str = "{'mostSignificantField':'msf1','leastSignificantField':'12'}";
        let res: String = unqlitedb.fetch(name, flt);
        println!("Got result: {}",res);
    }

}
