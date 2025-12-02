//! `beacondb` is a server to geolocate a client based on the nearby wifis, cell towers and bluetooth beacons.
//! It is also used to collect data from mappers and processes that data.

use std::path::{Path, PathBuf};

use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::{Args, Parser, Subcommand};
use sqlx::PgPool;

mod bounds;
mod bulk;
mod config;
mod geoip;
mod geolocate;
mod map;
mod mls;
mod model;
mod submission;

/// Command line interface parser.
#[derive(Debug, Parser)]
struct Cli {
    #[arg(short, long)]
    config: Option<PathBuf>,

    #[clap(subcommand)]
    command: Command,
}

#[derive(Debug, Args)]
struct MapArgs {
    /// Size of the lookback buffer used when merging cells.
    ///
    /// A larger lookback buffer will find more clusters of cells that can be merged, but will be
    /// slower and use more memory.
    #[arg(short, long, default_value_t = 20)]
    lookback_size: usize,
}

/// Subcommands of the cli parser
#[derive(Debug, Subcommand)]
enum Command {
    /// Serve the beacondb geolocate service
    Serve,
    /// Process newly submitted reports
    Process,
    /// Export a map of all data as h3 hexagons
    Map(MapArgs),
    /// Archive reports out of the database
    Bulk {
        #[clap(subcommand)]
        command: bulk::BulkCommand,
    },
    /// Reformat data to the MLS format
    FormatMls,
    /// Import mapping from ip address to a geolocation
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
        Command::Serve => {
            println!("beaconDB server is starting at port {}", config.http_port);
            HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(pool.clone()))
                    .app_data(web::JsonConfig::default().limit(500 * 1024 * 1024))
                    .service(geoip::country_service)
                    .service(geolocate::service)
                    .service(submission::geosubmit::service)
            })
            .bind(("::", config.http_port))?
            .run()
            .await?;
            println!("Gracefully stopped beaconDB server");
        }

        Command::Process => submission::process::run(pool, config).await?,
        Command::Map(a) => map::run(pool, a).await?,

        Command::Bulk { command } => bulk::run(pool, command).await?,

        Command::ImportGeoip => geoip::import::run(pool).await?,
        Command::FormatMls => mls::format()?,
    };

    Ok(())
}
