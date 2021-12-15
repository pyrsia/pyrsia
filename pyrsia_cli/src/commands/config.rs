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

use anyhow::{Context, Result};

pub fn add_config(content: String) -> Result<()> {
    let path = "pyrsia-cli.conf";
    std::fs::write(path, content)
        .with_context(|| format!("could not write to conf file `{}`", path))?;
    Ok(())
}

pub fn get_config() -> Result<String> {
    let path = "pyrsia-cli.conf";
    let content =
        std::fs::read_to_string(path).with_context(|| format!("could not read file `{}`", path))?;
    Ok(content)
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
