use geo::{HaversineDestination, Point};
use sha2::{Digest, Sha256};

pub mod model;

pub const SALT: [u8; 2] = [0xbc, 0xdb];
pub const MAX_OFFSET: f64 = 10_000.0; // 10km

pub type MacAddress = [u8; 6];

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct KnownBeacon {
    key: u16,
    secret: u32,
    x_offset: i32,
    y_offset: i32,
}

impl KnownBeacon {
    pub fn new(mac: MacAddress) -> Self {
        let mut hasher = Sha256::new();
        hasher.update(SALT);
        hasher.update(mac);
        let result: [u8; 32] = hasher.finalize().into();

        let key = u16::from_le_bytes(result[0..2].try_into().unwrap());
        let secret = u32::from_le_bytes(result[2..6].try_into().unwrap());
        let x_offset = i32::from_le_bytes(result[6..10].try_into().unwrap());
        let y_offset = i32::from_le_bytes(result[10..14].try_into().unwrap());

        Self {
            key,
            secret,
            x_offset,
            y_offset,
        }
    }

    pub fn key(&self) -> u16 {
        self.key
    }

    pub fn secret(&self) -> u32 {
        self.secret
    }

    fn offset(&self) -> (f64, f64) {
        let x = self.x_offset as f64 / 2.0f64.powi(32) * MAX_OFFSET;
        let y = self.y_offset as f64 / 2.0f64.powi(32) * MAX_OFFSET;
        (x, y)
    }

    pub fn add_offset(&self, p: Point) -> Point {
        let (x, y) = self.offset();
        p.haversine_destination(90.0, x)
            .haversine_destination(0.0, y)
    }

    pub fn remove_offset(&self, p: Point) -> Point {
        let (x, y) = self.offset();
        p.haversine_destination(270.0, x)
            .haversine_destination(180.0, y)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn deterministic() {
        let beacon = KnownBeacon::new([0x00, 0x01, 0x02, 0x03, 0x04, 0x05]);
        assert!(beacon.key == 38081);
        assert!(beacon.secret == 3840286906);
        assert!(beacon.x_offset == 1135212724);
        assert!(beacon.y_offset == -1067958802);
    }
}
