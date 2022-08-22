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

use super::config::get_config;
use crate::node_api::model::cli::{
    RequestDockerBuild, RequestDockerLog, RequestMavenBuild, RequestMavenLog, Status,
};

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

pub async fn request_docker_build(request: RequestDockerBuild) -> Result<String, reqwest::Error> {
    let node_url = format!("http://{}/build/docker", get_url());
    let client = reqwest::Client::new();
    match client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .error_for_status()
    {
        Ok(response) => response.json::<String>().await,
        Err(e) => Err(e),
    }
}

pub async fn request_maven_build(request: RequestMavenBuild) -> Result<String, reqwest::Error> {
    let node_url = format!("http://{}/build/maven", get_url());
    let client = reqwest::Client::new();
    match client
        .post(node_url)
        .json(&request)
        .send()
        .await?
        .error_for_status()
    {
        Ok(response) => response.json::<String>().await,
        Err(e) => Err(e),
    }
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
