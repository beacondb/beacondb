use actix_web::{error::ErrorInternalServerError, post, web, HttpResponse};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use sqlx::{query, PgPool};

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LocationRequest {
    cell_towers: Vec<CellTower>,
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
    pool: web::Data<PgPool>,
) -> actix_web::Result<HttpResponse> {
    let data = data.into_inner();
    let pool = pool.into_inner();

    for x in data.cell_towers {
        let radio = match x.radio_type {
            RadioType::Gsm => 0,
            RadioType::Wcdma => 1,
            RadioType::Lte => 2,
        };
        dbg!(&x);
        let row = query!("select x, y, r from cell where radio = $1 and country = $2 and network = $3 and area = $4 and cell = $5 and unit = $6",
            radio, x.mobile_country_code, x.mobile_network_code, x.location_area_code, x.cell_id, x.psc
        ).fetch_optional(&*pool).await.map_err(ErrorInternalServerError)?;
        if let Some(row) = row {
            dbg!(&row);
            return Ok(HttpResponse::Ok().json(LocationResponse {
                location: Location {
                    lat: row.y,
                    lng: row.x,
                },
                accuracy: row.r,
            }));
        }
    }

    // TODO: come up with a useful estimation algorithm
    // let mut count = 0;
    // let mut xs = 0.0;
    // let mut ys = 0.0;
    // for x in data.wifi_access_points {
    //     let w = query!("select x,y,r from wifi where bssid = $1", x.mac_address)
    //         .fetch_optional(&*pool)
    //         .await
    //         .map_err(ErrorInternalServerError)?;
    //     if let Some(w) = w {
    //         println!("{} {} {}", x.mac_address, w.x, w.y);
    //         count += 1;
    //         xs += w.x;
    //         ys += w.y;
    //     }
    // }

    // if count == 0 {
    return Ok(HttpResponse::NotFound().into());
    // } else {
    //     let lng = xs / count as f64;
    //     let lat = ys / count as f64;
    //     println!("https://openstreetmap.org/search?query={lat}%2C{lng}");
    //     Ok(HttpResponse::Ok().json(LocationResponse {
    //         location: Location { lat, lng },
    //         accuracy: 12.3,
    //     }))
    // }
}
