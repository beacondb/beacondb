use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use serde::Serialize;
use sqlx::{query, query_scalar, MySqlPool};

use crate::{bounds::Bounds, model::Transmitter};

pub async fn run(pool: MySqlPool, stats_path: Option<&Path>) -> Result<()> {
    let reports = query!("select id, raw from submission where processed_at is null order by id")
        .fetch_all(&pool)
        .await?;

    let count = reports.len();
    if count == 0 {
        println!("Nothing to process");
        return Ok(());
    }
    println!("{count} submissions need processing");

    let mut modified: BTreeMap<Transmitter, Bounds> = BTreeMap::new();
    let mut tx = pool.begin().await?;
    for next in reports {
        // TODO: parsing failures should be noted but not halt the queue
        let (pos, txs) = super::report::extract(&next.raw)
            .with_context(|| format!("Failed to parse report #{}", next.id))?;

        for x in txs {
            if let Some(b) = modified.get_mut(&x) {
                *b = *b + (pos.latitude, pos.longitude);
            } else {
                if let Some(b) = x.lookup(&pool).await? {
                    modified.insert(x, b + (pos.latitude, pos.longitude));
                } else {
                    modified.insert(x, Bounds::new(pos.latitude, pos.longitude));
                }
            }
        }

        query!("update submission set processed_at = now() where id = ?", next.id).execute(&mut *tx).await?;
    }


    println!("writing");
    for (x, b) in modified {
        match x {
            Transmitter::Cell {
                radio,
                country,
                network,
                area,
                cell,
                unit,
            } => {
                query!(
                    "replace into cell (radio, country, network, area, cell, unit, min_lat, min_lon, max_lat, max_lon) values (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
                    radio, country, network, area, cell, unit, b.min_lat, b.min_lon, b.max_lat, b.max_lon         ,
                )
                .execute(&mut *tx)
                .await?;
            }
            Transmitter::Wifi { mac } => {
                query!(
                    "replace into wifi (mac, min_lat, min_lon, max_lat, max_lon) values (?, ?, ?, ?, ?)",
                    &mac[..], b.min_lat, b.min_lon, b.max_lat, b.max_lon
                )
                .execute(&mut *tx)
                .await?;
            }
            Transmitter::Bluetooth{ mac } => {
                query!(
                    "replace into bluetooth (mac, min_lat, min_lon, max_lat, max_lon) values (?, ?, ?, ?, ?)",
                    &mac[..], b.min_lat, b.min_lon, b.max_lat, b.max_lon 
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    tx.commit().await?;

    if let Some(path) = stats_path {
        let stats = Stats {
            total_wifi: query_scalar!("select count(*) from wifi").fetch_one(&pool).await?,
            total_cell: query_scalar!("select count(*) from cell").fetch_one(&pool).await?,
            total_bluetooth: query_scalar!("select count(*) from bluetooth").fetch_one(&pool).await?,
            total_countries: query_scalar!("select count(distinct country) from cell").fetch_one(&pool).await?
        };

        let data = serde_json::to_string_pretty(&stats)?;
        fs::write(path, data)?;
    }

    Ok(())
}

#[derive(Serialize)]
struct Stats {
    total_wifi: i64,
    total_cell: i64,
    total_bluetooth: i64,
    total_countries: i64,
}
