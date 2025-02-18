//! Common utility functions.

/// Runs a function for each coordinate in a line.
/// If `exclude_start` is true, the start coordinate will not be included, unless it's the same as the end coordinate.
pub fn for_coord_in_line(
    exclude_start: bool,
    (start_x, start_y): (i64, i64),
    (end_x, end_y): (i64, i64),
    mut f: impl FnMut(i64, i64),
) {
    // only exclude start if it's not the same as end
    let exclude_start = exclude_start && (start_x != end_x || start_y != end_y);

    let dx = (end_x - start_x).abs();
    let dy = (end_y - start_y).abs();
    let sx = if start_x < end_x { 1 } else { -1 };
    let sy = if start_y < end_y { 1 } else { -1 };
    let mut err = dx - dy;
    let mut x = start_x;
    let mut y = start_y;
    loop {
        if !exclude_start || x != start_x || y != start_y {
            f(x, y);
        }
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

macro_rules! lerp_t_impl {
    ($fn_name:ident, $typ:ty) => {
        /// Get the interpolation factor `t` for a value `current` between `from` and `to`.
        #[allow(unused)]
        pub fn $fn_name(from: $typ, to: $typ, current: $typ) -> f32 {
            (current as f32 - from as f32) / (to as f32 - from as f32)
        }
    };
}

macro_rules! lerp_t_impl_clamped {
    ($fn_name:ident, $typ:ty) => {
        /// Get the interpolation factor `t` for a value `current` between `from` and `to`, clamped to the range [0, 1].
        #[allow(unused)]
        pub fn $fn_name(from: $typ, to: $typ, current: $typ) -> f32 {
            let t = (current as f32 - from as f32) / (to as f32 - from as f32);
            t.clamp(0.0, 1.0)
        }
    };
}

lerp_t_impl!(get_lerp_t_i8, i8);
lerp_t_impl!(get_lerp_t_u8, u8);
lerp_t_impl!(get_lerp_t_i16, i16);
lerp_t_impl!(get_lerp_t_u16, u16);
lerp_t_impl!(get_lerp_t_i32, i32);
lerp_t_impl!(get_lerp_t_u32, u32);
lerp_t_impl!(get_lerp_t_i64, i64);
lerp_t_impl!(get_lerp_t_u64, u64);
lerp_t_impl!(get_lerp_t_i128, i128);
lerp_t_impl!(get_lerp_t_u128, u128);
lerp_t_impl!(get_lerp_t_isize, isize);
lerp_t_impl!(get_lerp_t_usize, usize);
lerp_t_impl!(get_lerp_t_f32, f32);
lerp_t_impl!(get_lerp_t_f64, f64);

lerp_t_impl_clamped!(get_lerp_t_i8_clamped, i8);
lerp_t_impl_clamped!(get_lerp_t_u8_clamped, u8);
lerp_t_impl_clamped!(get_lerp_t_i16_clamped, i16);
lerp_t_impl_clamped!(get_lerp_t_u16_clamped, u16);
lerp_t_impl_clamped!(get_lerp_t_i32_clamped, i32);
lerp_t_impl_clamped!(get_lerp_t_u32_clamped, u32);
lerp_t_impl_clamped!(get_lerp_t_i64_clamped, i64);
lerp_t_impl_clamped!(get_lerp_t_u64_clamped, u64);
lerp_t_impl_clamped!(get_lerp_t_i128_clamped, i128);
lerp_t_impl_clamped!(get_lerp_t_u128_clamped, u128);
lerp_t_impl_clamped!(get_lerp_t_isize_clamped, isize);
lerp_t_impl_clamped!(get_lerp_t_usize_clamped, usize);
lerp_t_impl_clamped!(get_lerp_t_f32_clamped, f32);
lerp_t_impl_clamped!(get_lerp_t_f64_clamped, f64);

/// Linearly interpolate between two colors.
/// Uses RGB color space.
pub fn lerp_color(a: [u8; 3], b: [u8; 3], t: f32) -> [u8; 3] {
    [
        (a[0] as f32 + (b[0] as f32 - a[0] as f32) * t) as u8,
        (a[1] as f32 + (b[1] as f32 - a[1] as f32) * t) as u8,
        (a[2] as f32 + (b[2] as f32 - a[2] as f32) * t) as u8,
    ]
}
