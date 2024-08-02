use std::path::PathBuf;

use anyhow::Result;
use bcap::observation::Observation;
use clap::{Parser, ValueEnum};

mod neostumbler;
mod wigle;

#[derive(Debug, Clone, Parser)]
struct Cli {
    format: Format,
    files: Vec<PathBuf>,
}

#[derive(Debug, Clone, ValueEnum)]
enum Format {
    #[value(name = "neostumbler")]
    NeoStumbler,
    #[value(name = "wigle")]
    WiGLE,
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    for file in cli.files {
        let obs = match cli.format {
            Format::NeoStumbler => neostumbler::parse(&file)?,
            Format::WiGLE => wigle::parse(&file)?,
        };

        for ob in obs {
            match ob {
                Observation::WiFi(x) => {
                    println!(
                        "{},{},{}",
                        x.position.latitude, x.position.longitude, x.read_key
                    )
                }
                _ => (),
            }
        }
    }

    Ok(())
}
