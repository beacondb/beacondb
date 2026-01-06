//! Tools for handling large amounts of raw data.
//!
//! This amount of raw data is technically a database dump, but in the context of BeaconDB the term "database dumps"
//! already refers to the public dataset that the project plans to release.

use anyhow::Result;
use chrono::{DateTime, Utc};
use clap::Subcommand;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::PgPool;

use crate::config::Config;

mod export;
mod map_cells;
mod parse;

#[derive(Debug, Subcommand)]
pub enum BulkCommand {
    /// Export processed reports into a JSON file for cold storage (for now)
    Export,
    /// Parse reports to catch unexpected parsing errors
    Parse,
    /// Calculate h3 cells for the map table without reprocessing the entire database
    MapCells,
}

/// Format used to export reports from the database without losing data contained in the original JSON
#[derive(Deserialize, Serialize)]
struct BulkReport {
    id: i32,
    submitted_at: DateTime<Utc>,
    user_agent: Option<String>,
    raw: Value,
}

pub async fn run(pool: PgPool, config: Config, command: BulkCommand) -> Result<()> {
    match command {
        BulkCommand::Export => {
            export::run(pool).await?;
        }
        BulkCommand::Parse => {
            parse::run()?;
        }
        BulkCommand::MapCells => {
            map_cells::run(config)?;
        }
    }

    Ok(())
}
