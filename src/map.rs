//! Utilities to create maps to visualize data.

use anyhow::Result;
use futures::TryStreamExt;
use geojson::Geometry;
use h3o::{geom::SolventBuilder, CellIndex};
use sqlx::{query_scalar, PgPool};

/// Create a geometry from the database h3 tiles
pub async fn run(pool: PgPool) -> Result<()> {
    let tx = pool.begin().await?;
    let mut q = query_scalar!("select h3 from map").fetch(&pool);
    let mut cells = Vec::new();
    while let Some(x) = q.try_next().await? {
        // query!("update map set new = false where h3 = $1", x)
        //     .execute(&mut *tx)
        //     .await?;

        assert_eq!(x.len(), 8);
        let x: [u8; 8] = x.try_into().unwrap();
        let x = u64::from_be_bytes(x);
        let x = CellIndex::try_from(x)?;
        cells.push(x);
    }

    let solvent = SolventBuilder::new().disable_duplicate_detection().build();
    let poly = solvent.dissolve(cells)?;
    let geom = Geometry::new((&poly).into());
    println!("{}", geom.to_string());

    tx.commit().await?;

    Ok(())
}
