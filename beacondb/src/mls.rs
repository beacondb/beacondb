use std::io;

use anyhow::Result;
use rusqlite::{Connection, Transaction};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Serialize)]
struct Record {
    radio: RadioType,
    mcc: u16,
    net: u16,
    area: u32,
    cell: u64,
    unit: Option<u16>,
    lon: f64,
    lat: f64,
    range: f64,
    samples: u32,
    created: u64,
    updated: u64,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "UPPERCASE")]
enum RadioType {
    Gsm,
    Umts,
    Lte,
}

pub fn main(conn: &mut Connection) -> Result<()> {
    let tx = conn.transaction()?;
    {
        let mut stmt = tx.prepare("insert into cell (radio, country, network, area, cell, unit, lon, lat, range, samples, created, updated) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12)")?;
        let mut reader = csv::Reader::from_reader(io::stdin());
        for (i, result) in reader.deserialize().enumerate() {
            let record: Record = result?;
            if (i % 1_000_000) == 0 && i != 0 {
                println!("{i}");
            }

            let radio = match record.radio {
                RadioType::Gsm => 0,
                RadioType::Umts => 1,
                RadioType::Lte => 2,
            };

            // let cell: i32 = match record.cell.try_into() {
            //     Ok(x) => x,
            //     Err(_) => {
            //         println!("overflowing cell id: {record:?}");
            //         continue;
            //     }
            // };

            // no networks have conflicts where they both use `null` and `0`
            let unit = record.unit.unwrap_or_default();

            stmt.execute((
                radio,
                record.mcc,
                record.net,
                record.area,
                record.cell,
                unit,
                record.lon,
                record.lat,
                record.range,
                record.samples,
                record.created,
                record.updated,
            ))?;
        }
    }
    tx.commit()?;

    Ok(())
}