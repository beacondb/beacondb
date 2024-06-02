use actix_web::{web, App, HttpServer};
use clap::Parser;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

mod cells;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    subcommand: Subcommand,
}

#[derive(Debug, clap::Subcommand, Clone)]
enum Subcommand {
    ImportCells,
    Serve,
    ProcessSubmissions,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename("../beacondb/beacon.db")
            .read_only(true),
    )
    .await?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(cells::cell_area)
    })
    .bind(("0.0.0.0", 8099))?
    .run()
    .await?;

    Ok(())
}
