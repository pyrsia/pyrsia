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

extern crate pyrsia_node;

pub mod commands;

use commands::config::*;
use commands::node::*;
use std::collections::HashSet;

extern crate clap;
use clap::{crate_authors, crate_description, crate_version, App, AppSettings, Arg};

#[tokio::main]
async fn main() {
    //parsing command line arguments

    let matches = App::new("pyrsia")
        .author(crate_authors!("\n"))
        .version(crate_version!())
        .about(crate_description!())
        .setting(AppSettings::SubcommandRequiredElseHelp)
        // Config subcommand
        .subcommand(
            App::new("config")
                .short_flag('c')
                .long_flag("config")
                .about("Pyrsia config commands")
                .setting(AppSettings::ArgRequiredElseHelp)
                .setting(AppSettings::AllowHyphenValues)
                .arg(
                    Arg::new("add")
                        .short('a')
                        .long("add")
                        .help("Adds a node configuration")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("edit")
                        .long("edit")
                        .short('e')
                        .help("Edits a node configuration")
                        .takes_value(true),
                )
                .arg(
                    Arg::new("remove")
                        .long("remove")
                        .short('r')
                        .help("Removes the stored node configuration"),
                )
                .arg(
                    Arg::new("show")
                        .long("show")
                        .short('s')
                        .help("Shows the stored node configuration"),
                ),
        )
        // Node subcommand
        .subcommand(
            App::new("node")
                .short_flag('n')
                .long_flag("node")
                .about("Node commands")
                .setting(AppSettings::ArgRequiredElseHelp)
                .setting(AppSettings::AllowHyphenValues)
                .arg(
                    Arg::new("ping")
                        .short('p')
                        .long("ping")
                        .help("Ping configured pyrsia node"),
                )
                .arg(
                    Arg::new("status")
                        .long("status")
                        .short('s')
                        .help("Shows node information"),
                )
                .arg(
                    Arg::new("list")
                        .short('l')
                        .help("Shows list of connected Peers"),
                ),
        )
        .get_matches();

    // checking and preparing responses for each command and arguments

    match matches.subcommand() {
        // config subcommand
        Some(("config", config_matches)) => {
            if config_matches.is_present("add") {
                let node_config = config_matches.value_of("add").unwrap();
                let _result = add_config(String::from(node_config));
                println!("Node configured:      {}", node_config);
            }
            if config_matches.is_present("show") {
                let result = get_config();

                let _url = match result {
                    Ok(url) => {
                        println!("Node URL:      {}", url)
                    }
                    Err(error) => {
                        println!("No Node Configured:       {}", error);
                    }
                };
            }
        }

        Some(("node", node_matches)) => {
            if node_matches.is_present("ping") {
                let result = ping().await;
                let _resp = match result {
                    Ok(resp) => {
                        println!("Connection Successfull !! {}", resp)
                    }
                    Err(error) => {
                        println!("Error: {}", error);
                    }
                };
            } else if node_matches.is_present("list") {
                let result = peers_connected().await;
                let _resp = match result {
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
                };
            } else if node_matches.is_present("status") {
                let result = status().await;
                let _resp = match result {
                    Ok(resp) => {
                        println!("{}", resp);
                    }
                    Err(error) => {
                        println!("Error: {}", error);
                    }
                };
            } else {
                println!("No help topic for '{:?}'", node_matches)
            }
        }

        None => println!("No subcommand was used"),

        _ => unreachable!(),
    }
}
