use std::collections::BTreeMap;

use anyhow::{Context, Result};
use sqlx::{query, query_scalar, MySqlPool};

use crate::{bounds::Bounds, model::Transmitter};

pub async fn run(pool: MySqlPool) -> Result<()> {
    let count =
        query_scalar!("select count(*) as count from submission where processed_at is null")
            .fetch_one(&pool)
            .await?;

    if count == 0 {
        println!("Nothing to process");
        return Ok(());
    }
    println!("{count} submissions need processing");

    let mut modified: BTreeMap<Transmitter, Bounds> = BTreeMap::new();

    let reports = query!("select id, raw from submission order by id")
        .fetch_all(&pool)
        .await?;

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

    Ok(())
}
