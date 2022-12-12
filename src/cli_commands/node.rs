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

use anyhow::anyhow;
use async_trait::async_trait;
use reqwest::Response;
use serde_json::Value;

use crate::node_api::model::cli::{
    RequestAddAuthorizedNode, RequestDockerBuild, RequestDockerLog, RequestMavenBuild,
    RequestMavenLog, Status,
};

use super::config::get_config;

pub async fn ping() -> Result<String, reqwest::Error> {
    //TODO: implement ping api in Node
    let node_url = format!("http://{}/v2", get_url());
    let response = reqwest::get(node_url).await?.text().await?;
    Ok(response)
}

pub async fn peers_connected() -> Result<String, reqwest::Error> {
    let node_url = format!("http://{}/peers", get_url());
    let response = reqwest::get(node_url).await?.text().await?;
    Ok(response)
}

pub async fn status() -> Result<Status, reqwest::Error> {
    let node_url = format!("http://{}/status", get_url());

    let response = reqwest::get(node_url).await?.json::<Status>().await?;
    Ok(response)
}

pub async fn add_authorized_node(request: RequestAddAuthorizedNode) -> Result<(), reqwest::Error> {
    let node_url = format!("http://{}/authorized_node", get_url());
    let client = reqwest::Client::new();
    client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .error_for_status()
        .map(|_| ())
}

pub async fn request_docker_build(request: RequestDockerBuild) -> Result<String, anyhow::Error> {
    let node_url = format!("http://{}/build/docker", get_url());
    let client = reqwest::Client::new();
    client
        .post(&node_url)
        .json(&request)
        .send()
        .await?
        .error_for_status_with_body()
        .await
}

pub async fn request_maven_build(request: RequestMavenBuild) -> Result<String, anyhow::Error> {
    let node_url = format!("http://{}/build/maven", get_url());
    let client = reqwest::Client::new();
    client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .error_for_status_with_body()
        .await
}

pub async fn inspect_docker_transparency_log(
    request: RequestDockerLog,
) -> Result<String, reqwest::Error> {
    let node_url = format!("http://{}/inspect/docker", get_url());
    let client = reqwest::Client::new();
    match client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .error_for_status()
    {
        Ok(response) => response.text().await,
        Err(e) => Err(e),
    }
}

pub async fn inspect_maven_transparency_log(
    request: RequestMavenLog,
) -> Result<String, reqwest::Error> {
    let node_url = format!("http://{}/inspect/maven", get_url());
    let client = reqwest::Client::new();
    match client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .error_for_status()
    {
        Ok(response) => response.text().await,
        Err(e) => Err(e),
    }
}

pub fn get_url() -> String {
    let result = get_config();
    let mut host = String::new();
    let mut port = String::new();
    match result {
        Ok(data) => {
            host = data.host;
            port = data.port;
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    };

    format!("{}:{}", host, port)
}

#[async_trait]
trait ErrorResponseWithBody {
    async fn error_for_status_with_body(self) -> Result<String, anyhow::Error>;
}

#[async_trait]
impl ErrorResponseWithBody for Response {
    async fn error_for_status_with_body(self) -> Result<String, anyhow::Error> {
        let http_status = self.status();
        let requested_url = self.url().to_string();
        if http_status.is_client_error() || http_status.is_server_error() {
            let parsed_error: Value = serde_json::from_str(self.text().await?.as_str())?;
            return Err(anyhow!(
                "HTTP status error ({}) for url ({}): {}",
                http_status,
                requested_url,
                parsed_error["errors"][0]["message"]
            ));
        }
        Ok(self.text().await?)
    }
}
