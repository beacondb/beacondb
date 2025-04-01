//! Serde types for Mozilla Location Service.

use std::io;

use anyhow::Result;
use serde::{Deserialize, Serialize};

use crate::model::CellRadio;

/// MLS serde representation of a record
#[derive(Debug, Deserialize, Serialize)]
struct Record {
    radio: RadioType,
    mcc: i16,
    net: i16,
    area: i32,
    cell: i64,
    unit: Option<i16>,
    lon: f32,
    lat: f32,
    range: f32,
}

/// Type of radio as specified in MLS
#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum RadioType {
    Gsm,
    Umts,
    Lte,
}

/// Reformat MLS data
pub fn format() -> Result<()> {
    let mut reader = csv::Reader::from_reader(io::stdin());
    for (i, result) in reader.deserialize().enumerate() {
        let record: Record = result?;
        if (i % 1_000_000) == 0 && i != 0 {
            eprintln!("{i}");
        }

        let radio = match record.radio {
            RadioType::Gsm => CellRadio::Gsm,
            RadioType::Umts => CellRadio::Wcdma,
            RadioType::Lte => CellRadio::Lte,
        };

        let unit = record.unit.unwrap_or_default();
        println!(
            "{},{},{},{},{},{},{},{},{}",
            radio as i16,
            record.mcc,
            record.net,
            record.area,
            record.cell,
            unit,
            record.lat,
            record.lon,
            record.range,
        );
    }

    Ok(())
}
