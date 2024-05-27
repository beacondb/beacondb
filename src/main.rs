use actix_web::{get, web, App, HttpServer};
use clap::Parser;
use sqlx::PgPool;

mod geolocate;
// mod geosubmit;
mod import_cells;
// mod observation;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, clap::Subcommand, Clone)]
enum Subcommand {
    ImportCells,
    Serve,
}

#[get("/")]
pub async fn index() -> String {
    "hi".to_string()
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();

    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&pool).await?;

    match cli.subcommand {
        Subcommand::ImportCells => import_cells::main(&pool).await?,
        Subcommand::Serve => {
            HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(pool.clone()))
                    .service(index)
                    .service(geolocate::service)
                    // .service(geosubmit::service)
                    .app_data(web::JsonConfig::default().limit(50 * 1024 * 1024))
            })
            .bind(("0.0.0.0", 8099))?
            .run()
            .await?;
        }
    };

    Ok(())
}
