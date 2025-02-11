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

pub fn extract(raw: &[u8]) -> Result<(Position, Vec<Transmitter>)> {
    let parsed: Report = serde_json::from_slice(raw)?;

    let mut txs = Vec::new();
    for cell in parsed.cell_towers.unwrap_or_default() {
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
        txs.push(Transmitter::Bluetooth {
            mac: bt.mac_address,
        })
    }

    Ok((parsed.position, txs))
}
