extern crate anyhow;
extern crate reqwest;
extern crate tokio;
#[cfg(test)]
extern crate utilities;

use std::io;
use std::io::prelude::*;

// performs an HTTP GET of `url` and writes the body to `out`
pub async fn get<W>(mut out: W, url: String) -> anyhow::Result<u64>
where
    W: Write,
{
    use reqwest::blocking::{Client, Response};

    let client: Client = Client::new();
    let resp: Result<Response, reqwest::Error> = client.get(url.clone()).send();
    match resp {
        Ok(_) => Ok(io::copy(&mut resp.unwrap(), &mut out)?),
        Err(error) => Err(anyhow::anyhow!("Caught error {} on url {}", error, url)),
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;
    use std::fs::File;
    use std::io::BufReader;

    #[test]
    fn test_get() {
        let file_name: String = String::from("/tmp/apache_license.txt");
        let uri: String =
            String::from("https://raw.githubusercontent.com/pyrsia/.github/main/LICENSE");
        let result = get(File::create(file_name.clone()).unwrap(), uri);
        match futures::executor::block_on(result) {
            Err(_) => println!("Caught an error"),
            Ok(_) => println!("Got web page"),
        }

        let f: File = File::open(file_name.clone()).unwrap();
        let first: String = String::from(utilities::first_line(BufReader::new(f)));
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
}
