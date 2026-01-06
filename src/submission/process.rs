//! This module contains functions to process new submissions.
//!
//! `beacondb` estimate the position of the beacon using a weighted average
//! algorithm.
//! It also keeps track of the bounding box of the positions where a beacon has
//! been reported, to detect moving beacons and for cell locations, for which
//! the weighted average algorithm is less adapted.
//!
//! `beacondb` iterates over all beacons in the reports and checks if it has
//! been reported before.
//! If it can find the beacon in the database it increases the bounding box to
//! include the new reported position if needed, and add the new data to the
//! database by computing he submission weight and incorporating it to the
//! average.
//! Otherwise `beacondb` creates a new entry in the database with a zero-sized
//! bounding box around the reported position.
//!
//! Some dead reckoning is done to determine the real beacon location, which can
//! be different from the report location as the GNSS fix on the contributor
//! device isn't synced to the scans.
//!
//! The weight using to compute the average is based on the RSSI (signal level),
//! distance between scan and GNSS fix and GNSS fix accuracy, based on
//! exponential functions in the form 10^(-data/coefficient), with higher values
//! of data being better. The coefficient is used to adjust the behaviour of the
//! curve to match the input data.
//!
//! After processing the data the stats are updated.

use std::{
    collections::{BTreeMap, BTreeSet},
    fs,
};

use anyhow::Result;
use geo::{Destination, Point, Rhumb};
use h3o::LatLng;
use h3o::Resolution;
use serde::Serialize;
use sqlx::{PgPool, query, query_scalar};

use crate::{bounds::TransmitterLocation, config::Config, model::Transmitter};

// 2 for outside, 3-5 inside based on https://codeberg.org/beacondb/beacondb/issues/31#issuecomment-3098830
// const SIGNAL_DROP_COEFFICIENT: f64 = 5.0;
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
        let mut modified: BTreeMap<Transmitter, TransmitterLocation> = BTreeMap::new();
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

            let loaded_report = match super::report::load(&report.raw) {
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

            let (pos, txs) = match loaded_report {
                Some(x) => x,
                None => continue, // report was ignored
            };

            for x in txs {
                // If we can't get the signal strength, assume a low value
                // to prevent accuracy from being overestimated.
                // It also implies lower weight, so it can quickly be
                // improved by other reports with more data
                let rssi = x.signal_strength().unwrap_or(-90);

                let distance_since_scan;
                let lat;
                let lon;
                if let Some(speed) = pos.speed
                    && let Some(wifi_age) = x.age()
                    && let Some(pos_age) = pos.age
                {
                    distance_since_scan = speed * (wifi_age as f64 - pos_age as f64) / 1000.0;

                    // "Reversed dead reckoning": guess where the transmitter was
                    // scanned based on heading and distance since last scan
                    // Neostumbler reduced metadata feature impact this feature
                    // as speed is rounded to 2 m/s and heading to 30° (which
                    // means +/-15° of error, with +/- 7.5° on average)
                    // Here are values for a 80 km/h speed with 1 second age
                    // difference
                    // cos(15°) * 22.22 m = 5.75 m error at most
                    // cos(7.5°) * 22.22 m = 2.90 m on average
                    // This algorithm is still useful with this error as without
                    // it, the data point would be located even further away
                    // (22.22 m in the given example)
                    if let Some(heading) = pos.heading {
                        let transmitter_scan_pos = Rhumb.destination(
                            Point::new(pos.latitude, pos.longitude),
                            heading,
                            -distance_since_scan,
                        );
                        (lat, lon) = transmitter_scan_pos.x_y();
                    } else {
                        lat = pos.latitude;
                        lon = pos.longitude;
                    }
                } else {
                    distance_since_scan = 0.0;
                    lat = pos.latitude;
                    lon = pos.longitude;
                };

                // Based on https://codeberg.org/beacondb/beacondb/issues/31#issuecomment-3098830
                let distance_from_transmitter =
                    10_f64.powf((BASE_RSSI - rssi as f64) / (10.0 * SIGNAL_DROP_COEFFICIENT));

                let signal_weight = 10_f64.powf(rssi as f64 / (10.0 * SIGNAL_DROP_COEFFICIENT));
                // The formula for age was found by quick trial and error. This
                // one seems fine. Let's take an average of 1 second between
                // wifi and pos age.
                // 1 m/s (3.6 km/h, by foot) = 0.91
                // 8.33 m/s (30 km/h, slow car zone in France) = 0.46
                // 13.88 m/s (50 km/h, fast car speed in city) = 0.28
                // 22.22 m/s (80 km/h, rural car speed) = 0.13
                // 30.55 m/s (110 km/h, fast car road) = 0.06
                // 36.11 m/s (130 km/h, fastest car roads) = 0.04
                // When no data is available, this will be computed as if the
                // report was done without moving (giving it an higher than
                // average weight).
                let age_weight = 10_f64.powf(-distance_since_scan.abs() / 25.0);

                // Same, found through trial and error
                // 1m = 0.79
                // 5m = 0.31
                // 10m = 0.1
                // 20m = 0.01
                let gnss_accuracy_weight = 10_f64.powf(-pos.accuracy.unwrap_or(10.0) / 10.0);
                let weight = signal_weight * age_weight * gnss_accuracy_weight;

                let accuracy = distance_from_transmitter + pos.accuracy.unwrap_or_default();

                if let Some(b) = modified.get_mut(&x) {
                    b.update(lat, lon, accuracy, weight);
                } else if let Some(b) = x.lookup(&pool).await? {
                    modified.insert(x, b.update(lat, lon, accuracy, weight));
                } else {
                    modified.insert(x, TransmitterLocation::new(lat, lon, accuracy, weight));
                }
            }

            let pos = LatLng::new(pos.latitude, pos.longitude)?;
            let h3 = pos.to_cell(Resolution::try_from(config.h3_resolution)?);
            h3s.insert(h3);
        }

        let modified_count = modified.len();

        for (x, b) in modified {
            match x {
                Transmitter::Cell {
                    radio,
                    country,
                    network,
                    area,
                    cell,
                    unit,
                    signal_strength: _,
                    age: _,
                } => {
                    query!(
                        "insert into cell (radio, country, network, area, cell, unit, min_lat, min_lon, max_lat, max_lon, lat, lon, accuracy, total_weight) values ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13, $14)
                         on conflict (radio, country, network, area, cell, unit) do update set min_lat = EXCLUDED.min_lat, min_lon = EXCLUDED.min_lon, max_lat = EXCLUDED.max_lat, max_lon = EXCLUDED.max_lon,
                         lat = EXCLUDED.lat, lon = EXCLUDED.lon, accuracy = EXCLUDED.accuracy, total_weight = EXCLUDED.total_weight
                        ",
                    radio as i16, country, network, area, cell, unit, b.min_lat, b.min_lon, b.max_lat, b.max_lon, b.lat, b.lon, b.accuracy, b.total_weight
                )
                .execute(&mut *tx)
                .await?;
                }
                Transmitter::Wifi {
                    mac,
                    signal_strength: _,
                    age: _,
                } => {
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
                Transmitter::Bluetooth {
                    mac,
                    signal_strength: _,
                    age: _,
                } => {
                    query!(
                        "insert into bluetooth (mac, min_lat, min_lon, max_lat, max_lon, lat, lon, accuracy, total_weight) values ($1, $2, $3, $4, $5, $6, $7, $8, $9)
                         on conflict (mac) do update set min_lat = EXCLUDED.min_lat, min_lon = EXCLUDED.min_lon, max_lat = EXCLUDED.max_lat, max_lon = EXCLUDED.max_lon,
                        lat = EXCLUDED.lat, lon = EXCLUDED.lon, accuracy = EXCLUDED.accuracy, total_weight = EXCLUDED.total_weight
                        ",
                    &mac, b.min_lat, b.min_lon, b.max_lat, b.max_lon, b.lat, b.lon, b.accuracy, b.total_weight
                )
                .execute(&mut *tx)
                .await?;
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
        eprintln!(
            "processed reports up to #{last_report_in_batch} - {modified_count} transmitters modified"
        );
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
