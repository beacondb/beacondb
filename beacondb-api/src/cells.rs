use actix_web::{error::ErrorInternalServerError, get, web, HttpResponse, Result};
use serde::{Deserialize, Serialize};
use sqlx::{query_as, SqlitePool};

#[derive(Debug, Deserialize, Clone, Copy)]
#[serde(rename_all = "kebab-case")]
enum CellRadio {
    Gsm = 0,
    Umts = 1,
    Lte = 2,
}

#[derive(Debug, Serialize)]
struct CellAreaTower {
    cell: i64,
    unit: i64,
    x: f64,
    y: f64,
    r: f64,
    // created: i64,
    // updated: i64,
}

#[derive(Debug, Deserialize)]
struct QueryOptions {
    #[serde(default)]
    since: u64,
}

#[get("/v0/cells/{radio}/{country}/{network}/{area}")]
pub async fn cell_area(
    path: web::Path<(CellRadio, u16, u16, u32)>,
    web::Query(q): web::Query<QueryOptions>,
    pool: web::Data<SqlitePool>,
) -> Result<HttpResponse> {
    let (radio, country, network, area) = path.into_inner();
    let pool = pool.into_inner();

    let r = radio as u8;
    let updated = q.since as i64;
    let cells = query_as!(CellAreaTower, "select cell, unit, x, y, r from cell where radio = ?1 and country = ?2 and network = ?3 and area = ?4",
        r,
        country,
        network,
        area,
        // updated
    ).fetch_all(&*pool).await.map_err(ErrorInternalServerError)?;

    if cells.is_empty() {
        Ok(HttpResponse::NoContent().into())
    } else {
        let mut csv = csv::Writer::from_writer(Vec::new());
        for cell in cells {
            csv.serialize(cell).map_err(ErrorInternalServerError)?;
        }
        let body = csv.into_inner().map_err(ErrorInternalServerError)?;

        let r = match &radio {
            CellRadio::Gsm => "gsm",
            CellRadio::Umts => "umts",
            CellRadio::Lte => "lte",
        };
        Ok(HttpResponse::Ok()
            .insert_header(("content-type", "text/csv"))
            .insert_header((
                "content-disposition",
                format!("attachment; filename=\"{r}-{country}-{network}-{area}.csv\""),
            ))
            .insert_header(("cache-control", "public, max-age=604800"))
            .body(body))
    }
}
