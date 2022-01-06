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

extern crate pyrsia;
extern crate string_manipulation;

use pyrsia::network::http::get;
use std::fs::File;
use std::io::BufReader;

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
    let first: String = String::from(string_manipulation::first_line(BufReader::new(f)));
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
