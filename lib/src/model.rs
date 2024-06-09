pub struct Cell {
    pub radio: RadioType,
    pub country: u16,
    pub network: u16,
    pub area: u32,
    pub cell: u64,
    // #[serde(default)]
    pub unit: u16,
    pub x: f64,
    pub y: f64,
    pub r: f64,
}

// #[derive(Debug, Deserialize, Serialize, Clone, Copy, PartialEq, Eq)]
// #[serde(rename_all = "UPPERCASE")]
#[repr(u8)]
pub enum RadioType {
    Gsm,
    Umts,
    Lte,
}

pub struct Wifi {
    pub key: u16,
    pub x: f64,
    pub y: f64,
    pub r: f64,
}
