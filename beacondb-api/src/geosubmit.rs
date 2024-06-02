use std::collections::BTreeMap;

use actix_web::{
    error::ErrorInternalServerError, http::StatusCode, post, web, HttpResponse, Responder,
};
use anyhow::Context;
use mac_address::MacAddress;
use serde::Deserialize;
use sqlx::{query, PgPool};

use crate::bounds::Bounds;

#[derive(Deserialize)]
struct Submission {
    items: Vec<Report>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    timestamp: u64,
    position: Position,
    #[serde(default)]
    cell_towers: Vec<CellTower>,
    #[serde(default)]
    wifi_access_points: Vec<Wifi>,
    #[serde(default)]
    bluetooth_beacons: Vec<Bluetooth>,
}

#[derive(Deserialize)]
struct Position {
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
struct CellTower {
    radio_type: RadioType,
    mobile_country_code: i16,
    mobile_network_code: i16,
    location_area_code: i32,
    cell_id: i32,
    #[serde(default)]
    primary_scrambling_code: i16,
}

#[derive(Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
enum RadioType {
    Gsm,
    Wcdma, // (umts)
    Lte,
}

#[derive(Deserialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
struct Wifi {
    mac_address: MacAddress,
    ssid: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bluetooth {
    mac_address: MacAddress,
    name: Option<String>,
}

#[post("/v2/geosubmit")]
pub async fn service(
    data: web::Json<Submission>,
    pool: web::Data<PgPool>,
) -> actix_web::Result<impl Responder> {
    let data = data.into_inner();
    let pool = pool.into_inner();

    let mut cells: BTreeMap<CellTower, Bounds> = BTreeMap::new();
    let mut wifis: BTreeMap<Wifi, Bounds> = BTreeMap::new();

    for report in data.items {
        let (x, y) = (report.position.longitude, report.position.latitude);
        for c in report.cell_towers {
            if let Some(b) = cells.get_mut(&c) {
                b.add(x, y);
            } else {
                cells.insert(c, Bounds::new(x, y, 0.0));
            }
        }
        for w in report.wifi_access_points {
            if let Some(b) = wifis.get_mut(&w) {
                b.add(x, y);
            } else {
                wifis.insert(w, Bounds::new(x, y, 0.0));
            }
        }
    }

    insert(&*pool, cells, wifis)
        .await
        .context("database insert failed")
        .map_err(ErrorInternalServerError)?;

    Ok(HttpResponse::new(StatusCode::OK))
}

async fn insert(
    pool: &PgPool,
    cells: BTreeMap<CellTower, Bounds>,
    wifis: BTreeMap<Wifi, Bounds>,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;
    for (cell, bounds) in cells {
        let (x, y, r) = bounds.x_y_r();
        let radio = match cell.radio_type {
            RadioType::Gsm => 0,
            RadioType::Wcdma => 1,
            RadioType::Lte => 2,
        };
        query!(
            "insert into cell_submission (
                radio, country, network, area, cell, unit, x, y, r
            ) values (
                $1, $2, $3, $4, $5, $6, $7, $8, $9
            )",
            radio,
            cell.mobile_country_code,
            cell.mobile_network_code,
            cell.location_area_code,
            cell.cell_id,
            cell.primary_scrambling_code,
            x,
            y,
            r
        )
        .execute(&mut *tx)
        .await?;
    }
    for (wifi, bounds) in wifis {
        let ssid = wifi
            .ssid
            .map(|x| x.replace('\0', ""))
            .filter(|x| !x.is_empty());

        let (x, y, r) = bounds.x_y_r();
        query!(
            "insert into wifi_submission (
                bssid, ssid, x, y, r
            ) values (
                $1, $2, $3, $4, $5
            )",
            wifi.mac_address,
            ssid,
            x,
            y,
            r
        )
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;

    Ok(())
}
