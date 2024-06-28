use actix_web::{error::ErrorInternalServerError, post, web, HttpResponse};
use geo::HaversineDistance;
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use sqlx::{query, query_as, MySqlPool};

use crate::{bounds::Bounds, model::CellRadio};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocationRequest {
    #[serde(default)]
    cell_towers: Vec<CellTower>,
    #[serde(default)]
    wifi_access_points: Vec<AccessPoint>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct CellTower {
    radio_type: CellRadio,
    mobile_country_code: i16,
    mobile_network_code: i16,
    location_area_code: i32,
    cell_id: i32,
    #[serde(default)]
    psc: i16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AccessPoint {
    mac_address: MacAddress,
}

#[derive(Debug, Serialize)]
struct LocationResponse {
    location: Location,
    accuracy: f64,
}

impl LocationResponse {
    fn new(lat: f64, lon: f64, acc: f64) -> Self {
        LocationResponse {
            location: Location { lat, lng: lon },
            accuracy: acc,
        }
    }
}

impl From<Bounds> for LocationResponse {
    fn from(value: Bounds) -> Self {
        let (min, max) = value.points();
        let center = (min + max) / 2.0;
        let acc = min.haversine_distance(&center);
        let (lon, lat) = center.x_y();
        Self::new(lat, lon, acc)
    }
}

#[derive(Debug, Serialize)]
struct Location {
    lat: f64,
    lng: f64,
}

#[post("/v1/geolocate")]
pub async fn service(
    data: web::Json<LocationRequest>,
    pool: web::Data<MySqlPool>,
) -> actix_web::Result<HttpResponse> {
    let data = data.into_inner();
    let pool = pool.into_inner();

    for x in data.cell_towers {
        let row = query_as!(Bounds,"select min_lat, min_lon, max_lat, max_lon from cell where radio = ? and country = ? and network = ? and area = ? and cell = ? and unit = ?",
            x.radio_type, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id, x.psc
        ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
        if let Some(row) = row {
            return Ok(HttpResponse::Ok().json(Into::<LocationResponse>::into(row)));
        }

        // fallback to MLS if beaconDB does not know of this cell tower
        let row = query!("select lat, lon, radius from mls_cell where radio = ? and country = ? and network = ? and area = ? and cell = ? and unit = ?",
            x.radio_type, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id, x.psc
        ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
        if let Some(row) = row {
            return Ok(HttpResponse::Ok().json(LocationResponse::new(row.lat, row.lon, row.radius)));
        }
    }

    Ok(HttpResponse::NotFound().finish())
}
