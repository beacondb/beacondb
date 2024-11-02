use std::{collections::BTreeSet, fs, io};

use anyhow::Result;
use h3o::{geom::ToGeo, LatLng, Resolution};

const RESOLUTION: Resolution = Resolution::Eight;

pub fn run() -> Result<()> {
    let mut reader = io::stdin();
    let mut cells = BTreeSet::new();
    for result in reader.lines() {
        let line = result?;
        let (lat, lon) = line.trim().split_once('\t').unwrap();
        let lat: f64 = lat.parse()?;
        let lon: f64 = lon.parse()?;
        let loc = LatLng::new(lat, lon)?;
        let cell = loc.to_cell(RESOLUTION);
        cells.insert(cell);
    }

    let name = format!("{}.geojson", RESOLUTION as u8);
    let x = cells.to_geojson()?;
    let x = x.to_string();
    fs::write(name, x)?;

    Ok(())
}
