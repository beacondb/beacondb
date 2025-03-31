//! Archive all the reports submitted by users.
//!
//! This module handles the archive command.
//! Currently the only subcommand is `export` which exports all submitted data in JSON-format.
//! The export can be triggered manually to remove processed reports from the database
//! to decrease its size and improve speed.

use anyhow::Result;
use clap::Subcommand;
use futures::TryStreamExt;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use sqlx::{query, PgPool};

/// Enum of possible archive commands
#[derive(Debug, Subcommand)]
pub enum ArchiveCommand {
    /// Export processed reports into a JSON-file
    Export,
}

/// Serde representation of a report
#[derive(Deserialize, Serialize)]
struct ArchivedReport {
    id: i32,
    user_agent: Option<String>,
    raw: Value,
}

/// Main entry point of the archive command
pub async fn run(pool: PgPool, command: ArchiveCommand) -> Result<()> {
    match command {
        ArchiveCommand::Export => {
            let mut reports =
                query!("select id, user_agent, raw from report").fetch(&pool);
            while let Some(record) = reports.try_next().await? {
                let archived_report = ArchivedReport {
                    id: record.id,
                    user_agent: record.user_agent,
                    raw: serde_json::from_slice(&record.raw)?,
                };
                println!("{}", serde_json::to_string(&archived_report)?);
            }
        }
    }

    Ok(())
}
