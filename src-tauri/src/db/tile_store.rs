use rusqlite::{Connection, params};
use crate::tile::cell::TileData;
use crate::tile::coords::TILE_SIZE;

pub fn save_tile(conn: &Connection, tx: i32, ty: i32, lod: i32, data: &TileData) -> rusqlite::Result<()> {
    save_tile_blob(conn, tx, ty, lod, &data.compress())
}

/// Write an already-compressed tile blob (compression can then run on worker
/// threads, off the DB lock, and bulk writers can wrap many of these in one
/// transaction).
pub fn save_tile_blob(conn: &Connection, tx: i32, ty: i32, lod: i32, blob: &[u8]) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT OR REPLACE INTO tiles (tx, ty, lod, version, data)
         VALUES (?1, ?2, ?3, COALESCE((SELECT version FROM tiles WHERE tx=?1 AND ty=?2 AND lod=?3), 0) + 1, ?4)",
    )?;
    stmt.execute(params![tx, ty, lod, blob])?;
    Ok(())
}

pub fn load_tile(conn: &Connection, tx: i32, ty: i32, lod: i32) -> rusqlite::Result<Option<TileData>> {
    let mut stmt = conn.prepare_cached(
        "SELECT data FROM tiles WHERE tx = ?1 AND ty = ?2 AND lod = ?3"
    )?;
    let mut rows = stmt.query(params![tx, ty, lod])?;
    match rows.next()? {
        Some(row) => {
            let blob: Vec<u8> = row.get(0)?;
            Ok(Some(TileData::decompress(&blob)))
        }
        None => Ok(None),
    }
}

/// Fetch a tile's compressed blob and version in a single (cached) query. Used by
/// the tile-serving path so each tile costs one statement, not two. Returns the
/// raw blob (decompression is deferred so callers can parallelize it off-lock).
pub fn load_blob_with_version(
    conn: &Connection, tx: i32, ty: i32, lod: i32,
) -> rusqlite::Result<Option<(i64, Vec<u8>)>> {
    let mut stmt = conn.prepare_cached(
        "SELECT version, data FROM tiles WHERE tx = ?1 AND ty = ?2 AND lod = ?3"
    )?;
    let mut rows = stmt.query(params![tx, ty, lod])?;
    match rows.next()? {
        Some(row) => Ok(Some((row.get(0)?, row.get(1)?))),
        None => Ok(None),
    }
}

pub fn get_tile_version(conn: &Connection, tx: i32, ty: i32, lod: i32) -> rusqlite::Result<i64> {
    let mut stmt = conn.prepare(
        "SELECT version FROM tiles WHERE tx = ?1 AND ty = ?2 AND lod = ?3"
    )?;
    let mut rows = stmt.query(params![tx, ty, lod])?;
    match rows.next()? {
        Some(row) => Ok(row.get(0)?),
        None => Ok(0),
    }
}

/// Initialize all tiles for a new world as empty sea
pub fn init_tiles(conn: &Connection, grid_width: u32, grid_height: u32) -> rusqlite::Result<()> {
    let tiles_x = (grid_width + TILE_SIZE - 1) / TILE_SIZE;
    let tiles_y = (grid_height + TILE_SIZE - 1) / TILE_SIZE;

    let empty = TileData::new_sea();
    let blob = empty.compress();

    let tx_obj = conn.unchecked_transaction()?;
    {
        let mut stmt = tx_obj.prepare(
            "INSERT INTO tiles (tx, ty, lod, version, data) VALUES (?1, ?2, 0, 1, ?3)"
        )?;
        for ty in 0..tiles_y as i32 {
            for tx in 0..tiles_x as i32 {
                stmt.execute(params![tx, ty, &blob])?;
            }
        }
    }
    tx_obj.commit()?;
    Ok(())
}
