use std::io;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{query, MySqlPool};

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

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum RadioType {
    Gsm,
    Umts,
    Lte,
}

pub fn format() -> Result<()> {
    let mut reader = csv::Reader::from_reader(io::stdin());
    for (i, result) in reader.deserialize().enumerate() {
        let record: Record = result?;
        if (i % 1_000_000) == 0 && i != 0 {
            eprintln!("{i}");
        }

        let radio = match record.radio {
            RadioType::Gsm => "gsm",
            RadioType::Umts => "wcdma",
            RadioType::Lte => "lte",
        };

        let cell: i32 = match record.cell.try_into() {
            Ok(x) => x,
            Err(_) => {
                // println!("overflowing cell id: {record:?}");
                continue;
            }
        };

        let unit = record.unit.unwrap_or_default();
        println!(
            "{},{},{},{},{},{},{},{},{}",
            radio,
            record.mcc,
            record.net,
            record.area,
            cell,
            unit,
            record.lat,
            record.lon,
            record.range,
        );
    }

    Ok(())
}
