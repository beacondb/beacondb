use actix_web::{web, App, HttpServer};
use anyhow::Result;
use clap::{Parser, Subcommand};
use rusqlite::Connection;
use sqlx::PgPool;

mod bounds;
mod geosubmit;
mod mls;
mod process;

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
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    match cli.command {
        Command::Listen { port } => {
            let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
            sqlx::migrate!().run(&pool).await?;

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
        Command::ImportMls => {
            let mut conn = Connection::open("./beacon.db")?;
            conn.execute_batch(include_str!("../db.sql"))?;
            mls::import(&mut conn)?;
        }
        Command::Process => {
            let mut conn = Connection::open("./beacon.db")?;
            conn.execute_batch(include_str!("../db.sql"))?;
            let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
            process::main(pool, &mut conn).await?;
        }
    }

    Ok(())
}
