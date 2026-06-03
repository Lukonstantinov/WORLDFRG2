/// Returns a list of (wx, wy) cells within a circle brush
pub fn circle_brush(cx: i32, cy: i32, radius: i32, grid_width: u32, grid_height: u32) -> Vec<(u32, u32)> {
    let mut cells = Vec::new();
    let r2 = (radius * radius) as i64;

    for dy in -radius..=radius {
        for dx in -radius..=radius {
            if (dx as i64 * dx as i64 + dy as i64 * dy as i64) > r2 {
                continue;
            }
            let wx = cx + dx;
            let wy = cy + dy;
            // Wrap X for east-west cylinder, clamp Y
            if wy < 0 || wy >= grid_height as i32 {
                continue;
            }
            let wx = ((wx % grid_width as i32) + grid_width as i32) as u32 % grid_width;
            cells.push((wx, wy as u32));
        }
    }

    cells
}
