use actix_web::{
    error::ErrorInternalServerError,
    http::{header::USER_AGENT, StatusCode},
    post, web, HttpRequest, HttpResponse, Responder,
};
use anyhow::Context;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{query, MySqlPool};

// only the bare minimum is parsed here: it is assumed that certain data issues
// may be due to device manufacturer software, making it difficult for
// developers to handle per device.
//
// - https://github.com/mjaakko/NeoStumbler/issues/88

#[derive(Deserialize)]
struct Submission {
    items: Vec<Report>,
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct Report {
    #[serde(with = "chrono::serde::ts_milliseconds")]
    timestamp: DateTime<Utc>,
    position: Position,
    #[serde(flatten)]
    extra: Value,
}

#[derive(Deserialize, Serialize)]
struct Position {
    latitude: f32,
    longitude: f32,
    #[serde(flatten)]
    extra: Value,
}

#[post("/v2/geosubmit")]
pub async fn service(
    data: web::Json<Submission>,
    pool: web::Data<MySqlPool>,
    req: HttpRequest,
) -> actix_web::Result<impl Responder> {
    let data = data.into_inner();
    let pool = pool.into_inner();

    let ua = match req.headers().get(USER_AGENT).map(|x| x.to_str()) {
        Some(Ok(x)) => Some(x),
        Some(Err(_)) => {
            return Ok(HttpResponse::BadRequest().body("user agent contains invalid characters"))
        }
        None => None,
    };

    insert(&pool, ua, data)
        .await
        .context("writing to database failed")
        .map_err(ErrorInternalServerError)?;

    // StatusCode::ACCEPTED is more accurate but ichnaea API documentation says that should be
    // StatusCode::OK
    // https://ichnaea.readthedocs.io/en/latest/api/geosubmit2.html#response
    // https://github.com/zamojski/TowerCollector/pull/225
    Ok(HttpResponse::new(StatusCode::OK))
}

async fn insert(
    pool: &MySqlPool,
    user_agent: Option<&str>,
    submission: Submission,
) -> anyhow::Result<()> {
    let mut tx = pool.begin().await?;

    for report in submission.items {
        query!("insert ignore into submission (timestamp, latitude, longitude, user_agent, raw) values (?, ?, ?, ?, ?)",
            report.timestamp,
            report.position.latitude,
            report.position.longitude,
            user_agent,
            serde_json::to_string(&report)?,
        ).execute(&mut *tx).await?;
    }

    tx.commit().await?;
    Ok(())
}
