use anyhow::{Error, Result};
use reqwest::header::HeaderMap;
use serde::Deserialize;

#[derive(Debug)]
pub struct ConfigSpec {
    pub seconds_per_slot: u64,
    pub slots_per_epoch: u64,
    pub slots_per_historical_root: usize,
    pub epochs_per_historical_vector: usize,
    pub epochs_per_slashings_vector: usize,
}

#[allow(non_snake_case)]
#[derive(Deserialize)]
struct ConfigSpecResponseData {
    SECONDS_PER_SLOT: String,
    SLOTS_PER_EPOCH: String,
    SLOTS_PER_HISTORICAL_ROOT: String,
    EPOCHS_PER_HISTORICAL_VECTOR: String,
    EPOCHS_PER_SLASHINGS_VECTOR: String,
}

#[derive(Deserialize)]
struct ConfigSpecResponse {
    data: ConfigSpecResponseData,
}

pub async fn fetch_config(url: &str, extra_headers: &HeaderMap) -> Result<ConfigSpec> {
    let response = reqwest::Client::new()
        .get(format!("{}/eth/v1/config/spec", url))
        .headers(extra_headers.clone())
        .send()
        .await?;
    let data: ConfigSpecResponse = response.json().await?;
    Ok(ConfigSpec {
        seconds_per_slot: parse_usize(&data.data.SECONDS_PER_SLOT, "SECONDS_PER_SLOT")? as u64,
        slots_per_epoch: parse_usize(&data.data.SLOTS_PER_EPOCH, "SLOTS_PER_EPOCH")? as u64,
        slots_per_historical_root: parse_usize(
            &data.data.SLOTS_PER_HISTORICAL_ROOT,
            "SLOTS_PER_HISTORICAL_ROOT",
        )?,
        epochs_per_historical_vector: parse_usize(
            &data.data.EPOCHS_PER_HISTORICAL_VECTOR,
            "EPOCHS_PER_HISTORICAL_VECTOR",
        )?,
        epochs_per_slashings_vector: parse_usize(
            &data.data.EPOCHS_PER_SLASHINGS_VECTOR,
            "EPOCHS_PER_SLASHINGS_VECTOR",
        )?,
    })
}

fn parse_usize(usize_str: &str, name: &'static str) -> Result<usize> {
    usize_str.parse().map_err(|e| Error::new(e).context(name))
}

#[derive(Debug, Deserialize)]
pub struct Genesis {
    pub genesis_time: u64,
}

#[derive(Deserialize)]
struct BeaconGenesisResponse {
    data: BeaconGenesisResponseData,
}

#[derive(Deserialize)]
struct BeaconGenesisResponseData {
    genesis_time: String,
}

pub async fn fetch_genesis(url: &str, extra_headers: &HeaderMap) -> Result<Genesis> {
    let response = reqwest::Client::new()
        .get(format!("{}/eth/v1/beacon/genesis", url))
        .headers(extra_headers.clone())
        .send()
        .await?;
    let data: BeaconGenesisResponse = response.json().await?;
    Ok(Genesis {
        genesis_time: data.data.genesis_time.parse()?,
    })
}
