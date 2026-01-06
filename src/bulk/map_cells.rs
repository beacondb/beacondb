use std::{collections::BTreeSet, io::stdin};

use anyhow::Result;
use h3o::{LatLng, Resolution};

use crate::{bulk::BulkReport, config::Config, submission::report::Report};

pub fn run(config: Config) -> Result<()> {
    let mut input = stdin().lines();

    let mut i = 0;
    let mut h3s = BTreeSet::new();

    while let Some(line) = input.next() {
        let bulk: BulkReport = serde_json::from_str(&line?)?;
        let parsed: Report = match serde_json::from_value(bulk.raw) {
            Ok(x) => x,
            Err(_) => continue,
        };
        let loaded = match parsed.load() {
            Ok(x) => x,
            Err(_) => continue,
        };
        i += 1;
        if (i % 1_000_000) == 0 {
            eprintln!("{i}");
        }

        if let Some((pos, _)) = loaded {
            let pos = LatLng::new(pos.latitude, pos.longitude)?;
            let h3 = pos.to_cell(Resolution::try_from(config.h3_resolution)?);
            h3s.insert(h3);
        }
    }

    for h3 in h3s {
        println!("\\\\x0{h3}");
    }

    Ok(())
}
