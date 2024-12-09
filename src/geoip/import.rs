use std::{io, net::IpAddr, str::FromStr};

use anyhow::{bail, Result};
use ipnetwork::IpNetwork;
use serde::Deserialize;
use sqlx::{query, PgPool};

use super::Country;

#[derive(Debug, Deserialize)]
struct RawRecord {
    start: IpNetwork,
    end: IpNetwork,
    continent: String,
    country: String,
    state: String,
    city: String,
    latitude: f64,
    longitude: f64,
}

pub async fn run(pool: PgPool) -> Result<()> {
    let mut reader = csv::ReaderBuilder::new()
        .has_headers(false)
        .from_reader(io::stdin());
    let mut tx = pool.begin().await?;
    for (i, result) in reader.deserialize().enumerate() {
        let RawRecord {
            start,
            end,
            continent,
            country,
            state,
            city,
            latitude,
            longitude,
        } = result?;

        if country == "ZZ" {
            continue;
        }
        // check it fits into the rust enum
        Country::from_str(&country)?;

        query!(
            "insert into geoip (cidr, range_start, range_end, country, latitude, longitude) values (inet_merge($1, $2), $1, $2, $3, $4, $5)",
            start,
            end,
            country,
            latitude,
            longitude
        ).execute(&mut *tx).await?;

        if (i > 0 && i % 100_000 == 0) {
            dbg!(i);
        }
    }
    tx.commit().await?;

    Ok(())
}
