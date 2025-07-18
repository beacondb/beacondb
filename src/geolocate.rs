//! Contains the main geolocalization service.
//!
//! To geolocate a request `beacondb` first tries to locate based on the
//! surrounding WiFi networks.
//! A weight is determined by the WiFi signal strength reported by the client.
//! The center of the bounding boxes of the networks are queried and the
//! center position is averaged based on the weight.
//!
//! At least two WiFi networks have to been known to accurately determine the
//! position.
//! If this is not the case the position of the current cell tower is returned.
//!
//! If the cell tower is not known to `beacondb` the location is estimated
//! using the client's ip.
//!
//! WiFi networks are ignored if the bounding box if spans more less than 1m or
//! more than 500m to filter out moving access points.

use std::{collections::BTreeSet, str::FromStr};

use actix_web::{error::ErrorInternalServerError, post, web, HttpRequest, HttpResponse};
use anyhow::Context;
use geo::{Distance, Haversine};
use ipnetwork::IpNetwork;
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{query, query_as, query_file, PgPool};

use crate::{bounds::Bounds, model::CellRadio};

/// Serde representation of the client's request
#[derive(Debug, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
struct LocationRequest {
    /// List of cell towers around the client
    #[serde(default)]
    cell_towers: Vec<CellTower>,

    /// List of access points around the client
    #[serde(default)]
    wifi_access_points: Vec<AccessPoint>,

    /// Whether using the client's ip address to locate is allowed
    consider_ip: Option<bool>,
    fallbacks: Option<FallbackOptions>,
}

#[derive(Debug, Deserialize, Default)]
struct FallbackOptions {
    ipf: Option<bool>,
}

// Serde representation of cell towers in the client's request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CellTower {
    radio_type: CellRadio,
    mobile_country_code: i16,
    mobile_network_code: i16,
    location_area_code: i32,
    cell_id: i64,
    psc: Option<i16>,
}

// Serde representation of access points in the client's request
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccessPoint {
    mac_address: MacAddress,
    signal_strength: Option<i8>,
}

/// Struct for representing the server's response
#[derive(Debug, Serialize)]
struct LocationResponse {
    location: Location,
    accuracy: i64,
}

impl LocationResponse {
    /// Create a new location response from a position and an accuracy.
    fn new(lat: f64, lon: f64, acc: f64) -> Self {
        // round to 6 decimal places
        let lat = (lat * 1_000_000.0).round() / 1_000_000.0;
        let lon = (lon * 1_000_000.0).round() / 1_000_000.0;

        LocationResponse {
            location: Location { lat, lng: lon },
            accuracy: (acc.round() as i64).max(50),
        }
    }

    /// Convert the response into a HTTP response
    fn respond(self) -> actix_web::Result<HttpResponse> {
        if self.location.lat.is_nan() || self.location.lng.is_nan() {
            Ok(HttpResponse::InternalServerError().finish())
        } else {
            Ok(HttpResponse::Ok().json(self))
        }
    }
}

impl From<Bounds> for LocationResponse {
    fn from(value: Bounds) -> Self {
        let (min, max) = value.points();
        let center = (min + max) / 2.0;
        let acc = Haversine::distance(min, center);
        let (lon, lat) = center.x_y();
        Self::new(lat, lon, acc)
    }
}

/// Serde representation of a location
#[derive(Debug, Serialize)]
struct Location {
    lat: f64,
    lng: f64,
}

/// Main entrypoint to geolocate a client.
#[post("/v1/geolocate")]
pub async fn service(
    data: Option<web::Json<LocationRequest>>,
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let data = data.map(|x| x.into_inner()).unwrap_or_default();
    let pool = pool.into_inner();

    let mut latw = 0.0;
    let mut lonw = 0.0;
    let mut rw = 0.0;
    let mut ww = 0.0;
    let mut c = 0;
    let mut seen = BTreeSet::new();
    for x in data.wifi_access_points {
        if !seen.insert(x.mac_address) {
            continue;
        }

        let signal = match x.signal_strength.unwrap_or_default() {
            0 => -80,
            -50..=0 => -50,
            x if (-100..-50).contains(&x) => x,
            // ..-80 => -80,
            _ => continue,
        };
        let weight = ((1.0 / (signal as f64 - 20.0).powi(2)) * 10000.0).powi(2);

        let row = query_as!(
            Bounds,
            "select min_lat, min_lon, max_lat, max_lon from wifi where mac = $1",
            &x.mac_address
        )
        .fetch_optional(&*pool)
        .await
        .map_err(ErrorInternalServerError)?;
        if let Some(row) = row {
            let (min, max) = row.points();
            let center = (min + max) / 2.0;
            let r = Haversine::distance(min, center);
            let (lon, lat) = center.x_y();

            if (1.0..=500.0).contains(&r) {
                latw += lat * weight;
                lonw += lon * weight;
                rw += r * weight;
                ww += weight;
                c += 1;
            }
        }
    }
    if c >= 2 {
        latw /= ww;
        lonw /= ww;
        rw /= ww;

        if latw.is_nan() || lonw.is_nan() {
            dbg!(rw, ww);
        } else {
            return LocationResponse::new(latw, lonw, rw).respond();
        }
    }

    // todo: this is awful
    for x in data.cell_towers {
        if let Some(unit) = x.psc {
            let row = query_as!(Bounds,"select min_lat, min_lon, max_lat, max_lon from cell where radio = $1 and country = $2 and network = $3 and area = $4 and cell = $5 and unit = $6",
                x.radio_type as i16, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id, unit
            ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
            if let Some(row) = row {
                return LocationResponse::from(row).respond();
            }

            let row = query!("select lat, lon, radius from mls_cell where radio = $1 and country = $2 and network = $3 and area = $4 and cell = $5 and unit = $6",
                x.radio_type as i16, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id, unit
            ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
            if let Some(row) = row {
                return LocationResponse::new(row.lat, row.lon, row.radius).respond();
            }
        } else {
            let row = query_as!(Bounds,"select min_lat, min_lon, max_lat, max_lon from cell where radio = $1 and country = $2 and network = $3 and area = $4 and cell = $5",
                x.radio_type as i16, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id
            ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
            if let Some(row) = row {
                return LocationResponse::from(row).respond();
            }

            let row = query!("select lat, lon, radius from mls_cell where radio = $1 and country = $2 and network = $3 and area = $4 and cell = $5",
                x.radio_type as i16, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id
            ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
            if let Some(row) = row {
                return LocationResponse::new(row.lat, row.lon, row.radius).respond();
            }
        }
    }

    let consider_ip =
        data.consider_ip.unwrap_or(true) && data.fallbacks.unwrap_or_default().ipf.unwrap_or(true);
    if consider_ip {
        let ip = req
            .headers()
            .get("X-Forwarded-For")
            .and_then(|x| x.to_str().ok())
            .and_then(|x| IpNetwork::from_str(x).ok())
            .context("failed to get client ip address")
            .map_err(ErrorInternalServerError)?;
        if let Some(record) = query_file!("src/geoip/lookup.sql", ip)
            .fetch_optional(&*pool)
            .await
            .map_err(ErrorInternalServerError)?
        {
            return Ok(HttpResponse::Ok().json(json!({
                "license": crate::geoip::LICENSE,
                "location": {
                    "lat": record.latitude,
                    "lng": record.longitude,
                },
                "accuracy": 25_000,
                "fallback": "ipf"
            })));
        }
    }

    Ok(HttpResponse::NotFound().json(json!(
        {
            "error": {
                "errors": [{
                    "domain": "geolocation",
                    "reason": "notFound",
                    "message": "No location could be estimated based on the data provided",
                }],
                "code": 404,
                "message": "Not found",
            }
        }
    )))
}
