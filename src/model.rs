use serde::Deserialize;
use sqlx::{query_as, MySqlPool};

use crate::bounds::Bounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Transmitter {
    Cell {
        radio: CellRadio,
        country: u16,
        network: u16,
        area: u32, // u24 in db
        cell: u64,
        unit: u16,
    },
    Wifi {
        mac: [u8; 6],
    },
    Bluetooth {
        mac: [u8; 6],
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
pub enum CellRadio {
    Gsm,
    Wcdma,
    Lte,
    Nr,
}

impl Transmitter {
    pub async fn lookup(&self, pool: &MySqlPool) -> sqlx::Result<Option<Bounds>> {
        let bounds = match self {
            Transmitter::Cell {
                radio,
                country,
                network,
                area,
                cell,
                unit,
            } => {
                query_as!(
                    Bounds,
                    "select min_lat, min_lon, max_lat, max_lon from cell where radio = ? and country = ? and network = ? and area = ? and cell = ? and unit = ?",
                    radio,country,network,area,cell,unit

                ).fetch_optional(pool).await?
            }
            Transmitter::Wifi { mac } => {
                query_as!(
                    Bounds,
                    "select min_lat, min_lon, max_lat, max_lon from wifi where mac = ?",
                    &mac[..]
                )
                .fetch_optional(pool)
                .await?
            }
            Transmitter::Bluetooth { mac } => {
                query_as!(
                    Bounds,
                    "select min_lat, min_lon, max_lat, max_lon from wifi where mac = ?",
                    &mac[..]
                )
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(bounds)
    }
}
