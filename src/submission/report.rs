//! Module to deserialize reports.

use anyhow::Result;
use mac_address::MacAddress;
use serde::Deserialize;

use crate::model::{CellRadio, Transmitter};

/// Serde representation to deserialize report
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    #[allow(dead_code)]
    timestamp: u64,
    position: Position,
    cell_towers: Option<Vec<Cell>>,
    wifi_access_points: Option<Vec<Wifi>>,
    bluetooth_beacons: Option<Vec<Bluetooth>>,
}

/// Serde representation to deserialize a position in a report
#[derive(Deserialize)]
pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    #[serde(default)]
    pub speed: Option<f64>,
    // Tower Collector does not send age field
    #[serde(default)]
    pub age: Option<u32>,
    pub accuracy: Option<f64>,
    pub heading: Option<f64>,
}

/// Serde representation to deserialize a cell tower in a report
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
    // Tower Collector does not send age field
    #[serde(default)]
    age: Option<u32>,

    // Signal can be between -44 dBm and -140 dBm according to https://android.stackexchange.com/questions/167650/acceptable-signal-strength-ranges-for-2g-3g-and-4g
    // so we need to store it on an i16 as i8 would overflow
    signal_strength: Option<i16>,

    // Arbitrary Strength Unit, which can be parsed into signal strength based
    // on the underlying network
    asu: Option<i16>,
}

/// Serde representation to deserialize a radio type
#[derive(Deserialize)]
#[serde(rename_all = "lowercase")]
enum RadioType {
    Gsm,
    #[serde(rename = "wcdma")]
    Umts,
    Lte,
    Nr,
}

/// Serde representation to deserialize a wifi network in a report
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Wifi {
    mac_address: MacAddress,
    ssid: Option<String>,
    #[serde(default)]
    age: Option<u32>,
    signal_strength: Option<i16>,
}

/// Serde representation to deserialize a bluetooth beacon in a report
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Bluetooth {
    mac_address: MacAddress,
    #[serde(default)]
    age: Option<u32>,
    signal_strength: Option<i16>,
}

fn should_be_ignored(position: &Position, transmitter_age: Option<u32>) -> bool {
    if let Some(transmitter_age) = transmitter_age {
        if let Some(position_age) = position.age {
            let position_transmitter_diff_age: u32 = position_age.abs_diff(transmitter_age);
            // trasmitter is observed more than 30 seconds from position
            // Since Neostumbler/18 (1.4.0), age is limited to 30 seconds, before it, the age is not limited
            if position_transmitter_diff_age > 30_000 {
                return true;
            }

            if position.speed.unwrap_or(0.0) * position_transmitter_diff_age as f64 > 150_000.0 {
                return true;
            }
        }
    }

    // the age field is optional, so for now observations without an age are still considered valid.
    // ideally with a future weighted algorithm observations with no age field have little weight / high uncertainty
    false
}

// Get the signal strength in dBm, extracting it from ASU if unavailable
fn signal_strength(cell: &Cell) -> Option<i16> {
    if let Some(_) = cell.signal_strength {
        return cell.signal_strength;
    }
    // If signal strength is not available, we need to extract it from the ASU
    // Info about this process: https://en.wikipedia.org/wiki/Mobile_phone_signal#ASU
    if let Some(asu) = cell.asu {
        // 99 means unknown
        if asu == 99 {
            return None;
        }

        return match cell.radio_type {
            // Seems to be fine (match what's given on my phone -83 dBm 15 ASU)
            RadioType::Gsm => Some((2 * asu) - 113),

            // // TODO: According to Wikipedia, Android use GMS formula for UMTS,
            // // we need to figure out the best way to extract in this case.
            // RadioType::Umts => Some(asu - 115),
            // Based on my testing on Pixel 6a GrapheneOS with Android 16, the
            // formula is a bit different, but seems to match what's shown in
            // Android settings (type *#*#4636#*#* in dialer then select Phone
            // Info to get more detailed settings, I can't force 2G or 3G using
            // normal settings).
            RadioType::Umts => Some(asu - 120),

            // Value is between asu-140 and asu-143, we just take the highest
            // value, as middle point would be a floating point
            RadioType::Lte => Some(asu - 140),

            // Formula for 5G is not available on Wikipedia, but this post seems
            // to say it's the same as LTE formula. I don't know if it's
            // trustworthy as the same post also says ASU is linear and dBm is
            // logarithmic, which is obviously wrong as the conversion is an
            // affine function, which can't cancel a logarithm.
            // https://www.linkedin.com/pulse/what-arbitrary-signal-unit-why-does-matter-telecom-hassan-bin-tila-oap2c
            // I didn't verify this formula, as I don't have access to 5G networks
            RadioType::Nr => Some(asu - 140),
        };
    }

    None
}

/// Extract the position and the submitted transmitters from the raw data
pub fn extract(raw: &[u8]) -> Result<(Position, Vec<Transmitter>)> {
    let parsed: Report = serde_json::from_slice(raw)?;

    let mut txs = Vec::new();
    for cell in parsed.cell_towers.unwrap_or_default() {
        if should_be_ignored(&parsed.position, cell.age) {
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
            signal_strength: signal_strength(&cell),
            age: cell.age.map(Into::into),
        })
    }
    for wifi in parsed.wifi_access_points.unwrap_or_default() {
        if should_be_ignored(&parsed.position, wifi.age) {
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
                signal_strength: wifi.signal_strength,
                age: wifi.age.map(Into::into),
            });
        }
    }
    for bt in parsed.bluetooth_beacons.unwrap_or_default() {
        if should_be_ignored(&parsed.position, bt.age) {
            continue;
        }
        txs.push(Transmitter::Bluetooth {
            mac: bt.mac_address,
            signal_strength: bt.signal_strength,
            age: bt.age.map(Into::into),
        })
    }

    Ok((parsed.position, txs))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_signal_strength() {
        // 2G: -83 dBm = 15 asu
        assert_eq!(
            signal_strength(&Cell {
                mobile_country_code: 0,
                mobile_network_code: 0,
                location_area_code: None,
                cell_id: None,
                primary_scrambling_code: None,
                age: None,
                signal_strength: None,

                radio_type: RadioType::Gsm,
                asu: Some(15),
            }),
            Some(-83)
        );

        // 3G: -85 dBm = 35 asu
        assert_eq!(
            signal_strength(&Cell {
                mobile_country_code: 0,
                mobile_network_code: 0,
                location_area_code: None,
                cell_id: None,
                primary_scrambling_code: None,
                age: None,
                signal_strength: None,

                radio_type: RadioType::Umts,
                asu: Some(35),
            }),
            Some(-85)
        );

        // 4G: -108 dBm = 32 asu
        assert_eq!(
            signal_strength(&Cell {
                mobile_country_code: 0,
                mobile_network_code: 0,
                location_area_code: None,
                cell_id: None,
                primary_scrambling_code: None,
                age: None,
                signal_strength: None,

                radio_type: RadioType::Lte,
                asu: Some(32),
            }),
            Some(-108)
        );

        // Always prefer signal strength to ASU
        assert_eq!(
            signal_strength(&Cell {
                mobile_country_code: 0,
                mobile_network_code: 0,
                location_area_code: None,
                cell_id: None,
                primary_scrambling_code: None,
                age: None,

                radio_type: RadioType::Lte,
                signal_strength: Some(-20),
                asu: Some(32),
            }),
            Some(-20)
        );

        // Ignore ASU 99 (error)
        assert_eq!(
            signal_strength(&Cell {
                mobile_country_code: 0,
                mobile_network_code: 0,
                location_area_code: None,
                cell_id: None,
                primary_scrambling_code: None,
                age: None,
                signal_strength: None,

                radio_type: RadioType::Lte,
                asu: Some(99),
            }),
            None
        );

        // TODO: Test 5G/NR
    }
}
