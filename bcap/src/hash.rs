use arrayref::array_ref;
use blake3::Hasher;
use geo::{HaversineDestination, Point};

pub const SALT: [u8; 2] = [0xbc, 0xdb];
pub const MAX_OFFSET: f64 = 1_000.0; // 1km

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct BeaconHash {
    read_key: u32,
    write_key: u32,
    x_offset: i32,
    y_offset: i32,
}

impl BeaconHash {
    pub fn new(mac: [u8; 6], ssid: &str) -> Self {
        let mut hasher = Hasher::new();
        hasher.update(&SALT);
        hasher.update(&mac);
        hasher.update(ssid.as_bytes());

        let hash = *hasher.finalize().as_bytes();
        let read_key = u32::from_le_bytes(*array_ref![hash, 0, 4]);
        let write_key = u32::from_le_bytes(*array_ref![hash, 4, 4]);
        let x_offset = i32::from_le_bytes(*array_ref![hash, 8, 4]);
        let y_offset = i32::from_le_bytes(*array_ref![hash, 12, 4]);

        Self {
            read_key,
            write_key,
            x_offset,
            y_offset,
        }
    }

    pub fn read_key(&self) -> u32 {
        self.read_key
    }

    pub fn write_key(&self) -> u32 {
        self.write_key
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
mod tests {
    use super::*;

    #[test]
    fn deterministic() {
        let beacon = BeaconHash::new([0x12, 0x34, 0x56, 0xab, 0xcd, 0xef], "testing!");
        assert_eq!(beacon.read_key, 1502776492);
        assert_eq!(beacon.write_key, 1406914681);
        assert_eq!(beacon.x_offset, 634909180);
        assert_eq!(beacon.y_offset, 738142983);
    }
}
