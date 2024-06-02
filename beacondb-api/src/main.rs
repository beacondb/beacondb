use actix_web::{web, App, HttpServer};
use clap::Parser;
use sqlx::{sqlite::SqliteConnectOptions, SqlitePool};

mod cells;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long)]
    database_path: Option<String>,
    #[arg(short, long)]
    port: Option<u16>,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    let pool = SqlitePool::connect_with(
        SqliteConnectOptions::new()
            .filename(
                cli.database_path
                    .as_deref()
                    .unwrap_or("../beacondb/beacon.db"),
            )
            .read_only(true),
    )
    .await?;

    HttpServer::new(move || {
        App::new()
            .app_data(web::Data::new(pool.clone()))
            .service(cells::cell_area)
    })
    .bind(("0.0.0.0", cli.port.unwrap_or(8080)))?
    .run()
    .await?;

    Ok(())
}
