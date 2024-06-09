use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::{Parser, Subcommand};

mod bounds;
mod db;
mod geosubmit;
mod mls;
mod process;
mod sync;

#[derive(Debug, Parser)]
struct Cli {
    #[clap(subcommand)]
    command: Command,

    #[arg(short, long)]
    database_path: Option<String>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Accept new submissions over HTTP
    Listen {
        port: Option<u16>,
    },
    ImportMls,
    Process,
    Sync,
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Listen { port } => {
            let pool = db::parallel().await?;

            HttpServer::new(move || {
                App::new()
                    .app_data(web::Data::new(pool.clone()))
                    .app_data(web::JsonConfig::default().limit(50 * 1024 * 1024))
                    .service(geosubmit::service)
            })
            .bind(("0.0.0.0", port.unwrap_or(8080)))?
            .run()
            .await?;
        }

        Command::ImportMls => mls::import()?,
        Command::Process => process::run().await?,
        Command::Sync => sync::run()?,
    }

    Ok(())
}
