use anyhow::Result;
use mac_address::MacAddress;
use serde::Deserialize;

use crate::model::{CellRadio, Transmitter};

// TODO: use the age value?
// location interpolation should be client side imo, but that would require a
// new submission format

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    timestamp: u64,
    position: Position,
    cell_towers: Option<Vec<Cell>>,
    wifi_access_points: Option<Vec<Wifi>>,
    bluetooth_beacons: Option<Vec<Bluetooth>>,
}

#[derive(Deserialize)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub speed: Option<f32>,
    #[serde(default)]
    pub age: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cell {
    radio_type: RadioType,
    mobile_country_code: u16,
    mobile_network_code: u16,
    // NeoStumbler/18 send {"locationAreaCode":null}
    #[serde(default)]
    location_area_code: Option<u32>, // u24 in db
    // NeoStumbler/18 send {"cellId":null}
    #[serde(default)]
    cell_id: Option<u64>,
    // NeoStumbler/18 send {"primaryScramblingCode":null}
    #[serde(default)]
    primary_scrambling_code: Option<u16>,
    #[serde(default)]
    age: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RadioType {
    Gsm,
    #[serde(rename = "wcdma")]
    Umts,
    Lte,
    Nr,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wifi {
    mac_address: MacAddress,
    ssid: Option<String>,
    #[serde(default)]
    age: Option<u32>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bluetooth {
    mac_address: MacAddress,
    #[serde(default)]
    age: Option<u32>,
}

fn is_valid_transmitter_age(position: &Position, transmitter_age: Option<u32>) -> bool {
    if let Some(transmitter_age) = transmitter_age {
        if let Some(position_age) = position.age {
            let position_transmitter_diff_age: u32 = position_age.abs_diff(transmitter_age);
            // trasmitter is observed more than 30 seconds from position
            if position_transmitter_diff_age > 30_000 {
                return false;
            }
            // transmitter is observed more than 50m away from position
            return position.speed.unwrap_or(0.0) * (position_transmitter_diff_age as f32)
                < 50_000.0;
        }
    }

    // the age field is optional, so for now observations without an age are still considered valid.
    // ideally with a future weighted algorithm observations with no age field have little weight / high uncertainty
    true
}

pub fn extract(raw: &[u8]) -> Result<(Position, Vec<Transmitter>)> {
    let parsed: Report = serde_json::from_slice(raw)?;

    let mut txs = Vec::new();
    for cell in parsed.cell_towers.unwrap_or_default() {
        if !is_valid_transmitter_age(&parsed.position, cell.age) {
            continue;
        }
        if cell.mobile_country_code == 0
                // || cell.mobile_network_code == 0 // this is valid
                || cell.location_area_code.unwrap_or(0) == 0
                || cell.cell_id.unwrap_or(0) == 0
                || cell.primary_scrambling_code.is_none()
        {
            // TODO: reuse previous cell tower data
            continue;
        }

        txs.push(Transmitter::Cell {
            radio: match cell.radio_type {
                RadioType::Gsm => CellRadio::Gsm,
                RadioType::Umts => CellRadio::Wcdma,
                RadioType::Lte => CellRadio::Lte,
                RadioType::Nr => CellRadio::Nr,
            },
            // postgres uses signed integers
            country: cell.mobile_country_code as i16,
            network: cell.mobile_network_code as i16,
            area: cell.location_area_code.unwrap() as i32,
            cell: cell.cell_id.unwrap() as i64,
            unit: cell.primary_scrambling_code.unwrap() as i16,
        })
    }
    for wifi in parsed.wifi_access_points.unwrap_or_default() {
        if !is_valid_transmitter_age(&parsed.position, wifi.age) {
            continue;
        }
        // ignore hidden networks
        let ssid = wifi
            .ssid
            .map(|x| x.replace('\0', ""))
            .filter(|x| !x.is_empty());
        if ssid.is_some_and(|x| !x.contains("_nomap") && !x.contains("_optout")) {
            txs.push(Transmitter::Wifi {
                mac: wifi.mac_address,
            });
        }
    }
    for bt in parsed.bluetooth_beacons.unwrap_or_default() {
        if !is_valid_transmitter_age(&parsed.position, bt.age) {
            continue;
        }
        txs.push(Transmitter::Bluetooth {
            mac: bt.mac_address,
        })
    }

    Ok((parsed.position, txs))
}
