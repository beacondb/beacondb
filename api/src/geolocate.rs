use actix_web::{error::ErrorInternalServerError, post, web, HttpResponse};
use beacondb::KnownBeacon;
use geo::Point;
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use sqlx::{query, SqlitePool};

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
    radio_type: RadioType,
    mobile_country_code: i16,
    mobile_network_code: i16,
    location_area_code: i32,
    cell_id: i32,
    #[serde(default)]
    psc: i16,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "lowercase")]
enum RadioType {
    Gsm,
    Wcdma, // (umts)
    Lte,
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
#[derive(Debug, Serialize)]
struct Location {
    lat: f64,
    lng: f64,
}

#[post("/v1/geolocate")]
pub async fn service(
    data: web::Json<LocationRequest>,
    pool: web::Data<SqlitePool>,
) -> actix_web::Result<HttpResponse> {
    let data = data.into_inner();
    let pool = pool.into_inner();

    let mut points = Vec::new();
    for ap in data.wifi_access_points {
        let beacon = KnownBeacon::new(ap.mac_address.bytes());
        let key = beacon.key();
        let w = query!("select x,y,r from wifi where key = $1", key)
            .fetch_all(&*pool)
            .await
            .map_err(ErrorInternalServerError)?;
        for w in w {
            let (x, y) = beacon.remove_offset(Point::new(w.x, w.y)).x_y();
            if w.r > 1.0 {
                println!("{x},{y},{},{}", w.r, ap.mac_address);
                points.push((x, y, w.r));
            }
        }
    }

    if !points.is_empty() {
        // pretty basic algorithm - average access point location weighted by observed access point range
        // TODO: this doesn't work at all unless you get only unique keys by chance
        let mut lng = 0.0;
        let mut lat = 0.0;
        let mut accuracy = 0.0;
        let mut weights = 0.0;
        for (x, y, r) in points {
            let weight = 1.0 / r;
            lng += x * weight;
            lat += y * weight;
            accuracy += r * weight;
            weights += weight;
        }
        lng /= weights;
        lat /= weights;
        accuracy /= weights;

        let resp = LocationResponse {
            location: Location { lat, lng },
            accuracy,
        };
        println!("https://openstreetmap.org/search?query={lat}%2C{lng}");
        dbg!(&resp);
        return Ok(HttpResponse::Ok().json(resp));
    }

    for x in data.cell_towers {
        let radio = match x.radio_type {
            RadioType::Gsm => 0,
            RadioType::Wcdma => 1,
            RadioType::Lte => 2,
        };
        let row = query!("select x, y, r from cell where radio = ?1 and country = ?2 and network = ?3 and area = ?4 and cell = ?5 and unit = ?6",
            radio, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id, x.psc
        ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
        if let Some(row) = row {
            return Ok(HttpResponse::Ok().json(LocationResponse {
                location: Location {
                    lat: row.y,
                    lng: row.x,
                },
                accuracy: row.r,
            }));
        }

        // fallback to MLS if beaconDB does not know of this cell tower
        let row = query!("select x, y, r from cell_mls where radio = ?1 and country = ?2 and network = ?3 and area = ?4 and cell = ?5 and unit = ?6",
            radio, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id, x.psc
        ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
        if let Some(row) = row {
            return Ok(HttpResponse::Ok().json(LocationResponse {
                location: Location {
                    lat: row.y,
                    lng: row.x,
                },
                accuracy: row.r,
            }));
        }
    }

    Ok(HttpResponse::NotFound().finish())
}