use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

use crate::geoip::GeoIpConfig;

#[derive(Deserialize)]
pub struct Config {
    pub database_url: String,
    pub http_port: u16,

    pub geoip: Option<GeoIpConfig>,
    pub stats: Option<StatsConfig>,
}

#[derive(Deserialize)]
pub struct StatsConfig {
    pub path: PathBuf,

    // amount of reports that aren't stored in the database but should still
    // be added to the total count
    pub archived_reports: i64,
}

pub fn load(path: &Path) -> Result<Config> {
    let data = fs::read_to_string(path).context("Failed to read config")?;
    let config = toml::from_str(&data).context("Failed to parse config")?;
    Ok(config)
}
