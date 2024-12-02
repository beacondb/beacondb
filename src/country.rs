use std::collections::BTreeSet;

use actix_web::{error::ErrorInternalServerError, post, web, HttpRequest, HttpResponse};
use geo::{Distance, Haversine};
use mac_address::MacAddress;
use serde::{Deserialize, Serialize};
use serde_json::json;
use sqlx::{query, query_as, PgPool};

use crate::{bounds::Bounds, model::CellRadio};

#[post("/v1/country")]
pub async fn service(req: HttpRequest) -> actix_web::Result<HttpResponse> {
    Ok(HttpResponse::NotFound().json(json!(
        {
            "error": {
                "errors": [{
                    "domain": "geolocation",
                    "reason": "notFound",
                    "message": "No location could be estimated based on the data provided",
                }],
                "code": 404,
                "message": "Not found",
            }
        }
    )))
}
