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

extern crate anyhow;
extern crate confy;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::fmt::{Display, Formatter};

const CONF_FILE: &str = "pyrsia-cli";

#[derive(Debug, Serialize, Deserialize)]
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
            disk_allocated: "5.84 GB".to_string(),
        }
    }
}

impl Display for CliConfig {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let config_toml = toml::to_string_pretty(&self).expect("toml format error");
        write!(f, "{}", config_toml)
    }
}

pub fn add_config(new_cfg: CliConfig) -> Result<()> {
    let mut cfg: CliConfig = confy::load(CONF_FILE)?;
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

    confy::store(CONF_FILE, &cfg)?;

    Ok(())
}

pub fn get_config() -> Result<CliConfig> {
    let cfg: CliConfig = confy::load(CONF_FILE)?;

    Ok(cfg)
}

#[cfg(test)]
mod tests {
    use super::*;
    use assay::assay;
    use directories::ProjectDirs;
    use std::path::PathBuf;

    fn tear_down() {
        let config_dir_str = get_configuration_directory();

        let path: PathBuf = [
            config_dir_str.to_owned(),
            format!("{}.toml", CONF_FILE.to_owned()),
        ]
        .iter()
        .collect();

        if path.exists() {
            std::fs::remove_dir_all(path.parent().unwrap()).expect("Failed to remove directory");
        }
    }

    #[assay(teardown = tear_down())]
    fn test_config_file_update() {
        let cfg: CliConfig = get_config().expect("could not get conf file");
        assert_eq!(cfg.port, "7888".to_string());
        let cfg = CliConfig {
            port: "7878".to_string(),
            ..cfg
        };

        add_config(cfg).expect("could not update conf file");
        let new_cfg: CliConfig = get_config().expect("could not get conf file");
        assert_eq!(new_cfg.port, "7878".to_string());
    }

    fn get_configuration_directory() -> String {
        let project = ProjectDirs::from("rs", "", CONF_FILE).expect("bad config dir");

        let config_dir_option = project.config_dir().to_str();

        if let Some(x) = config_dir_option {
            return x.to_string();
        } else {
            return "".to_string();
        }
    }
}
