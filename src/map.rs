use std::{collections::BTreeSet, fs, io};

use anyhow::Result;
use h3o::{geom::ToGeo, LatLng, Resolution};

const BASE_RESOLUTION: Resolution = Resolution::Seven;

pub fn run() -> Result<()> {
    let mut reader = io::stdin();
    let mut cells = BTreeSet::new();
    for result in reader.lines() {
        let line = result?;
        let (lat, lon) = line.trim().split_once('\t').unwrap();
        let lat: f64 = lat.parse()?;
        let lon: f64 = lon.parse()?;
        let loc = LatLng::new(lat, lon)?;
        let cell = loc.to_cell(BASE_RESOLUTION);
        cells.insert(cell);
    }

    // TODO: should do this client side...
    let mut cells: Vec<_> = cells.into_iter().collect();
    let mut resolution = BASE_RESOLUTION;
    let mut parents = BTreeSet::new();
    while let Some(next) = resolution.pred() {
        for cell in &cells {
            parents.insert(cell.parent(next).unwrap());
        }

        let name = format!("{}.geojson", resolution as u8);
        let x = cells.to_geojson()?;
        let x = x.to_string();
        fs::write(name, x)?;

        cells = parents.into_iter().collect();
        parents = BTreeSet::new();
        resolution = next;
    }

    Ok(())
}
