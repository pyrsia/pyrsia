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

use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::build_service::model::BuildStatus;
use crate::node_api::model::cli::{
    RequestAddAuthorizedNode, RequestBuildStatus, RequestDockerBuild, RequestDockerLog,
    RequestMavenBuild, RequestMavenLog, Status,
};

use super::config::get_config;

pub async fn ping() -> Result<String> {
    //TODO: implement ping api in Node
    let node_url = format!("http://{}/v2", get_url());
    let response = reqwest::get(node_url).await?.text().await?;
    Ok(response)
}

pub async fn peers_connected() -> Result<String> {
    let node_url = format!("http://{}/peers", get_url());
    let response = reqwest::get(node_url).await?.text().await?;
    Ok(response)
}

pub async fn status() -> Result<Status> {
    let node_url = format!("http://{}/status", get_url());

    let response = reqwest::get(node_url).await?.json::<Status>().await?;
    Ok(response)
}

pub async fn add_authorized_node(request: RequestAddAuthorizedNode) -> Result<()> {
    post_and_parse_result_as_text(format!("http://{}/authorized_node", get_url()), request)
        .await
        .map(|_| ())
}

pub async fn request_docker_build(request: RequestDockerBuild) -> Result<String> {
    post_and_parse_result_as_json(format!("http://{}/build/docker", get_url()), request).await
}

pub async fn request_build_status(request: RequestBuildStatus) -> Result<BuildStatus> {
    post_and_parse_result_as_json(format!("http://{}/build/status", get_url()), request).await
}

pub async fn request_maven_build(request: RequestMavenBuild) -> Result<String> {
    post_and_parse_result_as_json(format!("http://{}/build/maven", get_url()), request).await
}

pub async fn inspect_docker_transparency_log(request: RequestDockerLog) -> Result<String> {
    post_and_parse_result_as_text(format!("http://{}/inspect/docker", get_url()), request).await
}

pub async fn inspect_maven_transparency_log(request: RequestMavenLog) -> Result<String> {
    post_and_parse_result_as_text(format!("http://{}/inspect/maven", get_url()), request).await
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

async fn post_and_parse_result_as_json<R: DeserializeOwned, T: Serialize>(
    node_url: String,
    request: T,
) -> Result<R> {
    let client = reqwest::Client::new();
    client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .json_or_error_with_body()
        .await
}

async fn post_and_parse_result_as_text<T: Serialize>(
    node_url: String,
    request: T,
) -> Result<String> {
    let client = reqwest::Client::new();
    client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .text_or_error_with_body()
        .await
}

#[async_trait]
trait ErrorResponseWithBody {
    async fn json_or_error_with_body<R: DeserializeOwned>(self) -> Result<R>;
    async fn text_or_error_with_body(self) -> Result<String>;
    async fn error_for_status_with_body(self) -> Result<Response>;
}

#[async_trait]
impl ErrorResponseWithBody for Response {
    async fn json_or_error_with_body<R: DeserializeOwned>(self) -> Result<R> {
        match self.error_for_status_with_body().await {
            Ok(r) => Ok(r.json::<R>().await?),
            Err(e) => Err(e),
        }
    }

    async fn text_or_error_with_body(self) -> Result<String> {
        match self.error_for_status_with_body().await {
            Ok(r) => Ok(r.text().await?),
            Err(e) => Err(e),
        }
    }

    async fn error_for_status_with_body(self) -> Result<Response> {
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
        Ok(self)
    }
}
