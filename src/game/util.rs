pub fn for_coord_in_line((start_x, start_y): (i64, i64), (end_x, end_y): (i64, i64), mut f: impl FnMut(i64, i64)) {
    let dx = (end_x - start_x).abs();
    let dy = (end_y - start_y).abs();
    let sx = if start_x < end_x { 1 } else { -1 };
    let sy = if start_y < end_y { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = start_x;
    let mut y = start_y;
    loop {
        f(x, y);
        if x == end_x && y == end_y {
            break;
        }
        let e2 = 2 * err;
        if e2 > -dy {
            err -= dy;
            x += sx;
        }
        if e2 < dx {
            err += dx;
            y += sy;
        }
    }
}