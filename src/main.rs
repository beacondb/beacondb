use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::{Parser, Subcommand};
use sqlx::PgPool;

mod bounds;
mod config;
mod geoip;
mod geolocate;
mod map;
mod mls;
mod model;
mod submission;

#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    Serve { port: Option<u16> },
    Process,
    Map,
    FormatMls,
    ImportGeoip,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let path = match cli.config.as_deref() {
        Some(x) => x,
        None => Path::new("config.toml"),
    };
    let config = config::load(path)?;

    let pool = PgPool::connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    match cli.command {
        Command::Serve { port } => {
            HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(pool.clone()))
                    .app_data(web::JsonConfig::default().limit(500 * 1024 * 1024))
                    .service(geoip::country_service)
                    .service(geolocate::service)
                    .service(submission::geosubmit::service)
            })
            .bind(("0.0.0.0", config.http_port))?
            .run()
            .await?;
        }

        Command::Process => submission::process::run(pool, config.stats.as_ref()).await?,
        Command::Map => map::run()?,

        Command::ImportGeoip => geoip::import::run(pool).await?,
        Command::FormatMls => mls::format()?,
    };

    Ok(())
}
