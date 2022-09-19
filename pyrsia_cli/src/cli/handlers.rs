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

use lazy_static::lazy_static;
use pyrsia::cli_commands::config;
use pyrsia::cli_commands::node;
use pyrsia::node_api::model::cli::{
    RequestDockerBuild, RequestDockerLog, RequestMavenBuild, RequestMavenLog,
};
use regex::Regex;
use std::collections::HashSet;
use std::io;
use std::io::BufRead;
use std::net::Ipv4Addr;

const CONF_REMINDER_MESSAGE: &str = "Please make sure the pyrsia CLI config is up to date and matches the node configuration. For more information, run 'pyrsia config --show'";

pub fn config_add() {
    let mut new_cfg = config::CliConfig {
        host: read_interactive_input("Enter host: ", &valid_host),
        ..Default::default()
    };
    new_cfg.port = read_interactive_input("Enter port: ", &valid_port);
    new_cfg.disk_allocated = read_interactive_input(
        "Enter disk space to be allocated to pyrsia(Please enter with units ex: 10 GB): ",
        &valid_disk_space,
    );

    let result = config::add_config(new_cfg);
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
    let result = config::get_config();
    match result {
        Ok(config) => {
            println!("{}", config)
        }
        Err(error) => {
            println!("No Node Configured:       {}", error);
        }
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

fn handle_request_build_result(build_result: Result<String, reqwest::Error>) {
    match build_result {
        Ok(build_id) => {
            println!(
                "Build request successfully handled. Build with ID {} has been started.",
                build_id
            );
        }
        Err(error) => {
            println!("Build request failed with error: {}", error);
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

pub async fn inspect_docker_transparency_log(image: &str) {
    let result = node::inspect_docker_transparency_log(RequestDockerLog {
        image: image.to_owned(),
    })
    .await;
    match result {
        Ok(logs) => {
            println!("Inspect log request returns the following logs: {}", logs);
        }
        Err(error) => {
            println!("Inspect log request failed with error: {:?}", error);
        }
    };
}

pub async fn inspect_maven_transparency_log(gav: &str) {
    let result = node::inspect_maven_transparency_log(RequestMavenLog {
        gav: gav.to_owned(),
    })
    .await;
    match result {
        Ok(logs) => {
            println!("Inspect log request returns the following logs: {}", logs);
        }
        Err(error) => {
            println!("Inspect log request failed with error: {:?}", error);
        }
    };
}

/// Read user input interactively until the validation passed
fn read_interactive_input<'a>(
    cli_prompt: &str,
    validation_func: &'a dyn Fn(&str) -> bool,
) -> String {
    loop {
        println!("{}", cli_prompt);
        let mut buffer = String::new();
        match io::stdin().lock().read_line(&mut buffer) {
            Ok(bytes_read) => {
                if bytes_read > 0 {
                    let input = buffer.lines().next().unwrap();
                    if validation_func(input.clone()) {
                        break input.to_string();
                    }
                }
            }
            Err(_) => {}
        }
    }
}

/// Returns true if input is a valid hostname or a valid IPv4 address
fn valid_host(input: &str) -> bool {
    /// Returns true if input is a valid hostname as per the definition
    /// at https://man7.org/linux/man-pages/man7/hostname.7.html, otherwise false
    fn valid_hostname(input: &str) -> bool {
        if input.is_empty() || input.len() > 253 {
            return false;
        }
        lazy_static! {
            static ref HOSTNAME_REGEX: Regex = Regex::new(r"^(([a-zA-Z0-9]{1,63}|[a-zA-Z0-9][a-zA-Z0-9\-]{0,62})\.)*([a-zA-Z0-9]{1,63}|[a-zA-Z0-9][a-zA-Z0-9\-]{0,62})$").unwrap();
        }
        HOSTNAME_REGEX.is_match(input)
    }

    /// Returns true if input is a valid IPv4 address, otherwise false
    fn valid_ipv4_address(input: &str) -> bool {
        match input.parse::<Ipv4Addr>() {
            Ok(_) => true,
            Err(_) => false,
        }
    }

    valid_ipv4_address(input) || valid_hostname(input)
}

fn valid_port(input: &str) -> bool {
    match input.parse::<u16>() {
        Ok(_) => true,
        Err(_) => false,
    }
}

fn valid_disk_space(input: &str) -> bool {
    const DISK_SPACE_NUM_MIN: u16 = 0;
    const DISK_SPACE_NUM_MAX: u16 = 4096;
    lazy_static! {
        static ref DISK_SPACE_RE: Regex = Regex::new(r"^([0-9]{1,4})\s+(GB)$").unwrap();
    }
    // let disk_space_re: Regex = Regex::new(r"^([0-9]{1,4})(\s*)(GB)$").unwrap();
    if DISK_SPACE_RE.is_match(input) {
        let captured_groups = DISK_SPACE_RE.captures(input).unwrap();
        let disk_space_num = captured_groups
            .get(1)
            .unwrap()
            .as_str()
            .parse::<u16>()
            .unwrap();
        DISK_SPACE_NUM_MIN < disk_space_num && disk_space_num <= DISK_SPACE_NUM_MAX
    } else {
        false
    }
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use crate::cli::handlers::{valid_disk_space, valid_host, valid_port};

    #[test]
    fn test_valid_host() {
        let valid_hosts = vec!["pyrsia.io", "localhost", "10.10.10.255"];
        assert!(valid_hosts.into_iter().all(|x| valid_host(x)));
    }

    #[test]
    fn test_invalid_host() {
        let invalid_hosts = vec![
            "-pyrsia.io",
            "@localhost",
            "%*%*%*%*NO_SENSE_AS_HOST@#$*@#$*@#$*",
        ];
        assert!(!invalid_hosts.into_iter().any(|x| valid_host(x)));
    }

    #[test]
    fn test_valid_port() {
        let valid_ports = vec!["0", "8988", "65535"];
        assert!(valid_ports.into_iter().all(|x| valid_port(x)));
    }

    #[test]
    fn test_invalid_port() {
        let invalid_ports = vec!["-1", "65536"];
        assert!(!invalid_ports.into_iter().any(|x| valid_port(x)));
    }

    #[test]
    fn test_valid_disk_space() {
        let valid_disk_space_list = vec!["100 GB", "1 GB", "4096 GB"];
        assert!(valid_disk_space_list
            .into_iter()
            .all(|x| valid_disk_space(x)));
    }

    #[test]
    fn test_invalid_disk_space() {
        let invalid_disk_space_list = vec!["0 GB", "4097 GB", "100GB", "100gb"];
        assert!(!invalid_disk_space_list
            .into_iter()
            .any(|x| valid_disk_space(x)));
    }
}
