use std::{collections::BTreeSet, fs, io};

use anyhow::Result;
use geojson::Geometry;
use geo_types::MultiPolygon;
use h3o::{geom::dissolve, LatLng, Resolution};

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

    let multi_polygon: MultiPolygon = dissolve(cells)?;
    let geom = Geometry::from(&multi_polygon);

    let name = format!("{}.geojson", RESOLUTION as u8);
    fs::write(name, geom.to_string())?;

    Ok(())
}
