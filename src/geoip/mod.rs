//! This module handles ip based localization.
//!
//! When the location cannot be determined using the data from the database
//! `beacondb` tries to estimate the location from the ip address.
//! The `DB-IP` dataset is used to link the ip address to a location.

use std::{fs, net::IpAddr, path::PathBuf};

use anyhow::{Context, Result, bail};
use maxminddb::{Metadata, Reader, path};

use crate::geolocate::Location;

/// Constants used for GeoIP
pub const LICENSE: &str =
    "IP geolocation data sourced from IP to City Lite by DB-IP, licensed under CC BY 4.0.";

pub const DATABASE_TYPE: &str = "DBIP-City-Lite";

// DB-IP data does not include location accuracy
pub const LOCATION_ACCURACY: u64 = 25_000;

pub type MMDB = Reader<Vec<u8>>;

pub fn load(path: &PathBuf) -> Result<MMDB> {
    eprintln!("Loading GeoIP...");
    let buf = fs::read(path)?;

    let database = maxminddb::Reader::from_source(buf)?;

    let Metadata {
        database_type,
        build_epoch,
        ..
    } = &database.metadata;

    // if you want to use a database from another provider, you'll most
    // likely need to change the paths used to access lat/lon data
    if database_type != DATABASE_TYPE {
        bail!("Unexpected GeoIP database type: {}", database_type);
    }

    eprintln!("Loaded GeoIP: {database_type} {build_epoch} ");
    Ok(database)
}

pub fn lookup(mmdb: &MMDB, ip: IpAddr) -> Result<Option<Location>> {
    let result = mmdb.lookup(ip)?;
    if result.has_data() {
        let latitude: f64 = result
            .decode_path(&path!["location", "latitude"])
            .context("geoip type mismatch")?
            .context("geoip data path missing")?;
        let longitude: f64 = result
            .decode_path(&path!["location", "longitude"])
            .context("geoip type mismatch")?
            .context("geoip data path missing")?;

        Ok(Some(Location {
            lat: latitude,
            lng: longitude,
        }))
    } else {
        Ok(None)
    }
}
