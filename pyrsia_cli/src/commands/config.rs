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
    use expectest::expect;
    use expectest::prelude::*;

    #[test]
    fn test_get_config_errors_when_config_file_not_found() {
        expect!(get_config()).to(be_err());
    }
}
