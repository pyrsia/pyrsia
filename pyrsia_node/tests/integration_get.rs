extern crate pyrsia_node;

use pyrsia_node::network::http::get;
use std::fs::File;
use std::io::{BufRead, BufReader};

// Reads the first line from a BufRead
fn first_line<R>(mut rdr: R) -> String
where
    R: BufRead,
{
    let mut first_line: String = String::new();
    rdr.read_line(&mut first_line).expect("Unable to read line");
    first_line
}

#[test]
fn test_get() {
    let file_name: String = String::from("/tmp/apache_license.txt");
    let uri: String = String::from("https://raw.githubusercontent.com/pyrsia/.github/main/LICENSE");
    let result = get(File::create(file_name.clone()).unwrap(), uri);
    match futures::executor::block_on(result) {
        Err(_) => println!("Caught an error"),
        Ok(_) => println!("Got web page"),
    }

    let f: File = File::open(file_name.clone()).unwrap();
    let first: String = String::from(first_line(BufReader::new(f)));
    let right: String = String::from("Apache License");
    assert_eq!(first.trim(), right);
    println!("\u{2705} test passed.");
    match std::fs::remove_file(file_name.clone()) {
        Err(e) => println!("Caught error removing temp file {}", e),
        Ok(_) => println!("Removed temp file"),
    }
}


#[test]
fn test_bad_site() {
    let file_name: String = String::from("/tmp/err.txt");
    let uri: String = String::from("https://nosuchsite.fake/");
    let result = get(File::create(file_name.clone()).unwrap(), uri);
    match futures::executor::block_on(result) {
        Err(_) => assert!(true, "This request should fail"),
        Ok(_) => assert!(false, "This request should have failed"),
    }
}
