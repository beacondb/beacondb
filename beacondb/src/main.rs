use anyhow::Result;
use rusqlite::Connection;

mod mls;

fn main() -> Result<()> {
    let mut conn = Connection::open("./beacon.db")?;
    conn.execute_batch(include_str!("../db.sql"))?;

    mls::main(&mut conn)?;

    Ok(())
}
