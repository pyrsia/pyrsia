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

use std::fmt::{Display, Formatter};
use std::net::Ipv4Addr;
use std::path::PathBuf;

use anyhow::{anyhow, Result};
use lazy_static::lazy_static;
use regex::Regex;
use serde::{Deserialize, Serialize};

const CONF_FILE: &str = "pyrsia-cli";

/// The name of the environment variable to use for hardcoding the location
/// of the configuration file during testing.
const PYRSIA_CONFIG_LOCATION_FOR_TEST: &str = "PYRSIA_CONFIG_LOCATION_FOR_TEST";

/// Gets the path of the configuration file. We always use [`confy::load_path`] and
/// [`confy::store_path`] (instead of [`confy::load`] and [`confy::store`] respectively).
/// That way we have full control over the exact location of the configuration file.
/// This is particularly useful during testing. Setting the environment variable named
/// [`PYRSIA_CONFIG_LOCATION_FOR_TEST`] will set the config path to that value.
fn get_config_path() -> Result<PathBuf, confy::ConfyError> {
    if cfg!(test) {
        if let Ok(config_path_for_test) = std::env::var(PYRSIA_CONFIG_LOCATION_FOR_TEST) {
            Ok(PathBuf::from(config_path_for_test))
        } else {
            confy::get_configuration_file_path(CONF_FILE, None)
        }
    } else {
        confy::get_configuration_file_path(CONF_FILE, None)
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CliConfig {
    pub host: String,
    pub port: String,
    pub disk_allocated: String,
}

impl Default for CliConfig {
    fn default() -> Self {
        CliConfig {
            host: "localhost".to_string(),
            port: "7888".to_string(),
            disk_allocated: "10 GB".to_string(),
        }
    }
}

impl Display for CliConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config_toml = toml::to_string_pretty(&self).expect("toml format error");
        write!(f, "{}", config_toml)
    }
}

impl PartialEq for CliConfig {
    fn eq(&self, other: &Self) -> bool {
        self.host.as_str() == other.host.as_str()
            && self.port.as_str() == other.port.as_str()
            && self.disk_allocated.as_str() == other.disk_allocated.as_str()
    }
}

pub fn add_config(new_cfg: CliConfig) -> Result<()> {
    let config_path = get_config_path()?;

    let mut cfg: CliConfig = confy::load_path(&config_path)?;
    if !new_cfg.host.is_empty() {
        cfg.host = new_cfg.host
    }

    if !new_cfg.port.is_empty() {
        cfg.port = new_cfg.port
    }
    // need more validation for checking units
    if !new_cfg.disk_allocated.is_empty() {
        cfg.disk_allocated = new_cfg.disk_allocated
    }

    confy::store_path(&config_path, &cfg)?;

    Ok(())
}

pub fn config_remove() -> Result<()> {
    let cfg_patch = confy::get_configuration_file_path(CONF_FILE, None)?;
    if cfg_patch.exists() {
        std::fs::remove_file(cfg_patch)?;
    }
    Ok(())
}

pub fn config_edit(
    host_name: Option<String>,
    port: Option<String>,
    disk_space: Option<String>,
) -> Result<()> {
    let mut cli_config = get_config()?;

    let mut errors: Vec<String> = Vec::new();

    if let Some(validation_result) = host_name.map(valid_host_name) {
        match validation_result {
            Ok(host_name) => cli_config.host = host_name,
            Err(description) => errors.push(description),
        }
    }

    if let Some(validation_result) = port.map(valid_port) {
        match validation_result {
            Ok(port) => cli_config.port = port,
            Err(description) => errors.push(description),
        }
    }

    if let Some(validation_result) = disk_space.map(valid_disk_space) {
        match validation_result {
            Ok(disk_space) => cli_config.disk_allocated = disk_space,
            Err(description) => errors.push(description),
        }
    }

    if errors.is_empty() {
        add_config(cli_config)
    } else {
        errors.into_iter().for_each(|x| println!("{}", x));
        Err(anyhow!("Invalid pyrsia config"))
    }
}

/// Returns true if input is a valid hostname or a valid IPv4 address
pub fn valid_host_name(input: String) -> Result<String, String> {
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

pub fn valid_port(input: String) -> Result<String, String> {
    match input.parse::<u16>() {
        Ok(_) => Ok(input),
        Err(_) => Err("Invalid value for Port Number".to_owned()),
    }
}

/// Disk space will only accept integer values. Currently we it will accept value greater than 0 GB till 4096 GB
pub fn valid_disk_space(input: String) -> Result<String, String> {
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

pub fn get_config() -> Result<CliConfig> {
    let config_path = get_config_path()?;

    let cfg: CliConfig = confy::load_path(config_path)?;

    Ok(cfg)
}

pub fn get_config_file_path() -> Result<PathBuf> {
    confy::get_configuration_file_path(CONF_FILE, None).map_err(|e| e.into())
}

#[cfg(test)]
#[cfg(not(tarpaulin_include))]
mod tests {
    use super::*;
    use serial_test::serial;
    use std::fs;

    #[test]
    #[serial]
    fn test_config_file_update() {
        setup_temp_home_dir_and_execute(|| {
            let cli_config_1 = CliConfig {
                port: "7888".to_string(),
                ..Default::default()
            };
            let cli_config_2 = CliConfig {
                port: "7878".to_string(),
                ..Default::default()
            };

            add_config(cli_config_1.clone()).expect("add_config failed");
            let current_cli_config = get_config().expect("get_config failed");
            assert_eq!(current_cli_config.port, cli_config_1.port);

            add_config(cli_config_2.clone()).expect("add_config failed");
            let current_cli_config = get_config().expect("get_config failed");
            assert_eq!(current_cli_config.port, cli_config_2.port);
        });
    }

    #[test]
    #[serial]
    fn test_config_file_remove() {
        setup_temp_home_dir_and_execute(|| {
            add_config(CliConfig {
                ..Default::default()
            })
            .expect("add_config_failed");
            let cfg_file = confy::get_configuration_file_path(CONF_FILE, None)
                .expect("cannot get config file path");
            assert!(cfg_file.exists(), "config file does not exist");

            config_remove().expect("remove_config failed");
            assert!(!cfg_file.exists(), "config must not exist");
        });
    }

    #[test]
    #[serial]
    fn test_remove_not_existed_config_file() {
        config_remove().expect("remove_config failed");
    }

    fn setup_temp_home_dir_and_execute<F>(op: F)
    where
        F: FnOnce(),
    {
        let tmp_dir = tempfile::tempdir()
            .expect("could not create temporary directory")
            .into_path();

        std::env::set_var(
            PYRSIA_CONFIG_LOCATION_FOR_TEST,
            tmp_dir.join("pyrsia-cli.config"),
        );

        op();

        std::env::remove_var(PYRSIA_CONFIG_LOCATION_FOR_TEST);
        fs::remove_dir_all(tmp_dir).expect("failed to clean up temporary directory");
    }

    fn test_common_valid_config_edit(
        host_name: Option<String>,
        port: Option<String>,
        disk_allocated: Option<String>,
    ) {
        let existing_cli_config = get_config().unwrap();
        let config_edit_result =
            config_edit(host_name.clone(), port.clone(), disk_allocated.clone());
        let updated_cli_config = get_config().unwrap();
        if config_edit_result.is_ok() {
            //restore the config to original state after test
            let _restore_config = add_config(existing_cli_config.clone());
        }
        assert_eq!(
            CliConfig {
                host: host_name.unwrap_or(existing_cli_config.host),
                port: port.unwrap_or(existing_cli_config.port),
                disk_allocated: disk_allocated.unwrap_or(existing_cli_config.disk_allocated),
            },
            updated_cli_config
        );
    }

    #[test]
    #[serial]
    fn test_config_edit_only_with_valid_host_name() {
        test_common_valid_config_edit(Some("some.localhost".to_string()), None, None);
    }

    #[test]
    #[serial]
    fn test_config_edit_only_with_valid_port() {
        test_common_valid_config_edit(None, Some(u16::MAX.to_string()), None);
    }

    #[test]
    #[serial]
    fn test_config_edit_only_with_valid_disk_allocated() {
        test_common_valid_config_edit(None, None, Some("10 GB".to_string()));
    }

    #[test]
    #[serial]
    fn test_config_edit_with_all_valid_attributes() {
        test_common_valid_config_edit(
            Some("some.localhost".to_string()),
            Some(u16::MAX.to_string()),
            Some("10 GB".to_string()),
        );
    }

    #[test]
    #[serial]
    fn test_invalid_config_edit() {
        let existing_cli_config = get_config().unwrap();
        let host_name = ".some.localhost";
        let port = (u16::MAX as u32 + 1).to_string();
        let disk_space = "10GB";
        let config_edit_result = config_edit(
            Some(host_name.to_owned()),
            Some(port.clone()),
            Some(disk_space.to_owned()),
        );
        let updated_cli_config = get_config().unwrap();
        if config_edit_result.is_ok() {
            //restore the config to original state after test
            let _restore_config = add_config(existing_cli_config);
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
    fn test_get_config_file_path() {
        let config_file_path = get_config_file_path();
        assert!(config_file_path.is_ok());
    }

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
}
