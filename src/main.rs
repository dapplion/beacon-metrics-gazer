use crate::config::fetch_genesis;
use crate::ranges::parse_ranges;
use crate::util::{current_epoch_start_slot, resolve_path_or_url, to_next_epoch_start};
use anyhow::{anyhow, Context, Result};
use clap::Parser;
use config::{fetch_config, ConfigSpec, Genesis};
use hyper::header::{HeaderName, CONTENT_TYPE};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, HeaderMap, Request, Response, Server};
use metrics::{
    set_gauge, HEAD_PARTICIPATION, INACTIVITY_SCORES, SOURCE_PARTICIPATION, TARGET_PARTICIPATION,
};
use prettytable::{format, Cell, Row, Table};
use prometheus::{Encoder, TextEncoder};
use ssz_state::{deserialize_partial_state, StatePartial};
use std::convert::Infallible;
use std::net::SocketAddr;
use std::str::FromStr;
use std::time::Duration;
use tokio::time;

//use ssz_state::parse_epoch_participation;
//use ssz_state::ConfigSpec;

mod config;
mod metrics;
mod ranges;
mod ssz_state;
mod util;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Beacon HTTP API URL: http://1.2.3.4:4000
    url: String,
    /// Extra headers sent to each request to the beacon node API at `url`.
    /// Same format as curl: `-H "Authorization: Bearer {token}"`
    #[arg(long, short = 'H')]
    headers: Option<Vec<String>>,
    /// Index ranges to group IDs as JSON or TXT. Example:
    /// `{"0..100": "lh-geth-0", "100..200": "lh-geth-1"}
    #[arg(long)]
    ranges: Option<String>,
    /// Local path or URL containing a file with index ranges
    /// with the format as defined in --ranges
    #[arg(long)]
    ranges_file: Option<String>,
    /// Dump participation ranges print to stderr on each fetch
    #[arg(long)]
    dump: bool,
    /// Metrics server port
    #[arg(long, short, default_value_t = 8080)]
    port: u16,
    /// Metrics server bind address
    #[arg(long, default_value = "127.0.0.1")]
    address: String,
}

type IndexGroups = Vec<(String, Vec<usize>)>;
struct RangeSummary {
    target_participation_ratio: f32,
    head_participation_ratio: f32,
    source_participation_ratio: f32,
    inactivity_scores_avg: f32,
}
type ParticipationByRange = Vec<(String, Vec<usize>, RangeSummary)>;

async fn handle_metrics_server_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // Create the response
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();

    Ok(Response::builder()
        .header("Content-Type", encoder.format_type())
        .body(Body::from(buffer))
        .unwrap())
}

const CONTENT_TYPE_SSZ: &str = "application/octet-stream";

async fn fetch_epoch_participation(
    config: &ConfigSpec,
    beacon_url: &str,
    extra_headers: &HeaderMap,
) -> Result<StatePartial> {
    let req = reqwest::Client::new()
        .get(format!("{beacon_url}/eth/v2/debug/beacon/states/head",))
        .header(reqwest::header::ACCEPT, CONTENT_TYPE_SSZ)
        .headers(extra_headers.clone())
        .send()
        .await?;

    // Guard against bad responses, else this function will attempt to decode a 404 html as if it
    // was an SSZ state
    if !req.status().is_success() {
        return Err(anyhow!(
            "getStates returned not success code {}",
            req.status().as_str()
        ));
    }

    // Additional guard in case the server sends JSON instead of SSZ. Could happen if a proxy or
    // some middleware strips the CONTENT_TYPE header out of this request
    if let Some(content_type) = req.headers().get(CONTENT_TYPE) {
        if let Ok(content_type) = content_type.to_str() {
            if !content_type.contains(CONTENT_TYPE_SSZ) {
                return Err(anyhow!(
                    "getState content-type not {}: {}",
                    CONTENT_TYPE_SSZ,
                    content_type
                ));
            }
        }
    }

    let state_buf = req.bytes().await?;

    deserialize_partial_state(config, &state_buf)
}

// https://github.com/ethereum/consensus-specs/blob/4a27f855439c16612ab1ae3995d71bed54f979ea/specs/altair/beacon-chain.md#participation-flag-indices
const TIMELY_SOURCE_FLAG_INDEX: u8 = 0;
const TIMELY_TARGET_FLAG_INDEX: u8 = 1;
const TIMELY_HEAD_FLAG_INDEX: u8 = 2;
const TIMELY_SOURCE: u8 = 1 << TIMELY_SOURCE_FLAG_INDEX;
const TIMELY_TARGET: u8 = 1 << TIMELY_TARGET_FLAG_INDEX;
const TIMELY_HEAD: u8 = 1 << TIMELY_HEAD_FLAG_INDEX;

fn has_flag(flag: u8, mask: u8) -> bool {
    flag & mask == mask
}

fn participation_avg(participation: &[u8], indexes: &[usize], flag_mask: u8) -> f32 {
    let participant_sum: u32 = indexes
        .iter()
        .map(|index| has_flag(participation[*index], flag_mask) as u32)
        .sum::<u32>();
    participant_sum as f32 / indexes.len() as f32
}

fn score_avg(values: &[u64], indexes: &[usize]) -> f32 {
    let sum: u64 = indexes.iter().map(|index| values[*index]).sum();
    sum as f32 / indexes.len() as f32
}

fn group_target_participation(
    index_groups: &IndexGroups,
    state: &StatePartial,
) -> ParticipationByRange {
    index_groups
        .iter()
        .map(|(range_name, indexes)| {
            (
                range_name.clone(),
                indexes.clone(),
                RangeSummary {
                    target_participation_ratio: participation_avg(
                        &state.previous_epoch_participation,
                        indexes,
                        TIMELY_TARGET,
                    ),
                    source_participation_ratio: participation_avg(
                        &state.previous_epoch_participation,
                        indexes,
                        TIMELY_SOURCE,
                    ),
                    head_participation_ratio: participation_avg(
                        &state.previous_epoch_participation,
                        indexes,
                        TIMELY_HEAD,
                    ),
                    inactivity_scores_avg: score_avg(&state.inactivity_scores, indexes),
                },
            )
        })
        .collect()
}

fn set_participation_to_metrics(participation_by_range: &ParticipationByRange) {
    for (range_name, _, summary) in participation_by_range.iter() {
        set_gauge(
            &SOURCE_PARTICIPATION,
            &[range_name],
            summary.source_participation_ratio as f64,
        );
        set_gauge(
            &TARGET_PARTICIPATION,
            &[range_name],
            summary.target_participation_ratio as f64,
        );
        set_gauge(
            &HEAD_PARTICIPATION,
            &[range_name],
            summary.head_participation_ratio as f64,
        );
        set_gauge(
            &INACTIVITY_SCORES,
            &[range_name],
            summary.inactivity_scores_avg as f64,
        );
    }
}

fn dump_participation_to_stdout(participation_by_range: &ParticipationByRange) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    table.add_row(Row::new(vec![
        Cell::new("Name"),
        Cell::new("Range"),
        Cell::new("Source"),
        Cell::new("Target"),
        Cell::new("Head"),
    ]));

    for (range_name, range, summary) in participation_by_range.iter() {
        table.add_row(Row::new(vec![
            Cell::new(range_name),
            Cell::new(&format!("{:?}", &range)),
            Cell::new(&summary.source_participation_ratio.to_string()),
            Cell::new(&summary.target_participation_ratio.to_string()),
            Cell::new(&summary.head_participation_ratio.to_string()),
        ]));
    }

    table.printstd();
}

async fn task_fetch_state_every_epoch(
    genesis: &Genesis,
    config: &ConfigSpec,
    beacon_url: &str,
    extra_headers: &HeaderMap,
    ranges: &IndexGroups,
    dump: bool,
) -> Result<()> {
    loop {
        match current_epoch_start_slot(genesis, config) {
            Err(e) => eprintln!("error computing current epoch: {:?}", e),
            Ok(slot) => {
                if slot == 0 {
                    println!("before genesis, going to sleep")
                } else {
                    // Only after genesis
                    match fetch_epoch_participation(config, beacon_url, extra_headers).await {
                        Err(e) => eprintln!("error fetching state: {:?}", e),
                        Ok(state) => {
                            let participation_by_range = group_target_participation(ranges, &state);
                            set_participation_to_metrics(&participation_by_range);
                            if dump {
                                dump_participation_to_stdout(&participation_by_range);
                            }
                        }
                    }
                }
            }
        }

        // Run once on boot, then every interval at end of epoch

        time::sleep(to_next_epoch_start(genesis, config).unwrap_or_else(|e| {
            eprintln!("error computing to_next_epoch_start: {:?}", e);
            Duration::from_secs(config.seconds_per_slot * config.slots_per_epoch)
        }))
        .await;
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let beacon_url = cli.url.clone();

    println!("connecting to beacon URL {:?}", beacon_url);

    let mut extra_headers = HeaderMap::new();
    if let Some(headers_str) = cli.headers {
        for header_str in headers_str {
            let parts: Vec<&str> = header_str.split(':').collect();
            if parts.len() != 2 {
                return Err(anyhow!("Invalid header: {}", header_str));
            }

            let name = HeaderName::from_str(parts[0])?;
            let value = parts[1].trim().parse()?;
            extra_headers.insert(name, value);
        }
        println!("extra headers {:?}", extra_headers);
    }

    // Parse groups file mapping index ranges to host names
    let ranges_str = if let Some(ranges_str) = &cli.ranges {
        ranges_str.clone()
    } else if let Some(path_or_url) = &cli.ranges_file {
        resolve_path_or_url(path_or_url).await?
    } else {
        return Err(anyhow!("Must set --groups or --groups_file"));
    };
    let ranges = parse_ranges(&ranges_str)?;
    println!("index ranges ---\n{}\n---", &ranges_str);

    let genesis = fetch_genesis(&beacon_url).await.context("fetch_genesis")?;
    println!("beacon genesis {:?}", genesis);

    let config = fetch_config(&beacon_url).await.context("fetch_config")?;
    println!("beacon config {:?}", config);

    // Background task fetching state every interval and registering participation
    // in metrics with provided index ranges
    tokio::spawn(async move {
        task_fetch_state_every_epoch(
            &genesis,
            &config,
            &beacon_url,
            &extra_headers,
            &ranges,
            cli.dump,
        )
        .await
    });

    // Start metrics server

    let addr = SocketAddr::new(cli.address.parse()?, cli.port);
    let server = Server::bind(&addr).serve(make_service_fn(|_conn| async {
        Ok::<_, Infallible>(service_fn(handle_metrics_server_request))
    }));

    println!("Server is running on http://{}", addr);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
