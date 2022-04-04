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
use clap::{command,ErrorKind};

#[tokio::main]
async fn main() {
    // parsing command line arguments
    let matches = cli_parser();

    // checking and preparing responses for each command and arguments

    let (ls, pg, st) = (
        matches.is_present("list"),
        matches.is_present("ping"),
        matches.is_present("status"),
    );
    match (ls, pg, st) {
        (true, false, false) => node_list().await,
        (false, true, false) => node_ping().await,
        (false, false, true) => node_status().await,
        _ => {
            command!().error(
                ErrorKind::UnknownArgument,
                "Can only modify one version field",
            )
            .exit();
        }
    };
    match matches.subcommand() {
        // config subcommand
        Some(("config", config_matches)) => {
            if config_matches.is_present("add") || config_matches.is_present("edit") {
                config_add();
            }
            if config_matches.is_present("show") {
                config_show();
            }
        }

        Some(("node", node_matches)) => {
            if node_matches.is_present("ping") {
                node_ping().await;
            } else if node_matches.is_present("list") {
                node_list().await;
            } else if node_matches.is_present("status") {
                node_status().await;
            } else {
                println!("No help topic for '{:?}'", node_matches)
            }
        }
        None => {
            command!().error(
                ErrorKind::UnrecognizedSubcommand,
                "Can only modify one version field",
            )
            .exit();
        },

        _ => unreachable!(),
    }
}
