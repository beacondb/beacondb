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
    #[serde(default)]
    cell_towers: Vec<Cell>,
    #[serde(default)]
    wifi_access_points: Vec<Wifi>,
    #[serde(default)]
    bluetooth_beacons: Vec<Bluetooth>,
}

#[derive(Deserialize)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: f64,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Cell {
    radio_type: RadioType,
    mobile_country_code: u16,
    mobile_network_code: u16,
    #[serde(default)]
    location_area_code: u32,
    #[serde(default)]
    cell_id: u64,
    #[serde(default)]
    primary_scrambling_code: u16,
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
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bluetooth {
    mac_address: MacAddress,
}

pub fn extract(raw: &str) -> Result<(Position, Vec<Transmitter>)> {
    let parsed: Report = serde_json::from_str(&raw)?;

    let mut txs = Vec::new();
    for cell in parsed.cell_towers {
        if cell.mobile_country_code == 0
                // || cell.mobile_network_code == 0 // this is valid
                || cell.location_area_code == 0
                || cell.cell_id == 0
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
            country: cell.mobile_country_code,
            network: cell.mobile_network_code,
            area: cell.location_area_code,
            cell: cell.cell_id,
            unit: cell.primary_scrambling_code,
        })
    }
    for wifi in parsed.wifi_access_points {
        // ignore hidden networks
        let ssid = wifi
            .ssid
            .map(|x| x.replace('\0', ""))
            .filter(|x| !x.is_empty());
        if ssid.is_some_and(|x| !x.contains("_nomap") && !x.contains("_output")) {
            txs.push(Transmitter::Wifi {
                mac: wifi.mac_address.bytes(),
            });
        }
    }
    for bt in parsed.bluetooth_beacons {
        txs.push(Transmitter::Bluetooth {
            mac: bt.mac_address.bytes(),
        })
    }

    Ok((parsed.position, txs))
}
