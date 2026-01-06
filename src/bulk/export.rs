use anyhow::Result;
use futures::TryStreamExt;
use sqlx::{query, PgPool};

use super::BulkReport;

pub async fn run(pool: PgPool) -> Result<()> {
    let mut reports = query!("select id, submitted_at, user_agent, raw from report").fetch(&pool);
    while let Some(record) = reports.try_next().await? {
        let archived_report = BulkReport {
            id: record.id,
            submitted_at: record.submitted_at,
            user_agent: record.user_agent,
            raw: serde_json::from_slice(&record.raw)?,
        };
        println!("{}", serde_json::to_string(&archived_report)?);
    }

    Ok(())
}
