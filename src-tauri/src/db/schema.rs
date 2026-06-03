use rusqlite::Connection;

pub fn create_tables(conn: &Connection) -> rusqlite::Result<()> {
    conn.execute_batch(
        "
        CREATE TABLE IF NOT EXISTS metadata (
            key   TEXT PRIMARY KEY,
            value TEXT NOT NULL
        );

        CREATE TABLE IF NOT EXISTS tiles (
            tx      INTEGER NOT NULL,
            ty      INTEGER NOT NULL,
            lod     INTEGER NOT NULL DEFAULT 0,
            version INTEGER NOT NULL DEFAULT 0,
            data    BLOB NOT NULL,
            PRIMARY KEY (tx, ty, lod)
        );

        CREATE TABLE IF NOT EXISTS objects (
            kind TEXT NOT NULL,
            id   TEXT NOT NULL,
            data BLOB NOT NULL,
            PRIMARY KEY (kind, id)
        );

        CREATE TABLE IF NOT EXISTS sim_state (
            key  TEXT PRIMARY KEY,
            data BLOB NOT NULL
        );

        CREATE TABLE IF NOT EXISTS undo_journal (
            id         INTEGER PRIMARY KEY AUTOINCREMENT,
            label      TEXT NOT NULL,
            timestamp  INTEGER NOT NULL,
            tile_diffs BLOB NOT NULL
        );

        CREATE INDEX IF NOT EXISTS idx_tiles_lod ON tiles(lod);
        CREATE INDEX IF NOT EXISTS idx_undo_ts ON undo_journal(timestamp);
        ",
    )
}
