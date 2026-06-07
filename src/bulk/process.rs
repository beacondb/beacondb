use std::io::stdin;

use anyhow::Result;
use sqlx::PgPool;

use crate::{
    config::Config,
    submission::{process::BatchProcessor, report::Report},
};

use super::BulkReport;

pub async fn run(config: &Config, pool: PgPool) -> Result<()> {
    let mut input = stdin().lines();
    let mut processor = BatchProcessor::new();

    let mut i = 0;
    while let Some(next) = input.next() {
        let line = next?;
        let bulk: BulkReport = serde_json::from_str(&line)?;
        let report: Report = match serde_json::from_value(bulk.raw) {
            Ok(x) => x,
            Err(e) => {
                eprintln!("failed to parse #{}: {} - {}", bulk.id, e, line);
                continue;
            }
        };

        if let Some((pos, txs)) = report.load() {
            processor.process(config, &pool, pos, txs).await?;
        }

        i += 1;
        if i > 0 && i % 100_000 == 0 {
            eprintln!("{} in batch. committing...", processor.modified.len());
            processor.commit(&pool).await?;
            eprintln!("processed {i} reports - completed up to #{}", bulk.id);

            processor = BatchProcessor::new();
        }
    }

    processor.commit(&pool).await?;

    Ok(())
}
