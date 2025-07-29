//! This module contains functions to process new submissions.
//!
//! Currently, `beacondb` does not predict the position of the beacon and
//! simply keeps track of the bounding box of the positions where a beacon has
//! been reported.
//!
//! `beacondb` iterates over all beacons in the reports and checks if it has
//! been reported before.
//! If it can find the beacon in the database it increases the bounding box to
//! include the new reported position if needed.
//! Otherwise `beacondb` creates a new entry in the database with a zero-sized
//! bounding box around the reported position.
//!
//! After processing the data the stats are updated.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use anyhow::Result;
use h3o::LatLng;
use h3o::Resolution;
use serde::Serialize;
use sqlx::{query, query_scalar, PgPool};

use crate::{bounds::Bounds, bounds::WeightedAverageBounds, config::Config, model::Transmitter};

// 2 for outside, 3-5 inside based on https://codeberg.org/beacondb/beacondb/issues/31#issuecomment-3098830
const SIGNAL_DROP_COEFFICIENT: f64 = 3.0;

// RSSI at 1m from AP, used to estimate accuracy
const BASE_RSSI: f64 = -30.0;

/// Process new submissions
pub async fn run(pool: PgPool, config: Config) -> Result<()> {
    loop {
        let mut tx = pool.begin().await?;
        let reports =
            query!("select id, raw, user_agent from report where processed_at is null order by id limit 10000")
                .fetch_all(&mut *tx)
                .await?;
        let mut modified: BTreeMap<Transmitter, Bounds> = BTreeMap::new();
        let mut wifi_modified: BTreeMap<Transmitter, WeightedAverageBounds> = BTreeMap::new();
        let mut h3s = BTreeSet::new();

        let last_report_in_batch = if let Some(report) = reports.last() {
            report.id
        } else {
            eprintln!("finished processing");
            break;
        };

        for report in reports {
            query!(
                "update report set processed_at = now() where id = $1",
                report.id
            )
            .execute(&mut *tx)
            .await?;

            let (pos, txs) = match super::report::extract(&report.raw) {
                Ok(x) => x,
                Err(e) => {
                    eprintln!(
                        "Failed to parse report #{} from '{}': {e}",
                        report.id,
                        report.user_agent.unwrap_or_default()
                    );
                    query!(
                        "update report set processing_error = $1 where id = $2",
                        format!("{e}"),
                        report.id
                    )
                    .execute(&mut *tx)
                    .await?;
                    continue;
                }
            };

            if txs.is_empty() {
                continue;
            };

            for x in txs {
                // Handle weighted average for Wifi only, for now
                if let Transmitter::Wifi { mac: _, signal_strength } = x {
                    let rssi = signal_strength.unwrap_or_default();

                    // Based on https://codeberg.org/beacondb/beacondb/issues/31#issuecomment-3098830
                    // TODO: Include age and accuracy/speed in weight
                    let weight = 10_f64.powf(rssi as f64 / (10.0 * SIGNAL_DROP_COEFFICIENT));
                    let distance = 10_f64.powf((BASE_RSSI - rssi as f64) / (10.0 * SIGNAL_DROP_COEFFICIENT));

                    if let Some(b) = wifi_modified.get_mut(&x) {
                        *b = *b + (pos.latitude, pos.longitude, distance, weight);
                    } else if let Some(b) = x.lookup_as_weighted_average(&pool).await? {
                        wifi_modified.insert(x, b + (pos.latitude, pos.longitude, distance, weight));
                    } else {
                        wifi_modified.insert(x, WeightedAverageBounds::new(pos.latitude, pos.longitude, distance, weight));
                    }
                } else {
                    if let Some(b) = modified.get_mut(&x) {
                        *b = *b + (pos.latitude, pos.longitude);
                    } else if let Some(b) = x.lookup(&pool).await? {
                        modified.insert(x, b + (pos.latitude, pos.longitude));
                    } else {
                        modified.insert(x, Bounds::new(pos.latitude, pos.longitude));
                    }
                }
            }

            let pos = LatLng::new(pos.latitude, pos.longitude)?;
            let h3 = pos.to_cell(Resolution::try_from(config.h3_resolution)?);
            h3s.insert(h3);
        }

        let modified_count = modified.len() + wifi_modified.len();
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
                Transmitter::Wifi { mac: _ , ..} => {
                    panic!("Bounding box used for Wi-Fi");
                //     query!(
                //         "insert into wifi (mac, min_lat, min_lon, max_lat, max_lon) values ($1, $2, $3, $4, $5)
                //          on conflict (mac) do update set min_lat = EXCLUDED.min_lat, min_lon = EXCLUDED.min_lon, max_lat = EXCLUDED.max_lat, max_lon = EXCLUDED.max_lon
                //         ",
                //     &mac, b.min_lat, b.min_lon, b.max_lat, b.max_lon
                // )
                // .execute(&mut *tx)
                // .await?;
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

        for (x, b) in wifi_modified {
            match x {
                Transmitter::Cell { .. } => {
                    todo!();
                }
                Transmitter::Wifi { mac, ..} => {
                    query!(
                        "insert into wifi (mac, min_lat, min_lon, max_lat, max_lon, lat, lon, accuracy, total_weight) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                         on conflict (mac) do update set min_lat = EXCLUDED.min_lat, min_lon = EXCLUDED.min_lon, max_lat = EXCLUDED.max_lat, max_lon = EXCLUDED.max_lon,
                         lat = EXCLUDED.lat, lon = EXCLUDED.lon, accuracy = EXCLUDED.accuracy, total_weight = EXCLUDED.total_weight
                        ",
                    &mac, b.min_lat, b.min_lon, b.max_lat, b.max_lon, b.lat, b.lon, b.accuracy, b.total_weight
                )
                .execute(&mut *tx)
                .await?;
                }
                Transmitter::Bluetooth { .. } => {
                    todo!();
                }
            }
        }


        for h3 in h3s {
            let h3_binary = u64::from(h3).to_be_bytes();
            query!(
                "insert into map (h3) values ($1) on conflict (h3) do nothing",
                &h3_binary
            )
            .execute(&mut *tx)
            .await?;
        }

        tx.commit().await?;
        eprintln!("processed reports up to #{last_report_in_batch} - {modified_count} transmitters modified");
    }

    if let Some(config_stats) = config.stats {
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
            total_reports: config_stats.archived_reports
                + query_scalar!("select count(*) from report")
                    .fetch_one(&pool)
                    .await?
                    .unwrap_or_default(),
        };

        let data = serde_json::to_string_pretty(&stats)?;
        fs::write(&config_stats.path, data)?;
    }

    Ok(())
}

/// Rust representation of database statistics
#[derive(Serialize)]
struct Stats {
    total_wifi: i64,
    total_cell: i64,
    total_bluetooth: i64,
    total_countries: i64,
    total_reports: i64,
}
