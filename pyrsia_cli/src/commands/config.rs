use anyhow::{Context, Result};

const CONF_FILE: &str = "pyrsia-cli.conf";

pub fn add_config(content: String) -> Result<()> {
    std::fs::write(CONF_FILE, content)
        .with_context(|| format!("could not write to conf file `{}`", CONF_FILE))?;
    Ok(())
}

pub fn get_config() -> Result<String> {
    let content = std::fs::read_to_string(CONF_FILE)
        .with_context(|| format!("could not read file `{}`", CONF_FILE))?;
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
