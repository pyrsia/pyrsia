pub mod commands;

use commands::config::*;
use commands::node::*;

extern crate clap;
use clap::{load_yaml, App};

#[tokio::main]
async fn main() {
    // There are 2 methods listed below, we can decide which method to use as we get more clarity on cli structure.
    //As of now yaml looks more neat

    // 1. Yaml method of parsing commands
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
                        println!("Connection Successfull!! {}", resp)
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

    // 2. Builder pattern for structuring cli : this might allow more advanced configuration
    /*let matches = App::new("pyrsia node")
    .version("0.1.0")
    .author("Mitali B. mitalib@jfrog.com")
    .about("Zero-Trust Universal Decentralized Binary Network")
    .subcommand(
            App::new("config")
                .about("node config")
                .arg(Arg::new("add").short('a').about("add config")),
    .get_matches();*/
}
