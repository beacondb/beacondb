use crate::model::CellRadio;

pub struct Cell {
    pub radio: CellRadio,
    pub country: u16,
    pub network: u16,
    pub area: u32,
    pub cell: u64,
    pub unit: u16,

    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub struct Wifi {
    pub mac: [u8; 6],

    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}

pub struct Bluetooth {
    pub mac: [u8; 6],

    pub x: f64,
    pub y: f64,
    pub w: f64,
    pub h: f64,
}
