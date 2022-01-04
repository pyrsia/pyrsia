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
