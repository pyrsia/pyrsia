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
