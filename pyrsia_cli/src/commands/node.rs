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

use crate::commands::config::get_config;
use reqwest;

pub async fn ping() -> Result<String, reqwest::Error> {
    let result = get_config();
    let mut url = String::new();
    let _data = match result {
        Ok(data) => {
            url = data;
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    };

    let node_url = format!("http://{}/v2", url);
    let response = reqwest::get(node_url).await?.text().await?;
    Ok(response)
}

pub async fn peers_connected() -> Result<String, reqwest::Error> {
    let result = get_config();
    let mut url = String::new();
    let _data = match result {
        Ok(data) => {
            url = data;
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    };

    let node_url = format!("http://{}/v2/peers", url);

    let response = reqwest::get(node_url).await?.text().await?;

    Ok(response)
}
