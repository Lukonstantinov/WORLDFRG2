use rusqlite::{Connection, params};
use crate::tile::cell::TileData;
use crate::tile::coords::TILE_SIZE;

pub fn save_tile(conn: &Connection, tx: i32, ty: i32, lod: i32, data: &TileData) -> rusqlite::Result<()> {
    save_tile_blob(conn, tx, ty, lod, &data.compress())
}

/// Write an already-compressed tile blob (compression can then run on worker
/// threads, off the DB lock, and bulk writers can wrap many of these in one
/// transaction). A base-tile (lod 0) write drops the downsampled LOD-pyramid
/// entries covering it, so every write path — paint, undo/redo, template
/// import, whole-world sim saves — keeps the pyramid coherent for free.
pub fn save_tile_blob(conn: &Connection, tx: i32, ty: i32, lod: i32, blob: &[u8]) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT OR REPLACE INTO tiles (tx, ty, lod, version, data)
         VALUES (?1, ?2, ?3, COALESCE((SELECT version FROM tiles WHERE tx=?1 AND ty=?2 AND lod=?3), 0) + 1, ?4)",
    )?;
    stmt.execute(params![tx, ty, lod, blob])?;
    if lod == 0 {
        invalidate_lods_for(conn, tx, ty)?;
    }
    Ok(())
}

/// Persist a downsampled LOD-pyramid entry with an explicit version (the max
/// version of the base tiles it was sampled from), so cached and freshly built
/// supertiles report the same version.
pub fn save_lod_blob(
    conn: &Connection, tx: i32, ty: i32, lod: i32, version: i64, blob: &[u8],
) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare_cached(
        "INSERT OR REPLACE INTO tiles (tx, ty, lod, version, data) VALUES (?1, ?2, ?3, ?4, ?5)",
    )?;
    stmt.execute(params![tx, ty, lod, version, blob])?;
    Ok(())
}

/// Drop the LOD-pyramid entries (lod 1..=4) covering base tile (tx, ty).
fn invalidate_lods_for(conn: &Connection, tx: i32, ty: i32) -> rusqlite::Result<()> {
    let mut stmt = conn.prepare_cached("DELETE FROM tiles WHERE lod = ?1 AND tx = ?2 AND ty = ?3")?;
    for l in 1..=4i32 {
        stmt.execute(params![l, tx >> l, ty >> l])?;
    }
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::db::schema;

    /// Writing a base tile must drop every LOD-pyramid entry covering it.
    #[test]
    fn lod_pyramid_invalidates_on_base_write() {
        let conn = Connection::open_in_memory().unwrap();
        schema::create_tables(&conn).unwrap();

        // Pyramid entries at every level covering base tile (17, 5).
        for l in 1..=4i32 {
            save_lod_blob(&conn, 17 >> l, 5 >> l, l, 9, &[1, 2, 3]).unwrap();
        }
        // Plus one that does NOT cover it (far away at lod 2).
        save_lod_blob(&conn, 40, 40, 2, 9, &[1, 2, 3]).unwrap();

        save_tile_blob(&conn, 17, 5, 0, &TileData::new_sea().compress()).unwrap();

        let lod_rows: i64 = conn
            .query_row("SELECT COUNT(*) FROM tiles WHERE lod > 0", [], |r| r.get(0))
            .unwrap();
        assert_eq!(lod_rows, 1, "only the non-covering entry survives");
        let survivor: i64 = conn
            .query_row("SELECT COUNT(*) FROM tiles WHERE lod = 2 AND tx = 40 AND ty = 40", [], |r| r.get(0))
            .unwrap();
        assert_eq!(survivor, 1);
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
