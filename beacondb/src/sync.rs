use std::borrow::BorrowMut;

use anyhow::{bail, Result};
use libbeacondb::model::{Cell, RadioType, Wifi};

pub fn run() -> Result<()> {
    let internal = crate::db::internal()?;
    let mut public = crate::db::public()?;

    let version: u32 = public.pragma_query_value(None, "user_version", |x| x.get(0))?;
    let tx = public.transaction()?;
    {
        let mut insert = tx.prepare("insert into cell (radio, country, network, area, cell, unit, x, y, r) values (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)")?;
        let mut stmt = internal.prepare("select radio, country, network, area, cell, unit, x, y, r from cell where last_seen >= ?1")?;
        let cells = stmt.query_map((version,), |row| {
            let radio: u8 = row.get(0)?;
            let radio = match radio {
                0 => RadioType::Gsm,
                1 => RadioType::Umts,
                2 => RadioType::Lte,
                _ => panic!("unknown radio type"), // TODO
            };

            Ok(Cell {
                radio,
                country: row.get(1)?,
                network: row.get(2)?,
                area: row.get(3)?,
                cell: row.get(4)?,
                unit: row.get(5)?,
                x: row.get(6)?,
                y: row.get(7)?,
                r: row.get(8)?,
            })
        })?;

        for row in cells {
            let cell = row?;
            insert.execute((
                cell.radio as u8,
                cell.country,
                cell.network,
                cell.area,
                cell.cell,
                cell.unit,
                cell.x,
                cell.y,
                cell.r,
            ))?;
        }

        let mut insert = tx.prepare("insert into wifi (key, x, y, r) values (?1, ?2, ?3, ?4)")?;
        let mut stmt = internal.prepare("select key, x, y, r from wifi where last_seen >= ?1")?;
        let wifis = stmt.query_map((version,), |row| {
            Ok(Wifi {
                key: row.get(0)?,
                x: row.get(1)?,
                y: row.get(2)?,
                r: row.get(3)?,
            })
        })?;

        for row in wifis {
            let wifi = row?;
            insert.execute((wifi.key, wifi.x, wifi.y, wifi.r))?;
        }
    }
    tx.commit()?;

    Ok(())
}
