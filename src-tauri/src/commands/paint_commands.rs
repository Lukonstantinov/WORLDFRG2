use tauri::State;
use crate::db::WorldDb;
use crate::paint::stroke::{self, PaintValue};
use crate::history::undo;

#[tauri::command]
pub fn paint_stroke(
    cells: Vec<(u32, u32)>,
    value: PaintValue,
    db: State<'_, WorldDb>,
) -> Result<Vec<(i32, i32)>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    stroke::apply_stroke(&conn, &cells, &value)
}

#[tauri::command]
pub fn undo(db: State<'_, WorldDb>) -> Result<Option<Vec<(i32, i32)>>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    undo::undo(&conn)
}

#[tauri::command]
pub fn redo(db: State<'_, WorldDb>) -> Result<Option<Vec<(i32, i32)>>, String> {
    let conn = db.conn.lock().map_err(|e| e.to_string())?;
    undo::redo(&conn)
}
