use std::io::stdin;

use anyhow::Result;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use serde_json::{json, Value};

use crate::{bulk::BulkReport, submission::report::Report};

const BATCH_SIZE: usize = 100_000;

pub fn run() -> Result<()> {
    let mut input = stdin().lines();
    let mut batch = Vec::new();
    let mut i = 0;

    while let Some(next) = input.next() {
        batch.push(next?);
        if batch.len() >= BATCH_SIZE {
            handle_batch(batch)?;
            batch = Vec::new();

            i += 1;
            if (i % 10) == 0 {
                eprintln!("{}", i * BATCH_SIZE);
            }
        }
    }
    handle_batch(batch)?;

    Ok(())
}

fn handle_batch(batch: Vec<String>) -> Result<()> {
    let batch: Vec<_> = batch
        .into_par_iter()
        .map(|report| handle_report(&report))
        .collect();
    for result in batch {
        if let Some(error) = result? {
            println!("{error}");
        }
    }
    Ok(())
}

fn handle_report(report: &str) -> Result<Option<Value>> {
    let bulk: BulkReport = serde_json::from_str(report)?;

    Ok(match parse_report(&bulk.raw) {
        Ok(()) => None,
        Err(e) => Some(json! ({ "error": e.to_string(), "report": bulk })),
    })
}

fn parse_report(raw: &Value) -> Result<()> {
    let parsed: Report = serde_json::from_value(raw.clone())?;
    parsed.load()?;
    Ok(())
}
