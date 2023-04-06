use crate::config::fetch_genesis;
use crate::groups_file::read_groups_file;
use anyhow::{Context, Result};
use clap::Parser;
use config::{fetch_config, ConfigSpec};
use hyper::service::{make_service_fn, service_fn};
use hyper::{Body, Request, Response, Server};
use lazy_static::lazy_static;
use prettytable::{format, Cell, Row, Table};
use prometheus::register_int_counter;
use prometheus::{Encoder, IntCounter, TextEncoder};
use ssz_state::{deserialize_partial_state, StatePartial};
use std::convert::Infallible;
use std::fs::File;
use std::io::{prelude::*, BufReader};
use std::net::SocketAddr;
use std::ops::Range;
use std::time::Duration;
use tokio::time;

//use ssz_state::parse_epoch_participation;
//use ssz_state::ConfigSpec;

mod config;
mod groups_file;
mod ssz_state;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    url: String,
    groups: String,
}

type GroupRanges = Vec<(String, Range<usize>)>;

lazy_static! {
    static ref HIGH_FIVE_COUNTER: IntCounter =
        register_int_counter!("highfives", "Number of high fives received").unwrap();
}

async fn handle_request(_req: Request<Body>) -> Result<Response<Body>, Infallible> {
    // Increment the counter
    HIGH_FIVE_COUNTER.inc();

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

async fn on_every_end_of_epoch() {
    loop {
        time::sleep(Duration::from_secs(60)).await;
        //        let new_state = fetch_state().await;
        //        *state.lock().unwrap() = new_state;
    }
}

async fn fetch_epoch_participation(
    config: &ConfigSpec,
    beacon_url: &str,
    // slot: u64,
) -> Result<StatePartial> {
    let req = reqwest::Client::new()
        .get(format!("{beacon_url}/eth/v2/debug/beacon/states/head",))
        .header(reqwest::header::ACCEPT, "application/octet-stream")
        .send()
        .await?;
    let state_buf = req.bytes().await?;

    let mut f = std::fs::File::create("state.ssz").unwrap();
    f.write_all(&state_buf).unwrap();

    Ok(deserialize_partial_state(config, &state_buf)?)
}

// https://github.com/ethereum/consensus-specs/blob/4a27f855439c16612ab1ae3995d71bed54f979ea/specs/altair/beacon-chain.md#participation-flag-indices
// const TIMELY_SOURCE_FLAG_INDEX: u8 = 0;
const TIMELY_TARGET_FLAG_INDEX: u8 = 1;
// const TIMELY_HEAD_FLAG_INDEX: u8 = 2;
// const TIMELY_SOURCE: u8 = 1 << TIMELY_SOURCE_FLAG_INDEX;
const TIMELY_TARGET: u8 = 1 << TIMELY_TARGET_FLAG_INDEX;
// const TIMELY_HEAD: u8 = 1 << TIMELY_HEAD_FLAG_INDEX;

fn has_flag(flag: u8, mask: u8) -> bool {
    flag & mask == mask
}

fn set_participation_to_metrics(groups: &GroupRanges, state: &StatePartial) {
    let mut table = Table::new();
    table.set_format(*format::consts::FORMAT_NO_BORDER_LINE_SEPARATOR);

    table.add_row(Row::new(vec![
        Cell::new("Client"),
        Cell::new("Range"),
        Cell::new("Target participation"),
    ]));

    for (group_id, range) in groups.iter() {
        let target_count: u32 = state.previous_epoch_participation[range.clone()]
            .iter()
            .map(|f| has_flag(*f, TIMELY_TARGET) as u32)
            .sum();
        let target_ratio = target_count as f32 / (range.end - range.start) as f32;
        // println!("{:?} \t{:?} \t {:?}", group_id, range, target_ratio);
        table.add_row(Row::new(vec![
            Cell::new(&group_id),
            Cell::new(&format!("{:?}", &range)),
            Cell::new(&target_ratio.to_string()),
        ]));
    }
    table.printstd();
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();
    let beacon_url = cli.url;
    let groups_filepath = cli.groups;
    let groups = read_groups_file(&groups_filepath)?;

    println!("connecting to beacon URL {:?}", beacon_url);

    let genesis = fetch_genesis(&beacon_url).await.context("fetch_genesis")?;
    println!("beacon genesis {:?}", genesis);

    let config = fetch_config(&beacon_url).await.context("fetch_config")?;
    println!("beacon config {:?}", config);

    let state = fetch_epoch_participation(&config, &beacon_url)
        .await
        .context("fetch_epoch_participation")?;

    set_participation_to_metrics(&groups, &state);

    tokio::spawn(async move {
        on_every_end_of_epoch().await;
    });

    // Start metrics server
    let addr = SocketAddr::from(([127, 0, 0, 1], 8080));
    let make_svc =
        make_service_fn(|_conn| async { Ok::<_, Infallible>(service_fn(handle_request)) });
    let server = Server::bind(&addr).serve(make_svc);

    println!("Server is running on http://{}", addr);
    if let Err(e) = server.await {
        eprintln!("server error: {}", e);
    }

    Ok(())
}
