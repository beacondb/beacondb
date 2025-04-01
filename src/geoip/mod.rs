//! This module handles ip based localization.
//!
//! When the location cannot be determined using the data from the database
//! `beacondb` tries to estimate the location from the ip address.
//! The `DB-IP` dataset is used to link the ip address to a location.

use std::str::FromStr;

use actix_web::{error::ErrorInternalServerError, post, web, HttpRequest, HttpResponse};
use anyhow::Context;
use ipnetwork::IpNetwork;
use serde_json::json;
use sqlx::{query_file, PgPool};

mod country;
pub use country::Country;
pub mod import;

/// License of DB-IP data
pub const LICENSE: &str =
    "IP geolocation data sourced from IP to City Lite by DB-IP, licensed under CC BY 4.0.";

/// Geolocalize user based on IP
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
