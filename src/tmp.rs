use anyhow::Result;
use serde::{Deserialize, Serialize};
use sqlx::{
    query,
    types::chrono::{DateTime, Utc},
    MySqlPool,
};

#[derive(Deserialize)]
struct Record {
    id: u32,
    // submitted_at: DateTime<Utc>,
    user_agent: String,
    raw: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    timestamp: DateTime<Utc>,
    position: Position,
    #[serde(default)]
    cell_towers: Vec<Cell>,
    #[serde(default)]
    wifi_access_points: Vec<Wifi>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Position {
    latitude: f32,
    longitude: f32,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wifi {
    ssid: String,
}

#[derive(Debug, Deserialize, Serialize, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "camelCase")]
struct Cell {
    radio_type: RadioType,
    mobile_country_code: u16,
    mobile_network_code: u16,
    #[serde(default)]
    location_area_code: u32,
    #[serde(default)]
    cell_id: u64,
    primary_scrambling_code: Option<u16>,
    signal_strength: Option<i32>,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(rename_all = "lowercase")]
enum RadioType {
    Gsm,
    #[serde(rename = "wcdma")]
    Umts,
    Lte,
}

pub async fn main(pool: MySqlPool) -> Result<()> {
    let mut csv = csv::Reader::from_path("geosubmission.csv")?;
    let mut tx = pool.begin().await?;
    for result in csv.deserialize() {
        let record: Record = result?;
        println!("{}", record.id);
        let report: Report = serde_json::from_str(&record.raw)?;

        query!("insert ignore into submission (id, submitted_at, timestamp, latitude, longitude, user_agent, raw) values (?,?,?,?,?,?,?)",
            record.id,
            // record.submitted_at,
            report.timestamp,
            report.timestamp,
            report.position.latitude,
            report.position.longitude,
            record.user_agent,
            record.raw
        ).execute(&mut *tx).await?;
    }
    tx.commit().await?;

    Ok(())
}
