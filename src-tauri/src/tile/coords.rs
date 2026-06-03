pub const TILE_SIZE: u32 = 128;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub struct TileCoord {
    pub tx: i32,
    pub ty: i32,
}

impl TileCoord {
    pub fn from_world(wx: u32, wy: u32) -> Self {
        Self {
            tx: (wx / TILE_SIZE) as i32,
            ty: (wy / TILE_SIZE) as i32,
        }
    }

    /// Local cell coordinates within the tile
    pub fn local(wx: u32, wy: u32) -> (u32, u32) {
        (wx % TILE_SIZE, wy % TILE_SIZE)
    }

    /// Convert tile coord + local offset back to world coordinates
    pub fn to_world(&self, lx: u32, ly: u32) -> (u32, u32) {
        (self.tx as u32 * TILE_SIZE + lx, self.ty as u32 * TILE_SIZE + ly)
    }
}

/// How many tiles span the grid in each dimension
pub fn tile_count(grid_width: u32, grid_height: u32) -> (u32, u32) {
    (
        (grid_width + TILE_SIZE - 1) / TILE_SIZE,
        (grid_height + TILE_SIZE - 1) / TILE_SIZE,
    )
}

/// How many cells are valid in a given tile (handles edge tiles)
pub fn tile_cell_count(tx: u32, ty: u32, grid_width: u32, grid_height: u32) -> (u32, u32) {
    let start_x = tx * TILE_SIZE;
    let start_y = ty * TILE_SIZE;
    let w = TILE_SIZE.min(grid_width.saturating_sub(start_x));
    let h = TILE_SIZE.min(grid_height.saturating_sub(start_y));
    (w, h)
}
