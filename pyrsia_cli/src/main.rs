pub mod commands;

use commands::config::*;
use commands::node::*;
use std::collections::HashSet;

extern crate clap;
use clap::{load_yaml, App};

#[tokio::main]
async fn main() {
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from(yaml).get_matches();

    match matches.subcommand() {
        // config subcommand
        Some(("config", config_matches)) => {
            if config_matches.is_present("add") {
                let node_config = config_matches.value_of("add").unwrap();
                let _result = add_config(String::from(node_config));
                println!("Node configured: {}", node_config);
            }
            if config_matches.is_present("show") {
                let result = get_config();

                let _url = match result {
                    Ok(url) => {
                        println!("Node Config: {}", url)
                    }
                    Err(error) => {
                        println!("Error: {}", error);
                    }
                };
            }
        }

        //node subcommand
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
            }
            if node_matches.is_present("ls") {
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
            }
        }

        None => println!("No subcommand was used"),

        _ => unreachable!(),
    }
}
