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

use crate::CONF_FILE_PATH_MSG_STARTER;
use anyhow::anyhow;
use lazy_static::lazy_static;
use pyrsia::cli_commands::config;
use pyrsia::cli_commands::node;
use pyrsia::node_api::model::cli::{
    RequestAddAuthorizedNode, RequestDockerBuild, RequestDockerLog, RequestMavenBuild,
    RequestMavenLog,
};
use regex::Regex;
use serde_json::Value;
use std::collections::HashSet;
use std::io;
use std::io::BufRead;
use std::net::Ipv4Addr;

const CONF_REMINDER_MESSAGE: &str = "Please make sure the pyrsia CLI config is up to date and matches the node configuration. For more information, run 'pyrsia config --show'";

pub fn config_add() -> anyhow::Result<()> {
    let default_config = config::CliConfig {
        ..Default::default()
    };

    let mut new_cfg = config::CliConfig {
        host: read_interactive_input(
            &format!("Enter host: [{}]", default_config.host),
            &default_config.host,
            &valid_host_name,
        ),
        ..Default::default()
    };
    new_cfg.port = read_interactive_input(
        &format!("Enter port: [{}]", default_config.port),
        &default_config.port,
        &valid_port,
    );
    new_cfg.disk_allocated = read_interactive_input(
        &format!(
            "Enter disk space to be allocated to pyrsia(Please enter with units ex: 10 GB): [{}]",
            default_config.disk_allocated
        ),
        &default_config.disk_allocated,
        &valid_disk_space,
    );

    config::add_config(new_cfg)
}

pub fn config_edit(
    host_name: Option<String>,
    port: Option<String>,
    disk_space: Option<String>,
) -> anyhow::Result<()> {
    let mut cli_config = config::get_config()?;

    let mut errors: Vec<String> = Vec::new();

    match host_name.map_or_else(
        || Err("Invalid value for Hostname".to_owned()),
        valid_host_name,
    ) {
        Ok(host_name) => cli_config.host = host_name,
        Err(description) => errors.push(description),
    }

    match port.map_or_else(
        || Err("Invalid value for Port Number".to_owned()),
        valid_port,
    ) {
        Ok(port) => cli_config.port = port,
        Err(description) => errors.push(description),
    }

    match disk_space.map_or_else(
        || Err("Invalid value for Disk Allocation".to_owned()),
        valid_disk_space,
    ) {
        Ok(disk_space) => cli_config.disk_allocated = disk_space,
        Err(description) => errors.push(description),
    }

    if errors.is_empty() {
        config::add_config(cli_config)
    } else {
        errors.into_iter().for_each(|x| println!("{}", x));
        Err(anyhow!("Invalid pyrsia config"))
    }
}

pub fn config_show() {
    match config::get_config_file_path() {
        Ok(path_buf) => {
            println!(
                "{} {}",
                CONF_FILE_PATH_MSG_STARTER,
                path_buf.into_os_string().into_string().unwrap()
            )
        }
        Err(error) => {
            println!("Error retrieving config file path: {}", error);
        }
    }
    let result = config::get_config();
    match result {
        Ok(config) => {
            println!("{}", config)
        }
        Err(error) => {
            println!("No Node Configured: {}", error);
        }
    };
}

pub async fn authorize(peer_id: &str) {
    match node::add_authorized_node(RequestAddAuthorizedNode {
        peer_id: peer_id.to_owned(),
    })
    .await
    {
        Ok(()) => println!("Authorize request successfully handled."),
        Err(error) => println!("Authorize request failed with error: {}", error),
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
            print_logs(logs);
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
            print_logs(logs);
        }
        Err(error) => {
            println!("Inspect log request failed with error: {:?}", error);
        }
    };
}

fn print_logs(logs: String) {
    let logs_as_json: Value = serde_json::from_str(logs.as_str()).unwrap();
    println!("{}", serde_json::to_string_pretty(&logs_as_json).unwrap());
}

/// Read user input interactively until the validation passed
fn read_interactive_input(
    cli_prompt: &str,
    default_val: &str,
    validation_func: &dyn Fn(String) -> Result<String, String>,
) -> String {
    loop {
        println!("{}", cli_prompt);
        let mut buffer = String::new();
        if let Ok(bytes_read) = io::stdin().lock().read_line(&mut buffer) {
            if bytes_read > 0 {
                let mut input = buffer.lines().next().unwrap();
                if input.is_empty() {
                    input = default_val;
                }
                if let Ok(r) = validation_func(input.to_owned()) {
                    break r;
                }
            }
        }
    }
}

/// Returns true if input is a valid hostname or a valid IPv4 address
fn valid_host_name(input: String) -> Result<String, String> {
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
        input.parse::<Ipv4Addr>().is_ok()
    }

    if valid_ipv4_address(&input) || valid_hostname(&input) {
        Ok(input)
    } else {
        Err("Invalid value for Hostname".to_owned())
    }
}

fn valid_port(input: String) -> Result<String, String> {
    match input.parse::<u16>() {
        Ok(_) => Ok(input),
        Err(_) => Err("Invalid value for Port Number".to_owned()),
    }
}

/// Disk space will only accept integer values. Currently we it will accept value greater than 0 GB till 4096 GB
fn valid_disk_space(input: String) -> Result<String, String> {
    const DISK_SPACE_NUM_MIN: u16 = 0;
    const DISK_SPACE_NUM_MAX: u16 = 4096;
    lazy_static! {
        static ref DISK_SPACE_RE: Regex = Regex::new(r"^([0-9,\.]+)\s+(GB)$").unwrap();
    }
    if DISK_SPACE_RE.is_match(&input) {
        let captured_groups = DISK_SPACE_RE.captures(&input).unwrap();
        //Group 1 is numeric part including decimal & Group 2 is metric part
        if let Ok(disk_space_num) = captured_groups.get(1).unwrap().as_str().parse::<u16>() {
            if DISK_SPACE_NUM_MIN < disk_space_num && disk_space_num <= DISK_SPACE_NUM_MAX {
                return Ok(format!(
                    "{} {}",
                    disk_space_num,
                    captured_groups.get(2).unwrap().as_str()
                ));
            }
        }
    }
    Err("Invalid value for Disk Allocation".to_owned())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use crate::cli::handlers::{
        config_edit, config_show, valid_disk_space, valid_host_name, valid_port,
    };
    use pyrsia::cli_commands::config::{self, CliConfig};

    #[test]
    fn test_valid_host() {
        let valid_hosts = vec!["pyrsia.io", "localhost", "10.10.10.255"];
        assert!(valid_hosts
            .into_iter()
            .all(|x| valid_host_name(x.to_owned()).is_ok()));
    }

    #[test]
    fn test_invalid_host() {
        let invalid_hosts = vec![
            "-pyrsia.io",
            "@localhost",
            "%*%*%*%*NO_SENSE_AS_HOST@#$*@#$*@#$*",
        ];
        assert!(!invalid_hosts
            .into_iter()
            .any(|x| valid_host_name(x.to_owned()).is_ok()));
    }

    #[test]
    fn test_valid_port() {
        let valid_ports = vec!["0", "8988", "65535"];
        assert!(valid_ports
            .into_iter()
            .all(|x| valid_port(x.to_owned()).is_ok()));
    }

    #[test]
    fn test_invalid_port() {
        let invalid_ports = vec!["-1", "65536"];
        assert!(!invalid_ports
            .into_iter()
            .any(|x| valid_port(x.to_owned()).is_ok()));
    }

    #[test]
    fn test_valid_disk_space() {
        let valid_disk_space_list = vec!["100 GB", "1 GB", "4096 GB"];
        assert!(valid_disk_space_list
            .into_iter()
            .all(|x| valid_disk_space(x.to_owned()).is_ok()));
    }

    #[test]
    fn test_invalid_disk_space() {
        let invalid_disk_space_list = vec![
            "0 GB",
            "4097 GB",
            "100GB",
            "100gb",
            "5.84 GB",
            "5..84 GB",
            "5..84 GB",
            "5.84.22 GB",
        ];
        assert!(!invalid_disk_space_list
            .into_iter()
            .any(|x| valid_disk_space(x.to_owned()).is_ok()));
    }

    #[test]
    fn test_valid_config_edit() {
        let existing_cli_config = config::get_config().unwrap();
        let host_name = "some.localhost";
        let port = u16::MAX.to_string();
        let disk_space = "10 GB";
        let config_edit_result = config_edit(
            Some(host_name.to_owned()),
            Some(port.clone()),
            Some(disk_space.to_owned()),
        );
        let updated_cli_config = config::get_config().unwrap();
        if config_edit_result.is_ok() {
            //restore the config to original state after test
            let _restore_config = config::add_config(existing_cli_config);
        }
        assert_eq!(
            CliConfig {
                host: host_name.to_owned(),
                port,
                disk_allocated: disk_space.to_owned()
            },
            updated_cli_config
        );
    }

    #[test]
    fn test_invalid_config_edit() {
        let existing_cli_config = config::get_config().unwrap();
        let host_name = ".some.localhost";
        let port = (u16::MAX as u32 + 1).to_string();
        let disk_space = "10GB";
        let config_edit_result = config_edit(
            Some(host_name.to_owned()),
            Some(port.clone()),
            Some(disk_space.to_owned()),
        );
        let updated_cli_config = config::get_config().unwrap();
        if config_edit_result.is_ok() {
            //restore the config to original state after test
            let _restore_config = config::add_config(existing_cli_config);
        }
        assert_ne!(
            CliConfig {
                host: host_name.to_owned(),
                port,
                disk_allocated: disk_space.to_owned()
            },
            updated_cli_config
        );
    }

    #[test]
    fn test_config_show() {
        config_show();
    }
}
