use std::{collections::BTreeSet, fs, io};

use anyhow::Result;
use futures::TryStreamExt;
use geo_types::MultiPolygon;
use geojson::{FeatureCollection, Geometry};
use h3o::{geom::dissolve, CellIndex, LatLng, Resolution};
use sqlx::{query, query_scalar, PgPool};

pub const RESOLUTION: Resolution = Resolution::Eight;

pub async fn run(pool: PgPool) -> Result<()> {
    let mut q = query_scalar!("select h3 from map").fetch(&pool);
    let mut features = Vec::new();
    while let Some(x) = q.try_next().await? {
        assert_eq!(x.len(), 8);
        let x: [u8; 8] = x.try_into().unwrap();
        let x = u64::from_be_bytes(x);
        let x = CellIndex::try_from(x)?;
        let poly = dissolve([x])?;
        let geom = Geometry::new((&poly).into());
        features.push(geom.into());
    }

    let coll = FeatureCollection {
        bbox: None,
        features,
        foreign_members: None,
    };
    println!("{}", coll.to_string());

    Ok(())
}
