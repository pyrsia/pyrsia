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

use crate::CONF_FILE_PATH_MSG_STARTER;
use pyrsia::cli_commands::config;
use pyrsia::cli_commands::node;
use pyrsia::node_api::handlers::swarm;
use pyrsia::node_api::model::cli::{
    RequestAddAuthorizedNode, RequestBuildStatus, RequestDockerBuild, RequestDockerLog,
    RequestMavenBuild, RequestMavenLog, TransparencyLogOutputParams,
};
use std::collections::HashSet;
use std::io;
use std::io::BufRead;

const CONF_REMINDER_MESSAGE: &str = "Please make sure the pyrsia CLI config is up to date and matches the node configuration. For more information, run 'pyrsia config --show'";

pub fn config_add() -> anyhow::Result<()> {
    let default_config = config::CliConfig {
        ..Default::default()
    };

    let mut new_cfg = config::CliConfig {
        host: read_interactive_input(
            &format!("Enter host: [{}]", default_config.host),
            &default_config.host,
            &config::valid_host_name,
        ),
        ..Default::default()
    };
    new_cfg.port = read_interactive_input(
        &format!("Enter port: [{}]", default_config.port),
        &default_config.port,
        &config::valid_port,
    );
    new_cfg.disk_allocated = read_interactive_input(
        &format!(
            "Enter disk space to be allocated to pyrsia(Please enter with units ex: 10 GB): [{}]",
            default_config.disk_allocated
        ),
        &default_config.disk_allocated,
        &config::valid_disk_space,
    );

    config::add_config(new_cfg)
}

pub fn config_edit(
    host_name: Option<String>,
    port: Option<String>,
    disk_space: Option<String>,
) -> anyhow::Result<()> {
    config::config_edit(host_name, port, disk_space)
}

pub fn config_remove() -> anyhow::Result<()> {
    config::config_remove()
}

pub fn config_show() {
    match config::get_config_file_path() {
        Ok(path_buf) => {
            println!(
                "{} {}",
                CONF_FILE_PATH_MSG_STARTER,
                path_buf.into_os_string().into_string().unwrap()
            )
        }
        Err(error) => {
            println!("Error retrieving config file path: {}", error);
        }
    }
    let result = config::get_config();
    match result {
        Ok(config) => {
            println!("{}", config)
        }
        Err(error) => {
            println!("No Node Configured: {}", error);
        }
    };
}

pub async fn authorize(peer_id: &str) {
    match node::add_authorized_node(RequestAddAuthorizedNode {
        peer_id: peer_id.to_owned(),
    })
    .await
    {
        Ok(()) => println!("Authorize request successfully handled."),
        Err(error) => println!("Authorize request failed with error: {}", error),
    };
}

pub async fn request_docker_build(image: &str) {
    let build_result = node::request_docker_build(RequestDockerBuild {
        image: image.to_owned(),
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

pub async fn request_build_status(build_id: &str) {
    let result = node::request_build_status(RequestBuildStatus {
        build_id: String::from(build_id),
    })
    .await;

    match result {
        Ok(build_status) => println!("Build status for '{}' is '{}'", build_id, build_status),
        Err(e) => {
            println!(
                "Build status for '{}' was not found: {}
Additional info related to the build might be available via 'pyrsia inspect-log' command",
                build_id, e
            );
        }
    }
}

fn handle_request_build_result(build_result: Result<String, anyhow::Error>) {
    match build_result {
        Ok(build_id) => {
            println!(
                "Build request successfully handled. Build with ID '{}' has been started.",
                build_id
            );
        }
        Err(error_message) => {
            println!("Build request failed with error: {}", error_message);
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
            println!("Error: {}. {}", error, CONF_REMINDER_MESSAGE);
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
            println!("Error: {}. {}", error, CONF_REMINDER_MESSAGE);
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
            println!("Error: {}. {}", error, CONF_REMINDER_MESSAGE);
        }
    }
}

pub async fn inspect_docker_transparency_log(image: &str, format: Option<String>) {
    let content_type = swarm::ContentType::from(format.as_ref()).unwrap();

    let result = node::inspect_docker_transparency_log(RequestDockerLog {
        image: image.to_owned(),
        output_params: Some(TransparencyLogOutputParams {
            format: Some(content_type),
        }),
    })
    .await;
    match result {
        Ok(logs) => {
            content_type.print_logs(logs);
        }
        Err(error) => {
            println!("Inspect log request failed with error: {:?}", error);
        }
    };
}

pub async fn inspect_maven_transparency_log(gav: &str, format: Option<String>) {
    let content_type = swarm::ContentType::from(format.as_ref()).unwrap();

    let result = node::inspect_maven_transparency_log(RequestMavenLog {
        gav: gav.to_owned(),
        output_params: Some(TransparencyLogOutputParams {
            format: Some(content_type),
        }),
    })
    .await;
    match result {
        Ok(logs) => {
            content_type.print_logs(logs);
        }
        Err(error) => {
            println!("Inspect log request failed with error: {:?}", error);
        }
    };
}

/// Read user input interactively until the validation passed
fn read_interactive_input(
    cli_prompt: &str,
    default_val: &str,
    validation_func: &dyn Fn(String) -> Result<String, String>,
) -> String {
    loop {
        println!("{}", cli_prompt);
        let mut buffer = String::new();
        if let Ok(bytes_read) = io::stdin().lock().read_line(&mut buffer) {
            if bytes_read > 0 {
                let mut input = buffer.lines().next().unwrap();
                if input.is_empty() {
                    input = default_val;
                }
                if let Ok(r) = validation_func(input.to_owned()) {
                    break r;
                }
            }
        }
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use crate::cli::handlers::config_show;

    #[test]
    fn test_config_show() {
        config_show();
    }
}
