use anyhow::{anyhow, Result};
use std::{
    path::Path,
    time::{Duration, SystemTime, UNIX_EPOCH},
};
use tokio::fs;
use url::Url;

use crate::config::{ConfigSpec, Genesis};

/// Given a path_or_url, if it's a valid URL download it. Else read from it as a local path
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

/// Return the start slot of the current epoch.
/// Returns 0 if before genesis.
pub fn current_epoch_start_slot(genesis: &Genesis, config: &ConfigSpec) -> Result<u64> {
    let now = SystemTime::now();
    let now_unix_sec = now.duration_since(UNIX_EPOCH)?.as_secs();
    if now_unix_sec < genesis.genesis_time {
        // Before genesis
        return Ok(0);
    }

    let since_genesis_sec = now_unix_sec - genesis.genesis_time;
    let since_genesis_slots = since_genesis_sec / config.seconds_per_slot;
    let slot_in_epoch = since_genesis_slots % config.slots_per_epoch;
    Ok(since_genesis_slots - slot_in_epoch)
}

/// Compute the time to the next epoch from now.
/// If before genesis returns the time to epoch 1 (not 0).
pub fn to_next_epoch_start(genesis: &Genesis, config: &ConfigSpec) -> Result<Duration> {
    let now = SystemTime::now();
    let now_unix_sec = now.duration_since(UNIX_EPOCH)?.as_secs();
    if now_unix_sec < genesis.genesis_time {
        // Before genesis
        let one_epoch_sec = 1 * config.slots_per_epoch * config.seconds_per_slot;
        return Ok(Duration::from_secs(
            one_epoch_sec + genesis.genesis_time - now_unix_sec,
        ));
    }

    let since_genesis_sec = now_unix_sec - genesis.genesis_time;
    let since_genesis_slots = since_genesis_sec / config.seconds_per_slot;
    let slot_in_epoch = since_genesis_slots % config.slots_per_epoch;
    let slots_to_next_epoch = config.slots_per_epoch - slot_in_epoch;
    let start_slot_next_epoch = since_genesis_slots + slots_to_next_epoch;
    let start_time_next_epoch = UNIX_EPOCH
        + Duration::from_secs(
            genesis.genesis_time + start_slot_next_epoch * config.seconds_per_slot,
        );
    Ok(start_time_next_epoch.duration_since(now)?)
}
