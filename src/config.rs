use std::{
    fs,
    path::{Path, PathBuf},
};

use anyhow::{Context, Result};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct Config {
    pub database_url: String,
    pub http_port: u16,
    pub stats_path: Option<PathBuf>,
}

pub fn load(path: &Path) -> Result<Config> {
    let data = fs::read_to_string(&path).context("Failed to read config")?;
    let config = toml::from_str(&data).context("Failed to parse config")?;
    Ok(config)
}
