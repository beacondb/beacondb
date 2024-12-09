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
pub use country::Country;
mod db;
pub use db::GeoIpDatabase;

#[derive(Deserialize)]
pub struct GeoIpConfig {
    path: PathBuf,
}

#[post("/v1/country")]
pub async fn country_service(
    geoip: web::Data<Option<Arc<GeoIpDatabase>>>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    // let ipmap = geoip
    //     .as_deref()
    //     .context("geoip has not been configured on this server")
    //     .map_err(ErrorInternalServerError)?;
    let ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|x| x.to_str().ok())
        .and_then(|x| IpAddr::from_str(x).ok())
        .context("failed to get client ip address")
        .map_err(ErrorInternalServerError)?;

    if let Some(country) = geoip.as_deref().and_then(|x| x.lookup(ip)) {
        Ok(HttpResponse::Ok().json(json!({
            "country_code": country.country.as_ref(),
            "country_name": country.country.name(),
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
