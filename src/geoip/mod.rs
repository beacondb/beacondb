use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::{BufRead, Read},
    net::IpAddr,
    path::PathBuf,
    str::FromStr,
    sync::Arc,
};

use actix_web::{error::ErrorInternalServerError, post, web, HttpRequest, HttpResponse};
use anyhow::{Context, Result};
use nodit::{interval::ii, Interval, NoditMap};
use serde::Deserialize;
use serde_json::json;

mod country;
use country::Country;

#[derive(Deserialize)]
pub struct GeoIpConfig {
    ipv4_path: PathBuf,
    ipv6_path: PathBuf,
}

pub struct IpAddrMap {
    v4: NoditMap<u32, Interval<u32>, Country>,
    v6: NoditMap<u128, Interval<u128>, Country>,
}

impl IpAddrMap {
    pub fn load(config: GeoIpConfig) -> Result<Self> {
        let mut v4 = NoditMap::new();
        for result in fs::read(&config.ipv4_path)?.lines() {
            let line = result?;
            let parts: Vec<_> = line.split(',').collect();
            assert_eq!(parts.len(), 3);
            let start: u32 = parts[0].parse()?;
            let end: u32 = parts[1].parse()?;
            let interval = ii(start, end);
            let country = Country::from_str(parts[2])?;
            v4.insert_strict(interval, country)
                .ok()
                .context("overlapping")?;
        }

        let mut v6 = NoditMap::new();
        for result in fs::read(&config.ipv6_path)?.lines() {
            let line = result?;
            let parts: Vec<_> = line.split(',').collect();
            assert_eq!(parts.len(), 3);

            // https://en.wikipedia.org/wiki/ISO_3166-1_alpha-2#Codes_currently_agreed_not_to_use
            if parts[2] == "AP" {
                continue;
            }

            let start: u128 = parts[0].parse()?;
            let end: u128 = parts[1].parse()?;
            let interval = ii(start, end);
            let country = Country::from_str(parts[2])?;

            v6.insert_strict(interval, country)
                .ok()
                .context("overlapping")?;
        }

        Ok(Self { v4, v6 })
    }

    pub fn lookup_country(&self, ip: IpAddr) -> actix_web::Result<HttpResponse> {
        let country = match ip {
            IpAddr::V4(x) => self.v4.get_at_point(x.to_bits()),
            IpAddr::V6(x) => self.v6.get_at_point(x.to_bits()),
        };

        if let Some(country) = country {
            Ok(HttpResponse::Ok().json(json!({
                "country_code": country.as_ref(),
                "country_name": country.name(),
                "fallback": "ipf"
            })))
        } else {
            Ok(HttpResponse::NotFound().json(json!({
                "error": {
                    "errors": [{
                        "domain": "geolocation",
                        "reason": "notFound",
                        "message": "No location could be estimated based on the data provided",
                    }],
                    "code": 404,
                    "message": "Not found",
            }})))
        }
    }
}

#[post("/v1/country")]
pub async fn country_service(
    ipmap: web::Data<Option<Arc<IpAddrMap>>>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let ipmap = ipmap
        .as_deref()
        .context("geoip has not been configured on this server")
        .map_err(ErrorInternalServerError)?;
    let ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|x| x.to_str().ok())
        .and_then(|x| IpAddr::from_str(x).ok())
        .context("failed to get client ip address")
        .map_err(ErrorInternalServerError)?;

    ipmap.lookup_country(ip)
}
