use actix_web::{get, web, App, HttpServer};
use sqlx::PgPool;

mod geolocate;
mod geosubmit;
mod observation;

#[get("/")]
pub async fn index() -> String {
    "hi".to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&pool).await?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(index)
            .service(geolocate::service)
            .service(geosubmit::service)
            .app_data(web::JsonConfig::default().limit(50 * 1024 * 1024))
    })
    .bind(("0.0.0.0", 8099))?
    .run()
    .await?;

    Ok(())
}
