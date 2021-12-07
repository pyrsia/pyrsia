extern crate anyhow;
extern crate reqwest;
extern crate tokio;

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
    // See pyrsia_node integration tests
}
