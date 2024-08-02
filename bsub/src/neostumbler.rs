use std::path::Path;

use anyhow::Result;
use bcap::{
    observation::{Observation, Position, WiFiObservation},
    utils::normalize_ssid,
};
use mac6::Mac;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct Record {
    // timestamp: u64,
    latitude: f64,
    longitude: f64,
    location_accuracy: f64,
    altitude: f64,
    altitude_accuracy: f64,
    // location_age: u64,
    speed: f64,
    mac_address: String,
    signal_strength: i8,
    ssid: Option<String>,
    // wifi_scan_age: u64,
}

pub fn parse(path: &Path) -> Result<Vec<Observation>> {
    let mut reader = csv::Reader::from_path(path)?;

    let mut observations = Vec::new();
    for result in reader.deserialize() {
        let Record {
            latitude,
            longitude,
            location_accuracy,
            altitude,
            altitude_accuracy,
            speed,
            mac_address,
            signal_strength,
            ssid,
        } = result?;

        let mac: Mac = mac_address.parse()?;
        let ssid = normalize_ssid(ssid.as_deref());

        if let Some(ssid) = ssid {
            let position = Position {
                latitude,
                longitude,
                accuracy: Some(location_accuracy),
                altitude: Some(altitude),
                altitude_accuracy: Some(altitude_accuracy),
                speed: Some(speed),
            };

            observations
                .push(WiFiObservation::new(position, mac.0, ssid, Some(signal_strength)).into())
        }
    }

    Ok(observations)
}
