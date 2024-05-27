use std::io;

use anyhow::{Context, Result};
use chrono::{TimeZone, Utc};
use serde::Deserialize;
use sqlx::{query, PgPool};

#[derive(Debug, Deserialize)]
struct Record {
    radio: RadioType,
    mcc: i16,
    net: i16,
    area: i32,
    cell: i64,
    unit: Option<i16>,
    lon: f64,
    lat: f64,
    range: f64,
    // samples: u32,
    created: i64,
    updated: i64,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
enum RadioType {
    Gsm,
    Umts,
    Lte,
}

pub async fn main(pool: &PgPool) -> Result<()> {
    let mut tx = pool.begin().await?;
    let mut reader = csv::Reader::from_reader(io::stdin());
    for (i, result) in reader.deserialize().enumerate() {
        let record: Record = result?;
        if (i % 1_000) == 0 && i != 0 {
            println!("{i}");
        }

        let radio = match record.radio {
            RadioType::Gsm => 0,
            RadioType::Umts => 1,
            RadioType::Lte => 2,
        };

        let cell: i32 = match record.cell.try_into() {
            Ok(x) => x,
            Err(_) => {
                println!("overflowing cell id: {record:?}");
                continue;
            }
        };

        // no networks have conflicts where they both use `null` and `0`
        let unit = record.unit.unwrap_or_default();

        let created_at = Utc
            .timestamp_opt(record.created, 0)
            .single()
            .context("timestamp out of range")?;
        let updated_at = Utc
            .timestamp_opt(record.updated, 0)
            .single()
            .context("timestamp out of range")?;

        query!(
            "insert into cell (
                radio, country, network, area, cell, unit, x, y, r, created_at, updated_at
            ) values (
                $1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11
            )",
            radio,
            record.mcc,
            record.net,
            record.area,
            cell,
            unit,
            record.lon,
            record.lat,
            record.range,
            created_at,
            updated_at
        )
        .execute(&mut *tx)
        .await
        .with_context(|| format!("failed to insert record {i}: {record:?}"))?;
    }
    tx.commit().await?;

    Ok(())
}
