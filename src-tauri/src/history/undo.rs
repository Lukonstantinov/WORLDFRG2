use rusqlite::{Connection, params};

const MAX_UNDO: i64 = 50;

/// Save the old state of modified tiles before a paint operation.
/// `tile_diffs` contains (tx, ty, compressed_old_data) for each modified tile.
pub fn push_undo(conn: &Connection, label: &str, tile_diffs: &[(i32, i32, Vec<u8>)]) -> Result<(), String> {
    if tile_diffs.is_empty() {
        return Ok(());
    }

    // When we push a new undo, discard any redo entries (entries after current position)
    // We track position via a metadata key "undo_pos"
    let pos = get_undo_pos(conn);
    conn.execute(
        "DELETE FROM undo_journal WHERE id > ?1",
        params![pos],
    ).map_err(|e| e.to_string())?;

    // Serialize tile diffs: count(u32) + [tx(i32) + ty(i32) + len(u32) + data]...
    let mut blob = Vec::new();
    let count = tile_diffs.len() as u32;
    blob.extend_from_slice(&count.to_le_bytes());
    for (tx, ty, data) in tile_diffs {
        blob.extend_from_slice(&tx.to_le_bytes());
        blob.extend_from_slice(&ty.to_le_bytes());
        let len = data.len() as u32;
        blob.extend_from_slice(&len.to_le_bytes());
        blob.extend_from_slice(data);
    }

    let compressed = zstd::encode_all(blob.as_slice(), 3).unwrap_or(blob);

    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    conn.execute(
        "INSERT INTO undo_journal (label, timestamp, tile_diffs) VALUES (?1, ?2, ?3)",
        params![label, now, compressed],
    ).map_err(|e| e.to_string())?;

    let new_id: i64 = conn.last_insert_rowid();
    set_undo_pos(conn, new_id);

    // Trim old entries beyond MAX_UNDO
    conn.execute(
        "DELETE FROM undo_journal WHERE id <= (SELECT MAX(id) FROM undo_journal) - ?1",
        params![MAX_UNDO],
    ).map_err(|e| e.to_string())?;

    Ok(())
}

/// Undo the last operation: restore old tile states, save current states for redo.
pub fn undo(conn: &Connection) -> Result<Option<Vec<(i32, i32)>>, String> {
    let pos = get_undo_pos(conn);
    if pos == 0 {
        return Ok(None);
    }

    // Load the undo entry at current position
    let entry = load_entry(conn, pos)?;
    let entry = match entry {
        Some(e) => e,
        None => return Ok(None),
    };

    let old_tiles = deserialize_diffs(&entry.tile_diffs)?;

    // Before restoring, save the CURRENT tile states into this entry
    // so we can redo later
    let mut current_states: Vec<(i32, i32, Vec<u8>)> = Vec::new();
    for (tx, ty, _) in &old_tiles {
        let current = crate::db::tile_store::load_tile(conn, *tx, *ty, 0)
            .map_err(|e| e.to_string())?
            .unwrap_or_else(crate::tile::cell::TileData::new_sea);
        current_states.push((*tx, *ty, current.compress()));
    }

    // Serialize current states and update the entry for redo
    let redo_blob = serialize_diffs(&current_states);
    let redo_compressed = zstd::encode_all(redo_blob.as_slice(), 3).unwrap_or(redo_blob);

    // Store redo data as a separate column would be ideal, but we'll use a trick:
    // we store redo data in a new undo_journal entry right after pos with label "redo:..."
    // Actually, simpler: update the existing entry's tile_diffs to be the current (redo) data,
    // and restore the old data to the tiles.

    // Restore old tiles
    let mut modified = Vec::new();
    for (tx, ty, data) in &old_tiles {
        let tile = crate::tile::cell::TileData::decompress(data);
        crate::db::tile_store::save_tile(conn, *tx, *ty, 0, &tile)
            .map_err(|e| e.to_string())?;
        modified.push((*tx, *ty));
    }

    // Replace entry's diffs with current state (for redo)
    conn.execute(
        "UPDATE undo_journal SET tile_diffs = ?1 WHERE id = ?2",
        params![redo_compressed, pos],
    ).map_err(|e| e.to_string())?;

    // Move position back
    let prev_pos = get_prev_id(conn, pos);
    set_undo_pos(conn, prev_pos);

    Ok(Some(modified))
}

/// Redo: re-apply the undone operation.
pub fn redo(conn: &Connection) -> Result<Option<Vec<(i32, i32)>>, String> {
    let pos = get_undo_pos(conn);

    // Find the next entry after current position
    let next_id = get_next_id(conn, pos);
    if next_id == 0 {
        return Ok(None);
    }

    let entry = load_entry(conn, next_id)?;
    let entry = match entry {
        Some(e) => e,
        None => return Ok(None),
    };

    // The entry now contains the "current" (redo) data from when we undid
    let redo_tiles = deserialize_diffs(&entry.tile_diffs)?;

    // Save current state (which is the old/undone state) back into the entry
    let mut current_states: Vec<(i32, i32, Vec<u8>)> = Vec::new();
    for (tx, ty, _) in &redo_tiles {
        let current = crate::db::tile_store::load_tile(conn, *tx, *ty, 0)
            .map_err(|e| e.to_string())?
            .unwrap_or_else(crate::tile::cell::TileData::new_sea);
        current_states.push((*tx, *ty, current.compress()));
    }

    // Restore the redo tiles
    let mut modified = Vec::new();
    for (tx, ty, data) in &redo_tiles {
        let tile = crate::tile::cell::TileData::decompress(data);
        crate::db::tile_store::save_tile(conn, *tx, *ty, 0, &tile)
            .map_err(|e| e.to_string())?;
        modified.push((*tx, *ty));
    }

    // Store the old (undo) data back
    let undo_blob = serialize_diffs(&current_states);
    let undo_compressed = zstd::encode_all(undo_blob.as_slice(), 3).unwrap_or(undo_blob);
    conn.execute(
        "UPDATE undo_journal SET tile_diffs = ?1 WHERE id = ?2",
        params![undo_compressed, next_id],
    ).map_err(|e| e.to_string())?;

    // Move position forward
    set_undo_pos(conn, next_id);

    Ok(Some(modified))
}

// --- helpers ---

struct UndoEntry {
    tile_diffs: Vec<u8>,
}

fn load_entry(conn: &Connection, id: i64) -> Result<Option<UndoEntry>, String> {
    let mut stmt = conn.prepare("SELECT tile_diffs FROM undo_journal WHERE id = ?1")
        .map_err(|e| e.to_string())?;
    let mut rows = stmt.query(params![id]).map_err(|e| e.to_string())?;
    match rows.next().map_err(|e| e.to_string())? {
        Some(row) => {
            let blob: Vec<u8> = row.get(0).map_err(|e| e.to_string())?;
            let decompressed = zstd::decode_all(blob.as_slice()).unwrap_or(blob);
            Ok(Some(UndoEntry { tile_diffs: decompressed }))
        }
        None => Ok(None),
    }
}

fn serialize_diffs(diffs: &[(i32, i32, Vec<u8>)]) -> Vec<u8> {
    let mut blob = Vec::new();
    let count = diffs.len() as u32;
    blob.extend_from_slice(&count.to_le_bytes());
    for (tx, ty, data) in diffs {
        blob.extend_from_slice(&tx.to_le_bytes());
        blob.extend_from_slice(&ty.to_le_bytes());
        let len = data.len() as u32;
        blob.extend_from_slice(&len.to_le_bytes());
        blob.extend_from_slice(data);
    }
    blob
}

fn deserialize_diffs(data: &[u8]) -> Result<Vec<(i32, i32, Vec<u8>)>, String> {
    if data.len() < 4 {
        return Ok(Vec::new());
    }
    let count = u32::from_le_bytes(data[0..4].try_into().unwrap()) as usize;
    let mut offset = 4;
    let mut result = Vec::with_capacity(count);
    for _ in 0..count {
        if offset + 12 > data.len() { break; }
        let tx = i32::from_le_bytes(data[offset..offset+4].try_into().unwrap());
        let ty = i32::from_le_bytes(data[offset+4..offset+8].try_into().unwrap());
        let len = u32::from_le_bytes(data[offset+8..offset+12].try_into().unwrap()) as usize;
        offset += 12;
        if offset + len > data.len() { break; }
        let tile_data = data[offset..offset+len].to_vec();
        offset += len;
        result.push((tx, ty, tile_data));
    }
    Ok(result)
}

fn get_undo_pos(conn: &Connection) -> i64 {
    conn.query_row(
        "SELECT CAST(value AS INTEGER) FROM metadata WHERE key = 'undo_pos'",
        [],
        |row| row.get(0),
    ).unwrap_or(0)
}

fn set_undo_pos(conn: &Connection, pos: i64) {
    let _ = conn.execute(
        "INSERT OR REPLACE INTO metadata (key, value) VALUES ('undo_pos', ?1)",
        params![pos.to_string()],
    );
}

fn get_prev_id(conn: &Connection, current: i64) -> i64 {
    conn.query_row(
        "SELECT MAX(id) FROM undo_journal WHERE id < ?1",
        params![current],
        |row| row.get::<_, Option<i64>>(0),
    ).unwrap_or(None).unwrap_or(0)
}

fn get_next_id(conn: &Connection, current: i64) -> i64 {
    conn.query_row(
        "SELECT MIN(id) FROM undo_journal WHERE id > ?1",
        params![current],
        |row| row.get::<_, Option<i64>>(0),
    ).unwrap_or(None).unwrap_or(0)
}
