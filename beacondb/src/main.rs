use std::path::{Path, PathBuf};

use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::{Parser, Subcommand};
use sqlx::MySqlPool;

mod bounds;
mod config;
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
    FormatMls,
    Serve { port: Option<u16> },
    Process,
    Map,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let path = match cli.config.as_deref() {
        Some(x) => x,
        None => Path::new("config.toml"),
    };
    let config = config::load(path)?;

    let pool = MySqlPool::connect(&config.database_url).await?;
    sqlx::migrate!().run(&pool).await?;

    match cli.command {
        Command::Serve { port } => {
            HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(pool.clone()))
                    .app_data(web::JsonConfig::default().limit(500 * 1024 * 1024))
                    .service(geolocate::service)
                    .service(submission::geosubmit::service)
            })
            .bind(("0.0.0.0", config.http_port))?
            .run()
            .await?;
        }

        Command::FormatMls => mls::format()?,
        Command::Process => submission::process::run(pool, config.stats_path.as_deref()).await?,
        Command::Map => map::run()?,
    }

    Ok(())
}
