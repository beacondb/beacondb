use std::{collections::BTreeMap, fs, path::Path};

use anyhow::{Context, Result};
use futures::{StreamExt, TryStreamExt};
use serde::Serialize;
use sqlx::{query, query_scalar, PgPool};

use crate::{bounds::Bounds, model::Transmitter};

pub async fn run(pool: PgPool, stats_path: Option<&Path>) -> Result<()> {
    let mut reports =
        query!("select id, raw from report where processed_at is null order by id").fetch(&pool);
    let mut modified: BTreeMap<Transmitter, Bounds> = BTreeMap::new();
    let mut tx = pool.begin().await?;

    while let Some(report) = reports.try_next().await? {
        // TODO: parsing failures should be noted
        let result = super::report::extract(&report.raw)
            .with_context(|| format!("Failed to parse report #{}: {}", report.id, &report.raw));
        let (pos, txs) = match result {
            Ok(x) => x,
            Err(e) => {
                println!("{e}");
                continue;
            }
        };

        for x in txs {
            if let Some(b) = modified.get_mut(&x) {
                *b = *b + (pos.latitude, pos.longitude);
            } else if let Some(b) = x.lookup(&pool).await? {
                modified.insert(x, b + (pos.latitude, pos.longitude));
            } else {
                modified.insert(x, Bounds::new(pos.latitude, pos.longitude));
            }
        }

        query!(
            "update report set processed_at = now() where id = $1",
            report.id
        )
        .execute(&mut *tx)
        .await?;
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
                    "insert into cell (radio, country, network, area, cell, unit, min_lat, min_lon, max_lat, max_lon) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
                     on conflict (radio, country, network, area, cell, unit) do update set min_lat = EXCLUDED.min_lat, min_lon = EXCLUDED.min_lon, max_lat = EXCLUDED.max_lat, max_lon = EXCLUDED.max_lon
                    ",
                    radio as i16, country, network, area, cell, unit, b.min_lat, b.min_lon, b.max_lat, b.max_lon
                )
                .execute(&mut *tx)
                .await?;
            }
            Transmitter::Wifi { mac } => {
                query!(
                    "insert into wifi (mac, min_lat, min_lon, max_lat, max_lon) values ($1, $2, $3, $4, $5)
                     on conflict (mac) do update set min_lat = EXCLUDED.min_lat, min_lon = EXCLUDED.min_lon, max_lat = EXCLUDED.max_lat, max_lon = EXCLUDED.max_lon
                    ",
                    &mac, b.min_lat, b.min_lon, b.max_lat, b.max_lon
                )
                .execute(&mut *tx)
                .await?;
            }
            Transmitter::Bluetooth { mac } => {
                query!(
                    "insert into bluetooth (mac, min_lat, min_lon, max_lat, max_lon) values ($1, $2, $3, $4, $5)
                     on conflict (mac) do update set min_lat = EXCLUDED.min_lat, min_lon = EXCLUDED.min_lon, max_lat = EXCLUDED.max_lat, max_lon = EXCLUDED.max_lon
                    ",
                    &mac, b.min_lat, b.min_lon, b.max_lat, b.max_lon
                )
                .execute(&mut *tx)
                .await?;
            }
        }
    }
    tx.commit().await?;

    if let Some(path) = stats_path {
        let stats = Stats {
            total_wifi: query_scalar!("select count(*) from wifi")
                .fetch_one(&pool)
                .await?
                .unwrap_or_default(),
            total_cell: query_scalar!("select count(*) from cell")
                .fetch_one(&pool)
                .await?
                .unwrap_or_default(),
            total_bluetooth: query_scalar!("select count(*) from bluetooth")
                .fetch_one(&pool)
                .await?
                .unwrap_or_default(),
            total_countries: query_scalar!("select count(distinct country) from cell")
                .fetch_one(&pool)
                .await?
                .unwrap_or_default(),
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
