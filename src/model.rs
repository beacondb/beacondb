use mac_address::MacAddress;
use serde::Deserialize;
use sqlx::{query_as, PgPool};

use crate::bounds::Bounds;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Transmitter {
    Cell {
        radio: CellRadio,
        // all integers are stored as signed in postgres
        country: i16,
        network: i16,
        area: i32,
        cell: i64,
        unit: i16,
    },
    Wifi {
        mac: MacAddress,
    },
    Bluetooth {
        mac: MacAddress,
    },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Deserialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[repr(i16)]
pub enum CellRadio {
    Gsm = 2,
    Wcdma = 3,
    Lte = 4,
    Nr = 5,
}

impl Transmitter {
    pub async fn lookup(&self, pool: &PgPool) -> sqlx::Result<Option<Bounds>> {
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
                    "select min_lat, min_lon, max_lat, max_lon from cell where radio = $1 and country = $2 and network = $3 and area = $4 and cell = $5 and unit = $6",
                    *radio as i16, country, network, area, cell, unit
                ).fetch_optional(pool).await?
            }
            Transmitter::Wifi { mac } => {
                query_as!(
                    Bounds,
                    "select min_lat, min_lon, max_lat, max_lon from wifi where mac = $1",
                    mac
                )
                .fetch_optional(pool)
                .await?
            }
            Transmitter::Bluetooth { mac } => {
                query_as!(
                    Bounds,
                    "select min_lat, min_lon, max_lat, max_lon from wifi where mac = $1",
                    mac
                )
                .fetch_optional(pool)
                .await?
            }
        };

        Ok(bounds)
    }
}
