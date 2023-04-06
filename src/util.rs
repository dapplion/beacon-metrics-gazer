use anyhow::{anyhow, Result};
use std::path::Path;
use tokio::fs;
use url::Url;

pub async fn resolve_path_or_url(path_or_url: &str) -> Result<String> {
    if Url::parse(path_or_url).is_ok() {
        // If it's a valid URL
        let response = reqwest::get(path_or_url).await?;
        let content = response.text().await?;
        Ok(content)
    } else if Path::new(path_or_url).exists() {
        // If it's a valid local path
        let content = fs::read_to_string(path_or_url).await?;
        Ok(content)
    } else {
        Err(anyhow!(
            "Input is not a valid URL or local path: {}",
            path_or_url
        ))
    }
}
