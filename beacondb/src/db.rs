use anyhow::Result;
use rusqlite::Connection;
use sqlx::PgPool;

pub fn internal() -> Result<Connection> {
    let conn = Connection::open("./internal.db")?;
    conn.execute_batch(include_str!("../db-internal.sql"))?;
    Ok(conn)
}

pub fn public() -> Result<Connection> {
    let conn = Connection::open("./public.db")?;
    conn.execute_batch(include_str!("../db-public.sql"))?;
    Ok(conn)
}

pub async fn parallel() -> Result<PgPool> {
    let pool = PgPool::connect(&dotenvy::var("DATABASE_URL")?).await?;
    sqlx::migrate!().run(&pool).await?;
    Ok(pool)
}
