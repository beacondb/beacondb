use actix_web::{
    error::ErrorInternalServerError, http::StatusCode, post, web, HttpResponse, Responder,
};
use mac_address::MacAddress;
use serde::Deserialize;
use sqlx::PgPool;

use crate::observation::{Beacon, Locality, Observation, ObservationHelper};

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
    wifi_access_points: Vec<Wifi>,
    #[serde(default)]
    bluetooth_beacons: Vec<Bluetooth>,
}

#[derive(Deserialize)]
struct Position {
    latitude: f32,
    longitude: f32,
}

#[derive(Deserialize)]
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
    for report in data.items {
        let mut helper = ObservationHelper::new();
        let date = (report.timestamp / 1000 / 86400) as i32;
        let locality = Locality::new(report.position.latitude, report.position.longitude);

        for ap in report.wifi_access_points {
            helper.add(
                Observation {
                    beacon: Beacon::Wifi {
                        bssid: ap.mac_address,
                        ssid: ap.ssid.map(|x| x.replace('\0', "")).unwrap_or_default(),
                    },
                    locality,
                },
                date,
            );
        }
        for bt in report.bluetooth_beacons {
            helper.add(
                Observation {
                    beacon: Beacon::Bluetooth {
                        mac: bt.mac_address,
                        name: bt.name.unwrap_or_default(),
                    },
                    locality,
                },
                date,
            );
        }
        helper
            .commit(&*pool)
            .await
            .map_err(ErrorInternalServerError)?;
    }

    Ok(HttpResponse::new(StatusCode::OK))
}
