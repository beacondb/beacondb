#[macro_use]
extern crate log;

use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use clap::Parser;
use env_logger::Env;
use libwifi::Addresses;
use pcap::{Capture, Packet};
use radiotap::{
    field::{AntennaSignal, Field},
    Radiotap, RadiotapIterator,
};
use serde::{Deserialize, Serialize};

mod gps;

pub fn get_signal(input: &[u8]) -> Result<Option<i8>> {
    let fields = RadiotapIterator::from_bytes(&input)?;
    for field in fields {
        let (kind, data) = field?;
        match kind {
            radiotap::field::Kind::AntennaSignal => {
                let v = AntennaSignal::from_bytes(&data)?.value;
                if v != 0 {
                    return Ok(Some(v));
                }
            }
            _ => (),
        }
    }

    Ok(None)
}

fn handle_packet(packet: Packet) -> Result<RawBeacon> {
    let signal = get_signal(&packet.data)?.context("Missing signal")?;

    let radiotap = Radiotap::from_bytes(&packet.data)?;
    let payload = &packet.data[radiotap.header.length..];
    let frame = libwifi::parse_frame(payload)?;
    let src = frame.src().context("Missing source address")?.clone();
    if !src.is_real_device() {
        bail!("Not a real device");
    }

    let beacon = match frame {
        libwifi::Frame::Beacon(x) => x,
        _ => bail!("not a beacon"),
    };

    let (_, channel) = beacon
        .station_info
        .data
        .iter()
        .find(|(k, v)| *k == 0x3 && v.len() == 1)
        .context("missing channel")?;
    // todo: validation, 5ghz
    let channel = channel[0];

    let timestamp_ms = packet.header.ts.tv_sec * 1000 + packet.header.ts.tv_usec / 1000;
    let timestamp_ms = timestamp_ms.try_into()?;

    Ok(RawBeacon {
        channel,
        frequency: radiotap.channel.context("Missing channel")?.freq,
        mac: src.to_string(),
        signal,
        ssid: beacon.station_info.ssid,
        timestamp_ms,
    })
}

#[derive(Debug, Deserialize, Serialize)]
struct RawBeacon {
    pub channel: u8,
    pub frequency: u16,
    pub mac: String,
    pub signal: i8,
    pub ssid: Option<String>,
    pub timestamp_ms: u64,
}

#[derive(Debug, Deserialize, Serialize)]
struct Beacon {
    pub timestamp_ms: u64,
    pub lat: f64,
    pub lon: f64,
    pub channel: u8,
    pub rx_freq: u16,
    pub rss: i8,
    pub bssid: String,
    pub ssid: Option<String>,
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Args {
    path: PathBuf,
}

fn main() -> Result<()> {
    env_logger::init_from_env(Env::new().default_filter_or("info"));

    let args = Args::parse();

    let gps = gps::load(&*args.path.with_extension("csv"))?;
    let mut cap = Capture::from_file(args.path)?;

    let mut gps = gps.into_iter();
    let mut prev = gps.next().unwrap();
    let mut next_gps = gps.next();
    let mut w = csv::Writer::from_path("x.csv")?;
    loop {
        let p = match cap.next_packet() {
            Ok(x) => x,
            Err(pcap::Error::NoMorePackets) => return Ok(()),
            Err(e) => bail!(e),
        };
        if let Ok(p) = handle_packet(p) {
            loop {
                if prev.timestamp_ms < p.timestamp_ms {
                    if let Some(next) = next_gps {
                        prev = next;
                        next_gps = gps.next();
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            }

            let (lat, lon) = if let Some(next) = &next_gps {
                // interpolate
                let window = next.timestamp_ms - prev.timestamp_ms;
                let point = next.timestamp_ms - p.timestamp_ms;
                let x = point as f64 / window as f64;
                let (lat_d, lon_d) = (next.lat - prev.lat, next.lon - prev.lon);
                (prev.lat + (lat_d * x), prev.lon + (lon_d * x))
            } else {
                (prev.lat, prev.lon)
            };

            let beacon = Beacon {
                lat,
                lon,
                channel: p.channel,
                rx_freq: p.frequency,
                bssid: p.mac,
                rss: p.signal,
                ssid: p.ssid,
                timestamp_ms: p.timestamp_ms,
            };
            w.serialize(&beacon)?;
        }
    }
}
