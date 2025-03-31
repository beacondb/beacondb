//! Models and functionality to work with the config file.

use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

/// Rust representation of the configuration
#[derive(Deserialize)]
pub struct Config {
    /// URL of the database
    pub database_url: String,

    /// Port on which beacondb listens
    pub http_port: u16,

    /// Resolution of the h3 hexagons in the data map preview
    pub h3_resolution: u8,

    /// Optional statistics configuration
    pub stats: Option<StatsConfig>,
}

/// Rust representation of the statistics configuration
#[derive(Deserialize)]
pub struct StatsConfig {
    /// Location where the statistics should be exported in JSON format
    pub path: PathBuf,

    /// The amount of reports that aren't stored in the database but should still
    /// be added to the total count
    pub archived_reports: i64,
}

pub fn load(path: &Path) -> Result<Config> {
    let data = fs::read_to_string(path).context("Failed to read config")?;
    let config = toml::from_str(&data).context("Failed to parse config")?;
    Ok(config)
}
