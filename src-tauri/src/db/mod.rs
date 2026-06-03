pub mod schema;
pub mod tile_store;
pub mod metadata;

use rusqlite::Connection;
use std::sync::Mutex;

pub struct WorldDb {
    pub conn: Mutex<Connection>,
}

impl WorldDb {
    pub fn new(conn: Connection) -> Self {
        Self { conn: Mutex::new(conn) }
    }

    pub fn open_or_create(path: &str) -> Result<Self, String> {
        let conn = Connection::open(path).map_err(|e| e.to_string())?;
        conn.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=NORMAL;")
            .map_err(|e| e.to_string())?;
        schema::create_tables(&conn).map_err(|e| e.to_string())?;
        Ok(Self::new(conn))
    }

    pub fn in_memory() -> Result<Self, String> {
        let conn = Connection::open_in_memory().map_err(|e| e.to_string())?;
        schema::create_tables(&conn).map_err(|e| e.to_string())?;
        Ok(Self::new(conn))
    }
}
