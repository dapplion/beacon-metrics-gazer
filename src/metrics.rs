use anyhow::Result;
use lazy_static::lazy_static;
use prometheus::GaugeVec;

lazy_static! {
    pub static ref SOURCE_PARTICIPATION: GaugeVec = try_create_gauge_vec(
        "beacon_network_source_participation",
        "Source participation in previous epoch by pre-defined named ranges",
        &["range"]
    )
    .unwrap();
}
lazy_static! {
    pub static ref TARGET_PARTICIPATION: GaugeVec = try_create_gauge_vec(
        "beacon_network_target_participation",
        "Target participation in previous epoch by pre-defined named ranges",
        &["range"]
    )
    .unwrap();
}
lazy_static! {
    pub static ref HEAD_PARTICIPATION: GaugeVec = try_create_gauge_vec(
        "beacon_network_head_participation",
        "Head participation in previous epoch by pre-defined named ranges",
        &["range"]
    )
    .unwrap();
}
lazy_static! {
    pub static ref INACTIVITY_SCORES: GaugeVec = try_create_gauge_vec(
        "beacon_network_inactivity_scores",
        "Inactivity scores avg by pre-defined named ranges",
        &["range"]
    )
    .unwrap();
}

/// Attempts to create a `GaugeVec`, returning `Err` if the registry does not accept the gauge
/// (potentially due to naming conflict).
fn try_create_gauge_vec(name: &str, help: &str, label_names: &[&str]) -> Result<GaugeVec> {
    let opts = prometheus::Opts::new(name, help);
    let counter_vec = GaugeVec::new(opts, label_names)?;
    prometheus::register(Box::new(counter_vec.clone()))?;
    Ok(counter_vec)
}

/// If `gauge_vec.is_ok()`, sets the gauge with the given `name` to the given `value`
/// otherwise returns false.
pub fn set_gauge(gauge_vec: &GaugeVec, name: &[&str], value: f64) -> bool {
    gauge_vec
        .get_metric_with_label_values(name)
        .map(|v| {
            v.set(value);
            true
        })
        .unwrap_or_else(|_| false)
}
