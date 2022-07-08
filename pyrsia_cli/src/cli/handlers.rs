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
use std::collections::HashSet;
use std::io;
use std::io::BufRead;

pub fn config_add() {
    println!("Enter host: ");
    let mut new_cfg = CliConfig {
        host: io::stdin().lock().lines().next().unwrap().unwrap(),
        ..Default::default()
    };

    println!("Enter port: ");
    new_cfg.port = io::stdin().lock().lines().next().unwrap().unwrap();

    println!("Enter disk space to be allocated to pyrsia(Please enter with units ex: 10 GB): ");
    new_cfg.disk_allocated = io::stdin().lock().lines().next().unwrap().unwrap();

    let result = add_config(new_cfg);
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
    let result = get_config();
    match result {
        Ok(config) => {
            println!("{}", config)
        }
        Err(error) => {
            println!("No Node Configured:       {}", error);
        }
    };
}

pub async fn node_ping() {
    let result = ping().await;
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
    let result = status().await;
    match result {
        Ok(resp) => {
            println!("{}", resp);
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    }
}

pub async fn node_list() {
    let result = peers_connected().await;
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
