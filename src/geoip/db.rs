use std::{
    fs,
    net::{IpAddr, Ipv4Addr},
    path::Path,
    str::FromStr,
};

use actix_web::{error::ErrorInternalServerError, post, web, HttpRequest, HttpResponse};
use anyhow::{bail, Context, Result};
use nodit::{interval::ii, Interval, NoditMap};
use serde::Deserialize;
use serde_json::json;

use super::{Country, GeoIpConfig};

pub struct GeoIpDatabase {
    v4: NoditMap<u32, Interval<u32>, Record>,
    v6: NoditMap<u128, Interval<u128>, Record>,
}

#[derive(Debug, Deserialize)]
struct RawRecord {
    start: IpAddr,
    end: IpAddr,
    continent: String,
    country: String,
    state: String,
    city: String,
    latitude: f64,
    longitude: f64,
}

#[derive(Debug, Deserialize)]
pub struct Record {
    pub continent: String,
    pub country: String,
    pub state: String,
    pub city: String,
    pub latitude: f64,
    pub longitude: f64,
}

impl GeoIpDatabase {
    pub fn load(config: GeoIpConfig) -> Result<Self> {
        eprintln!("Parsing");

        let mut v4 = NoditMap::new();
        let mut v6 = NoditMap::new();

        let mut reader = csv::ReaderBuilder::new()
            .has_headers(false)
            .from_path(&config.path)?;
        for result in reader.deserialize() {
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
            let record = Record {
                continent,
                country,
                state,
                city,
                latitude,
                longitude,
            };
            match (start, end) {
                (IpAddr::V4(start), IpAddr::V4(end)) => {
                    v4.insert_strict(ii(start.to_bits(), end.to_bits()), record);
                }
                (IpAddr::V6(start), IpAddr::V6(end)) => {
                    v6.insert_strict(ii(start.to_bits(), end.to_bits()), record);
                }
                _ => bail!("mismatched ip versions"),
            }
        }
        eprintln!("OK");

        Ok(Self { v4, v6 })
    }

    pub fn lookup(&self, addr: IpAddr) -> Option<&Record> {
        dbg!(&addr);
        match addr {
            IpAddr::V4(x) => self.v4.get_at_point(x.to_bits()),
            IpAddr::V6(x) => self.v6.get_at_point(x.to_bits()),
        }
    }
}
