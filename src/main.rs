use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::{Parser, Subcommand};
use sqlx::MySqlPool;

mod bounds;
mod db;
mod geolocate;
mod mls;
mod model;
mod submission;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    FormatMls,
    Serve { port: Option<u16> },
    Process,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let pool = MySqlPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&pool).await?;

    match cli.command {
        Command::Serve { port } => {
            HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(pool.clone()))
                    .app_data(web::JsonConfig::default().limit(50 * 1024 * 1024))
                    .service(geolocate::service)
                    .service(submission::geosubmit::service)
            })
            .bind(("0.0.0.0", port.unwrap_or(8080)))?
            .run()
            .await?;
        }

        Command::FormatMls => mls::format()?,
        Command::Process => submission::process::run(pool).await?,
    }

    Ok(())
}
