use rusqlite::Connection;

pub fn set_meta(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

pub fn get_meta(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM metadata WHERE key = ?1")?;
    let mut rows = stmt.query(rusqlite::params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

pub fn get_meta_required(conn: &Connection, key: &str) -> rusqlite::Result<String> {
    get_meta(conn, key)?.ok_or_else(|| {
        rusqlite::Error::QueryReturnedNoRows
    })
}

// ── Campaign-scoped key/value store (the `campaign` table) ──

pub fn campaign_set(conn: &Connection, key: &str, value: &str) -> rusqlite::Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO campaign (key, value) VALUES (?1, ?2)",
        rusqlite::params![key, value],
    )?;
    Ok(())
}

pub fn campaign_get(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    let mut stmt = conn.prepare("SELECT value FROM campaign WHERE key = ?1")?;
    let mut rows = stmt.query(rusqlite::params![key])?;
    match rows.next()? {
        Some(row) => Ok(Some(row.get(0)?)),
        None => Ok(None),
    }
}

/// Read a campaign key, falling back to the same key in `metadata` (where
/// pre-split saves stored settlements/economy).
pub fn campaign_get_or_meta(conn: &Connection, key: &str) -> rusqlite::Result<Option<String>> {
    if let Some(v) = campaign_get(conn, key)? {
        return Ok(Some(v));
    }
    get_meta(conn, key)
}
