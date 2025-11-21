//! Utilities to create maps to visualize data.

use anyhow::Result;
use futures::TryStreamExt;
use geo::ConvexHull;
use geo_types::LineString;
use geojson::Geometry;
use h3o::CellIndex;
use sqlx::{query_scalar, PgPool};

/// Export all h3 cells as individual polygons.
///
/// This exports all cells from the map table as geojson polygons, one per line. This output can
/// then be piped into tippecanoe which can handle 1 geojson Polygon per line. This should perform
/// better than creating a single big MultiPolygon because tippecanoe won't have to parse a big json
/// object.
pub async fn run(pool: PgPool) -> Result<()> {
    let mut q = query_scalar!("select h3 from map").fetch(&pool);

    let mut first = true;
    while let Some(x) = q.try_next().await? {
        assert_eq!(x.len(), 8);

        if first {
            first = false;
        } else {
            print!(",");
        }
        let x: [u8; 8] = x.try_into().unwrap();
        let x = u64::from_be_bytes(x);
        let x = CellIndex::try_from(x)?;
        let s: LineString = x.boundary().into();
        let p = Geometry::new((&s.convex_hull()).into());
        println!("{}", p.to_string());
    }

    Ok(())
}
