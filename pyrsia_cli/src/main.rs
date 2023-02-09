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

pub mod cli;

use cli::handlers::*;
use cli::parser::*;

const CONF_FILE_PATH_MSG_STARTER: &str = "Config file path:";

#[tokio::main]
async fn main() {
    // parsing command line arguments
    let matches = cli_parser();

    // checking and preparing responses for each command and its arguments if applicable

    match matches.subcommand() {
        Some(("config", config_matches)) => {
            if let Some(("edit", edit_config_matches)) = config_matches.subcommand() {
                if vec!["host", "port", "diskspace"]
                    .into_iter()
                    .any(|opt_str| edit_config_matches.contains_id(opt_str))
                {
                    let host_name = edit_config_matches.get_one::<String>("host");
                    let port = edit_config_matches.get_one::<String>("port");
                    let diskspace = edit_config_matches.get_one::<String>("diskspace");
                    match config_edit(host_name.cloned(), port.cloned(), diskspace.cloned()) {
                        Ok(_) => {
                            println!("Node configuration saved !!");
                        }
                        Err(error) => {
                            eprintln!("Error saving node configuration: {}", error);
                        }
                    }
                } else {
                    match config_add() {
                        Ok(_) => {
                            println!("Node configuration saved !!");
                        }
                        Err(error) => {
                            eprintln!("Error saving node configuration: {}", error);
                        }
                    }
                }
            }
            if *config_matches.get_one::<bool>("remove").unwrap_or(&false) {
                match config_remove() {
                    Ok(_) => {
                        println!("Node configuration removed !!");
                    }
                    Err(error) => {
                        eprintln!("Error removing node configuration: {}", error);
                    }
                }
            }
            if *config_matches.get_one::<bool>("show").unwrap_or(&false) {
                config_show();
            }
        }
        Some(("authorize", authorize_matches)) => {
            authorize(authorize_matches.get_one::<String>("peer").unwrap()).await;
        }
        Some(("build", build_matches)) => match build_matches.subcommand() {
            Some(("docker", docker_matches)) => {
                request_docker_build(docker_matches.get_one::<String>("image").unwrap()).await;
            }
            Some(("maven", maven_matches)) => {
                request_maven_build(maven_matches.get_one::<String>("gav").unwrap()).await;
            }
            Some(("status", status_matches)) => {
                request_build_status(status_matches.get_one::<String>("id").unwrap()).await;
            }
            _ => {}
        },
        Some(("list", _config_matches)) => {
            node_list().await;
        }
        Some(("ping", _config_matches)) => {
            node_ping().await;
        }
        Some(("status", _config_matches)) => {
            node_status().await;
        }
        Some(("inspect-log", build_matches)) => match build_matches.subcommand() {
            Some(("docker", docker_matches)) => {
                inspect_docker_transparency_log(
                    docker_matches.get_one::<String>("image").unwrap(),
                    docker_matches.get_one::<String>("format").cloned(),
                    docker_matches.get_one::<String>("fields").cloned(),
                )
                .await;
            }
            Some(("maven", maven_matches)) => {
                inspect_maven_transparency_log(
                    maven_matches.get_one::<String>("gav").unwrap(),
                    maven_matches.get_one::<String>("format").cloned(),
                    maven_matches.get_one::<String>("fields").cloned(),
                )
                .await;
            }
            _ => {}
        },
        _ => {} //this should be handled by clap arg_required_else_help
    }
}
