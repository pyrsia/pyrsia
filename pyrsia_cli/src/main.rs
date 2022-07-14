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

#[tokio::main]
async fn main() {
    // parsing command line arguments
    let matches = cli_parser();

    // checking and preparing responses for each command and its arguments if applicable

    match matches.subcommand() {
        Some(("config", config_matches)) => {
            if config_matches.is_present("add") || config_matches.is_present("edit") {
                config_add();
            }
            if config_matches.is_present("show") {
                config_show();
            }
        }
        Some(("build", build_matches)) => match build_matches.subcommand() {
            Some(("docker", docker_matches)) => {
                request_docker_build(docker_matches.get_one::<String>("manifest").unwrap()).await;
            }
            Some(("maven", maven_matches)) => {
                request_maven_build(maven_matches.get_one::<String>("gav").unwrap()).await;
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
        _ => {} //this should be handled by clap arg_required_else_help
    }
}
