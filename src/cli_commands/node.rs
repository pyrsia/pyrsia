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

use crate::cli_commands::model::BuildResultResponse;
use anyhow::{anyhow, Result};
use async_trait::async_trait;
use reqwest::Response;
use serde::de::DeserializeOwned;
use serde::Serialize;
use serde_json::Value;

use crate::node_api::model::request::{
    MavenMapping, RequestAddAuthorizedNode, RequestBuildStatus, RequestDockerBuild,
    RequestDockerLog, RequestMavenBuild, RequestMavenLog, Status,
};

use super::config::get_config;

pub async fn ping() -> Result<String> {
    //TODO: implement ping api in Node
    let response = reqwest::get(get_url("/v2")).await?.text().await?;
    Ok(response)
}

pub async fn peers_connected() -> Result<String> {
    let response = reqwest::get(get_url("/peers")).await?.text().await?;
    Ok(response)
}

pub async fn status() -> Result<Status> {
    let response = reqwest::get(get_url("/status"))
        .await?
        .json::<Status>()
        .await?;
    Ok(response)
}

pub async fn add_authorized_node(request: RequestAddAuthorizedNode) -> Result<()> {
    post_and_parse_result_as_text(get_url("/authorized_node"), request)
        .await
        .map(|_| ())
}

pub async fn request_docker_build(request: RequestDockerBuild) -> Result<BuildResultResponse> {
    post_and_parse_json_result_as_object::<RequestDockerBuild, BuildResultResponse>(
        get_url("/build/docker"),
        request,
    )
    .await
}

pub async fn request_build_status(request: RequestBuildStatus) -> Result<String> {
    post_and_parse_result_as_json(get_url("/build/status"), request).await
}

pub async fn request_maven_build(request: RequestMavenBuild) -> Result<BuildResultResponse> {
    post_and_parse_json_result_as_object::<RequestMavenBuild, BuildResultResponse>(
        get_url("/build/docker"),
        request,
    )
    .await
}

pub async fn inspect_docker_transparency_log(request: RequestDockerLog) -> Result<String> {
    post_and_parse_result_as_text(get_url("/inspect/docker"), request).await
}

pub async fn inspect_maven_transparency_log(request: RequestMavenLog) -> Result<String> {
    post_and_parse_result_as_text(get_url("/inspect/maven"), request).await
}

pub async fn add_maven_mapping(request: MavenMapping) -> Result<String> {
    post_and_parse_result_as_json(get_url("/add-mapping/maven"), request).await
}

pub fn get_url(path: &str) -> String {
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

    format!("http://{}:{}{}", host, port, path)
}

async fn post_and_parse_result_as_json<T: Serialize>(
    node_url: String,
    request: T,
) -> Result<String> {
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

async fn post_and_parse_json_result_as_object<T, R>(node_url: String, request: T) -> Result<R>
where
    T: Serialize,
    R: DeserializeOwned,
{
    let client = reqwest::Client::new();
    client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .object_or_error_with_body::<R>()
        .await
}

#[async_trait]
trait ErrorResponseWithBody {
    async fn json_or_error_with_body(self) -> Result<String>;
    async fn text_or_error_with_body(self) -> Result<String>;
    async fn object_or_error_with_body<R>(self) -> Result<R>
    where
        R: DeserializeOwned;
    async fn error_for_status_with_body(self) -> Result<Response>;
}

#[async_trait]
impl ErrorResponseWithBody for Response {
    async fn json_or_error_with_body(self) -> Result<String> {
        match self.error_for_status_with_body().await {
            Ok(r) => Ok(r.json::<String>().await?),
            Err(e) => Err(e),
        }
    }

    async fn text_or_error_with_body(self) -> Result<String> {
        match self.error_for_status_with_body().await {
            Ok(r) => Ok(r.text().await?),
            Err(e) => Err(e),
        }
    }

    async fn object_or_error_with_body<R>(self) -> Result<R>
    where
        R: DeserializeOwned,
    {
        match self.error_for_status_with_body().await {
            Ok(r) => Ok(r.json::<R>().await?),
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
