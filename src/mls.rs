use std::io;

use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};

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

pub async fn import(pool: PgPool) -> Result<()> {
    // let mut tx = pool.begin().await?;
    {
        let mut reader = csv::Reader::from_reader(io::stdin());
        for (i, result) in reader.deserialize().enumerate() {
            let record: Record = result?;
            if (i % 1_000_000) == 0 && i != 0 {
                eprintln!("{i}");
            }

            let radio = match record.radio {
                RadioType::Gsm => 0,
                RadioType::Umts => 1,
                RadioType::Lte => 2,
            };

            let cell: i32 = match record.cell.try_into() {
                Ok(x) => x,
                Err(_) => {
                    // println!("overflowing cell id: {record:?}");
                    continue;
                }
            };

            // no networks have conflicts where they both use `null` and `0`
            let unit = record.unit.unwrap_or_default();
            println!(
                "{},{},{},{},{},{},{},{},{}",
                radio,
                record.mcc,
                record.net,
                record.area,
                cell,
                unit,
                record.lon,
                record.lat,
                record.range,
            );

            // query!(
            //     "insert into cell_mls (radio, country, network, area, cell, unit, x, y, r) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
            //     radio,
            //     record.mcc,
            //     record.net,
            //     record.area,
            //     cell,
            //     unit,
            //     record.lon,
            //     record.lat,
            //     record.range,
            // ).execute(&mut *tx).await?;
        }
    }
    // tx.commit().await?;

    Ok(())
}
