use geo::Point;

use crate::BeaconHash;

#[non_exhaustive]
pub enum Observation {
    WiFi(WiFiObservation),
}

pub struct WiFiObservation {
    pub position: Position,
    pub read_key: u32,
    pub write_key: u32,
    pub signal: Option<i8>,
}

impl WiFiObservation {
    pub fn new(mut position: Position, mac: [u8; 6], ssid: &str, signal: Option<i8>) -> Self {
        let hash = BeaconHash::new(mac, ssid);

        // add offset to position
        let p = Point::new(position.longitude, position.latitude);
        let (x, y) = hash.add_offset(p).x_y();
        position.longitude = x;
        position.latitude = y;

        let (read_key, write_key) = (hash.read_key(), hash.write_key());
        Self {
            position,
            read_key,
            write_key,
            signal,
        }
    }
}

impl From<WiFiObservation> for Observation {
    fn from(value: WiFiObservation) -> Self {
        Observation::WiFi(value)
    }
}

pub struct Position {
    pub latitude: f64,
    pub longitude: f64,
    pub accuracy: Option<f64>,
    pub altitude: Option<f64>,
    pub altitude_accuracy: Option<f64>,
    pub speed: Option<f64>,
}
