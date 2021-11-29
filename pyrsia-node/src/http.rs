extern crate reqwest;
extern crate tokio;

use reqwest::blocking;
use std::io;
use std::io::prelude::*;
use std::io::BufRead;

// performs an HTTP GET of `url` and writes the body to `out`
pub async fn get<W>(mut out: W, url: String) -> Result<u64, std::io::Error>
where
    W: Write,
{
    use reqwest::blocking::Client;
    use reqwest::blocking::Response;
    use std::io::ErrorKind::Other;

    let client = Client::new();
    let resp: Result<Response, reqwest::Error> = client.get(url.clone()).send();
    match resp {
        Ok(_) => io::copy(&mut resp.unwrap(), &mut out),
        Err(error) => Err(std::io::Error::new(
            Other,
            format!("Http Client Failed to Get {} with {}", url, error),
        )),
    }
}

#[cfg(test)]
mod tests {
    // Note this useful idiom: importing names from outer (for mod tests) scope.
    use super::*;

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
        use std::fs::File;
        use std::io::BufReader;

        let file_name: String = String::from("/tmp/great_expectations.txt");
        let uri: String = String::from("https://www.gutenberg.org/files/1400/1400-0.txt");
        let result = get(File::create(file_name.clone()).unwrap(), uri);
        match futures::executor::block_on(result) {
            Err(_) => println!("Caught an error"),
            Ok(_) => println!("Got web page"),
        }

        let f = File::open(file_name.clone()).unwrap();
        let first: String = String::from(
            first_line(BufReader::new(f))
                .strip_prefix("\u{feff}")
                .unwrap(),
        );
        let right = "The Project Gutenberg eBook of Great Expectations, by Charles Dickens";
        assert_eq!(first.trim(), right);
        println!("\u{2705} test passed.");
        match std::fs::remove_file(file_name.clone()) {
            Err(e) => println!("Caught error removing temp file {}", e),
            Ok(_) => println!("Removed temp file"),
        }
    }

    #[test]
    fn test_bad_site() {
        use std::fs::File;

        // let uri: String = String::from("https://nosuchsite.fake/");
        let file_name: String = String::from("/tmp/err.txt");
        let uri: String = String::from("https://nosuchsite.fake/");
        let result = get(File::create(file_name.clone()).unwrap(), uri);
        match futures::executor::block_on(result) {
            Err(_) => assert!(true, "This request should fail"),
            Ok(_) => assert!(false, "This request should have failed"),
        }
    }
}
