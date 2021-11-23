use crate::commands::config::get_config;
use reqwest;

pub async fn ping() -> Result<String, reqwest::Error> {
    let result = get_config();
    let mut url = String::new();
    let _data = match result {
        Ok(data) => {
            url = data;
        }
        Err(error) => {
            println!("Error: {}", error);
        }
    };

    let node_url = format!("http://{}/v2", url);

    let response = reqwest::get(node_url).await?.text().await?;

    Ok(response)
}
