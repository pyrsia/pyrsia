extern crate clap;

use clap::{load_yaml, App};
use std::fs;
use anyhow::{Context, Result};


fn main() {
    // There are 2 methods listed below, we can decide which method to use as we get more clarity on cli structure.
    //As of now yaml looks more neat

    // 1. Yaml method of parsing structuring
    let yaml = load_yaml!("cli.yaml");
    let matches = App::from(yaml).get_matches();

    match matches.subcommand() {
        // config subcommand
        Some(("config", config_matches)) => {

            if config_matches.is_present("add") {
                let node_config = config_matches.value_of("add").unwrap();
                fs::write("pyrsia-cli.conf", node_config).expect("Unable to write to conf file");
                println!("Node configured: {}", node_config);
            }
            if config_matches.is_present("show") {
                let result = show_config();
            }
        }

        //node subcommand
        Some(("node", node_matches)) => {
            println!("Calling ping api");
        }

        None => println!("No subcommand was used"),
        _ => unreachable!(),
    }

    // 2. Builder pattern for structuring cli : this might allow more advanced configuration
    /*let matches = App::new("pyrsia node")
    .version("0.1.0")
    .author("Mitali B. mitalib@jfrog.com")
    .about("Zero-Trust Universal Decentralized Binary Network")
    .arg(Arg::with_name("FILE")
          .help("File to print.")
          .empty_values(false)
      )
    .get_matches();*/
}

fn show_config() -> Result<()> {
    let path = "pyrsia-cli.conf";
    let content = std::fs::read_to_string(path)
        .with_context(|| format!("could not read conf file `{}`", path))?;
    println!("node config: {}", content);
    Ok(())
}
