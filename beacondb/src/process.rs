use std::collections::BTreeMap;

use anyhow::{Context, Result};
use mac_address::MacAddress;
use rusqlite::{Connection, OptionalExtension};
use serde::Deserialize;
use sqlx::{query, PgPool};

use crate::bounds::Bounds;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    timestamp: u64,
    position: Position,
    #[serde(default)]
    cell_towers: Vec<Cell>,
    #[serde(default)]
    wifi_access_points: Vec<Wifi>,
}

#[derive(Deserialize)]
struct Position {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
struct Cell {
    radio_type: RadioType,
    mobile_country_code: u16,
    mobile_network_code: u16,
    #[serde(default)]
    location_area_code: u32,
    cell_id: u64,
    primary_scrambling_code: Option<u16>,
}

#[derive(Debug, Deserialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
enum RadioType {
    Gsm,
    #[serde(rename = "wcdma")]
    Umts,
    Lte,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
struct Wifi {
    mac_address: MacAddress,
    ssid: Option<String>,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
enum Beacon {
    Cell {
        radio: RadioType,
        country: u16,
        network: u16,
        area: u32,
        cell: u64,
        unit: u16,
    },
    Wifi {
        bssid: MacAddress,
        ssid: String,
    },
}

pub async fn main(pool: PgPool, conn: &mut Connection) -> Result<()> {
    // TODO: probably dont fetch every single one at once
    let batch = query!("select id, raw from geosubmission where status = 1")
        .fetch_all(&pool)
        .await?;
    let mut tx = pool.begin().await?;
    let mut bounds: BTreeMap<Beacon, Bounds> = BTreeMap::new();
    for report in batch {
        query!(
            "update geosubmission set status = 0 where id = $1",
            report.id
        )
        .execute(&mut *tx)
        .await?;

        let parsed: Report = serde_json::from_str(&report.raw)
            .with_context(|| format!("parsing: {}", report.raw))?;
        let (x, y) = (parsed.position.longitude, parsed.position.latitude);

        let mut beacons = Vec::new();
        for cell in parsed.cell_towers {
            if cell.mobile_country_code == 0
                || cell.mobile_network_code == 0
                || cell.location_area_code == 0
            {
                // TODO: reuse previous cell tower data
                continue;
            }

            beacons.push(Beacon::Cell {
                radio: cell.radio_type,
                country: cell.mobile_country_code,
                network: cell.mobile_network_code,
                area: cell.location_area_code,
                cell: cell.cell_id,
                unit: cell.primary_scrambling_code.unwrap_or(0),
            })
        }
        for wifi in parsed.wifi_access_points {
            let ssid = wifi
                .ssid
                .map(|x| x.replace('\0', ""))
                .filter(|x| !x.is_empty());
            // ignoring hidden networks for now
            if let Some(ssid) = ssid {
                beacons.push(Beacon::Wifi {
                    bssid: wifi.mac_address,
                    ssid,
                });
            }
        }

        for k in beacons {
            if let Some(v) = bounds.get_mut(&k) {
                *v = *v + (x, y);
            } else {
                bounds.insert(k, Bounds::new(x, y, 0.0));
            }
        }
    }

    let lite_tx = conn.transaction()?;
    for (k, v) in bounds {
        match k {
            Beacon::Cell {
                radio,
                country,
                network,
                area,
                cell,
                unit,
            } => {
                let existing = lite_tx.query_row(
                    "select x, y, r from cell where radio = ?1 and country = ?2 and network = ?3 and area = ?4 and cell = ?5 and unit = ?6",
                    (radio as u8, country, network, area, cell, unit),
                    |row| {
                        Ok(Bounds::new(row.get(0)?, row.get(1)? , row.get(2)? ))
                    }
                ).optional()?;

                if let Some(existing) = existing {
                    let bounds = existing + v;
                    let (x, y, r) = bounds.x_y_r();
                    lite_tx.execute(
                        "update cell set x = ?1, y = ?2, r = ?3 where radio = ?4 and country = ?5 and network = ?6 and area = ?7 and cell = ?8 and unit = ?9",
                        ( x, y, r, radio as u8, country, network, area, cell, unit)
                    )?;
                } else {
                    let (x, y, r) = v.x_y_r();
                    lite_tx.execute(
                        "insert into cell (radio, country, network, area, cell, unit, x, y, r) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)", 
                        (radio as u8, country, network, area, cell, unit, x, y, r)
                    )?;
                }
            }
            Beacon::Wifi { bssid, ssid: _ } => {
                let bssid = bssid.to_string().to_lowercase();
                let existing = lite_tx
                    .query_row(
                        "select x, y, r from wifi where bssid = ?1",
                        (&bssid,),
                        |row| Ok(Bounds::new(row.get(0)?, row.get(1)?, row.get(2)?)),
                    )
                    .optional()?;

                if let Some(existing) = existing {
                    let bounds = existing + v;
                    let (x, y, r) = bounds.x_y_r();
                    lite_tx.execute(
                        "update wifi set x = ?1, y = ?2, r = ?3 where bssid = ?4",
                        (x, y, r, bssid),
                    )?;
                } else {
                    let (x, y, r) = v.x_y_r();
                    lite_tx.execute(
                        "insert into wifi (bssid, x, y, r) values (?1, ?2, ?3, ?4)",
                        (bssid, x, y, r),
                    )?;
                }
            }
        }
    }

    lite_tx.commit()?;
    tx.commit().await?;

    Ok(())
}
