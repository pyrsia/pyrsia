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

use pyrsia::build_service::model::{BuildInfo, BuildStatus};
use pyrsia::cli_commands::config;
use pyrsia::cli_commands::node;
use pyrsia::node_api::model::cli::{RequestDockerBuild, RequestMavenBuild};
use std::collections::HashSet;
use std::io;
use std::io::BufRead;

pub fn config_add() {
    println!("Enter host: ");
    let mut new_cfg = config::CliConfig {
        host: io::stdin().lock().lines().next().unwrap().unwrap(),
        ..Default::default()
    };

    println!("Enter port: ");
    new_cfg.port = io::stdin().lock().lines().next().unwrap().unwrap();

    println!("Enter disk space to be allocated to pyrsia(Please enter with units ex: 10 GB): ");
    new_cfg.disk_allocated = io::stdin().lock().lines().next().unwrap().unwrap();

    let result = config::add_config(new_cfg);
    match result {
        Ok(_result) => {
            println!("Node configuration Saved !!");
        }
        Err(error) => {
            println!("Error Saving Node Configuration:       {}", error);
        }
    };
}

pub fn config_show() {
    let result = config::get_config();
    match result {
        Ok(config) => {
            println!("{}", config)
        }
        Err(error) => {
            println!("No Node Configured:       {}", error);
        }
    };
}

pub async fn request_docker_build(manifest: &str) {
    let build_result = node::request_docker_build(RequestDockerBuild {
        manifest: manifest.to_owned(),
    })
    .await;
    handle_request_build_result(build_result);
}

pub async fn request_maven_build(gav: &str) {
    let build_result = node::request_maven_build(RequestMavenBuild {
        gav: gav.to_owned(),
    })
    .await;
    handle_request_build_result(build_result);
}

fn handle_request_build_result(build_result: Result<BuildInfo, reqwest::Error>) {
    match build_result {
        Ok(build_info) => match build_info.status {
            BuildStatus::Running => {
                println!(
                    "Build request successfully handled. Build with ID {} has been started.",
                    build_info.id
                );
            }
            BuildStatus::Success { .. } => {
                println!(
                    "Build request successfully handled. Build with ID {} already completed.",
                    build_info.id
                );
            }
            BuildStatus::Failure(msg) => {
                println!("Build request failed with error: {}", msg);
            }
        },
        Err(error) => {
            println!("Error: {}", error);
        }
    }
}

pub async fn node_ping() {
    let result = node::ping().await;
    match result {
        Ok(_resp) => {
            println!("Connection Successful !!")
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    };
}

pub async fn node_status() {
    let result = node::status().await;
    match result {
        Ok(resp) => {
            println!("Connected Peers Count:       {}", resp.peers_count);
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    }
}

pub async fn node_list() {
    let result = node::peers_connected().await;
    match result {
        Ok(resp) => {
            println!("Connected Peers:");
            let peers_split = resp.split(',');
            let mut unique_peers = HashSet::new();
            for peer in peers_split {
                unique_peers.insert(peer);
            }
            unique_peers.iter().for_each(|p| println!("{}", p));
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    }
}
