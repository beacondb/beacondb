use std::collections::BTreeMap;

use mac_address::MacAddress;
use sqlx::{query, PgPool};

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub struct Observation {
    pub beacon: Beacon,
    pub locality: Locality,
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum Beacon {
    Wifi { bssid: MacAddress, ssid: String },
    Bluetooth { mac: MacAddress, name: String },
}

#[derive(Debug, PartialEq, Eq, PartialOrd, Ord, Clone, Copy)]
pub struct Locality {
    latitude: i32,
    longitude: i32,
}

impl Locality {
    pub fn new(latitude: f32, longitude: f32) -> Self {
        let latitude = (latitude * 1000.0).round() as i32;
        let longitude = (longitude * 1000.0).round() as i32;
        Self {
            latitude,
            longitude,
        }
    }
}

#[derive(Debug)]
pub struct ObservationHelper {
    date_last_seen: BTreeMap<Observation, i32>,
}

impl ObservationHelper {
    pub fn new() -> ObservationHelper {
        Self {
            date_last_seen: BTreeMap::new(),
        }
    }

    pub fn add(&mut self, observation: Observation, date: i32) {
        if let Some(x) = self.date_last_seen.get(&observation) {
            if *x >= date {
                return;
            }
        }
        self.date_last_seen.insert(observation, date);
    }

    pub async fn commit(self, pool: &PgPool) -> sqlx::Result<()> {
        let tx = pool.begin().await?;
        for (Observation { beacon, locality }, date) in self.date_last_seen {
            match beacon {
                Beacon::Wifi { bssid, ssid } => {
                    let row = query!("select date_last_seen, days_seen from wifi_grid where bssid = $1 and ssid = $2 and latitude = $3 and longitude = $4",  bssid, ssid, locality.latitude, locality.longitude).fetch_optional(pool).await?;

                    if let Some(x) = row {
                        if x.date_last_seen >= date {
                            continue;
                        } else {
                            query!("update wifi_grid set date_last_seen = $1, days_seen = $2 where bssid = $3 and ssid = $4 and latitude = $5 and longitude = $6", date, x.days_seen + 1, bssid, ssid, locality.latitude, locality.longitude).execute(pool).await?;
                        }
                    } else {
                        query!("insert into wifi_grid (bssid, ssid, latitude, longitude, date_first_seen, date_last_seen, days_seen) values ($1, $2, $3, $4, $5, $6, $7)", bssid, ssid, locality.latitude, locality.longitude, date, date, 1).execute(pool).await?;
                    }
                }
                // Beacon::Bluetooth { mac, name } => {
                //     let row = query!("select date_last_seen, days_seen from bluetooth_grid where mac = $1 and name = $2 and latitude = $3 and longitude = $4",  mac , name, locality.latitude, locality.longitude).fetch_optional(&pool).await?;
                // }
                _ => (),
            }
        }
        tx.commit().await?;

        Ok(())
    }
}
