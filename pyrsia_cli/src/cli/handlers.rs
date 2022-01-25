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

use super::config::*;
use super::node::*;
use clap::ArgMatches;
use std::collections::HashSet;

pub fn handle_config_add(config_matches: &ArgMatches) {
    let node_config = config_matches.value_of("add").unwrap();
    let _result = add_config(String::from(node_config));
    println!("Node configured:      {}", node_config);
}

pub fn handle_config_show() {
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

pub async fn handle_node_ping() {
    let result = ping().await;
    let _resp = match result {
        Ok(resp) => {
            println!("Connection Successfull !!")
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    };
}

pub async fn handle_node_status() {
    let result = status().await;
    let _resp = match result {
        Ok(resp) => {
            println!("{}", resp);
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    };
}

pub async fn handle_node_list() {
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
