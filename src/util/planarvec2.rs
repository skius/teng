/// A bounds structure with exponential growth, to amortize the cost of resizing.
///
/// Growth behavior:
/// * x and y dimensions are considered separately.
/// * Considering dimension `d` (x or y), the bounds can represent values from `-2^min_d_log2` to `2^max_d_log2`.
/// * When a value outside that range is requested, the corresponding limit (i.e., min or max), is grown to
///   (next_higher_log2.max(other_limit)). The other limit is kept the same.
pub struct ExponentialGrowingBounds {
    // The minimum x value that the bounds can represent.
    // It is computed as `-2^min_x_log2`.
    min_x_log2: u8,
    // The maximum x value that the bounds can represent.
    // It is computed as `2^max_x_log2`.
    max_x_log2: u8,
    // The minimum y value that the bounds can represent.
    // It is computed as `-2^min_y_log2`.
    min_y_log2: u8,
    // The maximum y value that the bounds can represent.
    // It is computed as `2^max_y_log2`.
    max_y_log2: u8,
}

impl ExponentialGrowingBounds {
    pub fn new() -> Self {
        Self {
            min_x_log2: 0,
            max_x_log2: 0,
            min_y_log2: 0,
            max_y_log2: 0,
        }
    }

    /// Returns the minimum x value that the bounds can represent.
    pub fn min_x(&self) -> i64 {
        -(1i64 << self.min_x_log2)
    }

    /// Returns the maximum x value that the bounds can represent.
    pub fn max_x(&self) -> i64 {
        1i64 << self.max_x_log2
    }

    /// Returns the minimum y value that the bounds can represent.
    pub fn min_y(&self) -> i64 {
        -(1i64 << self.min_y_log2)
    }

    /// Returns the maximum y value that the bounds can represent.
    pub fn max_y(&self) -> i64 {
        1i64 << self.max_y_log2
    }

    fn grow_to_contain_d_limit(d: i64, limit_log2: &mut u8, other_limit_log2: u8) {
        let abs = d.abs() as u64;
        let l_zeros = abs.leading_zeros() as u8;
        let higher_log2 = 64 - l_zeros;
        if higher_log2 > *limit_log2 {
            *limit_log2 = higher_log2.max(other_limit_log2);
        }
    }

    fn grow_to_contain_d(d: i64, min_d_log2: &mut u8, max_d_log2: &mut u8) {
        if d > 0 {
            Self::grow_to_contain_d_limit(d, max_d_log2, *min_d_log2);
        } else if d < 0 {
            Self::grow_to_contain_d_limit(d, min_d_log2, *max_d_log2);
        }
    }


    /// Grows the bounds to contain the given point, applying the growth rules described in
    /// [`ExponentialGrowingBounds`].
    pub fn grow_to_contain(&mut self, (x, y): (i64, i64)) {
        Self::grow_to_contain_d(x, &mut self.min_x_log2, &mut self.max_x_log2);
        Self::grow_to_contain_d(y, &mut self.min_y_log2, &mut self.max_y_log2);
    }
}