use std::{
    fs::File,
    io::{BufRead, BufReader},
    path::Path,
};

use anyhow::Result;
use bcap::{
    observation::{Observation, Position, WiFiObservation},
    utils::normalize_ssid,
};
use mac6::Mac;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
#[serde(rename_all = "PascalCase")]
struct Record {
    #[serde(rename = "MAC")]
    mac: String,
    #[serde(rename = "SSID")]
    ssid: Option<String>,
    #[serde(rename = "RSSI")]
    rssi: f32,
    current_latitude: f64,
    current_longitude: f64,
    altitude_meters: f64,
    accuracy_meters: f64,
    #[serde(rename = "Type")]
    type_: String,
}

pub fn parse(path: &Path) -> Result<Vec<Observation>> {
    let mut reader = BufReader::new(File::open(path)?);
    reader.read_line(&mut String::new())?; // skip header
    let mut reader = csv::ReaderBuilder::new().from_reader(reader);

    let mut observations = Vec::new();
    for result in reader.deserialize() {
        let Record {
            mac,
            ssid,
            rssi,
            current_latitude,
            current_longitude,
            altitude_meters,
            accuracy_meters,
            type_,
        } = result?;

        if type_ != "WIFI" {
            continue;
        }

        let mac: Mac = mac.parse()?;
        let ssid = normalize_ssid(ssid.as_deref());

        if let Some(ssid) = ssid {
            let position = Position {
                latitude: current_latitude,
                longitude: current_longitude,
                accuracy: Some(accuracy_meters),
                altitude: Some(altitude_meters),
                altitude_accuracy: None,
                speed: None,
            };

            observations.push(WiFiObservation::new(position, mac.0, ssid, Some(rssi as i8)).into());
        }
    }

    Ok(observations)
}
