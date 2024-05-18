use std::{collections::BTreeMap, path::Path};

use anyhow::Result;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct GpsRecord {
    pub timestamp_ms: u64,
    pub lat: f64,
    pub lon: f64,
    pub accuracy: f64,
    pub speed: f64,
}

pub fn load(path: &Path) -> Result<Vec<GpsRecord>> {
    let mut output = Vec::new();
    let mut reader = csv::Reader::from_path(path)?;
    for result in reader.deserialize() {
        let record: GpsRecord = result?;
        output.push(record);
    }

    Ok(output)
}
