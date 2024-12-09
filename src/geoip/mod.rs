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
use ipnetwork::IpNetwork;
use nodit::{interval::ii, Interval, NoditMap};
use serde::Deserialize;
use serde_json::json;
use sqlx::{query_file, PgPool};

mod country;
pub use country::Country;
pub mod import;

pub const LICENSE: &str =
    "IP geolocation data sourced from IP to City Lite by DB-IP, licensed under CC BY 4.0.";

#[post("/v1/country")]
pub async fn country_service(
    pool: web::Data<PgPool>,
    req: HttpRequest,
) -> actix_web::Result<HttpResponse> {
    let ip = req
        .headers()
        .get("X-Forwarded-For")
        .and_then(|x| x.to_str().ok())
        .and_then(|x| IpNetwork::from_str(x).ok())
        .context("failed to get client ip address")
        .map_err(ErrorInternalServerError)?;

    if let Some(record) = query_file!("src/geoip/lookup.sql", ip)
        .fetch_optional(&**pool)
        .await
        .context("database error")
        .map_err(ErrorInternalServerError)?
    {
        let country: Country = record
            .country
            .parse()
            .context("invalid database")
            .map_err(ErrorInternalServerError)?;
        Ok(HttpResponse::Ok().json(json!({
            "license": LICENSE,
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
