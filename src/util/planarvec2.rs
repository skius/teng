pub use crate::util::planarvec::Bounds;
use std::ops::{Index, IndexMut};

/// A bounds structure with exponential growth, to amortize the cost of resizing.
///
/// Growth behavior:
/// * x and y dimensions are considered separately.
/// * Considering dimension `d` (x or y), the bounds can represent values from `-2^min_d_log2` to `2^max_d_log2`.
/// * When a value outside that range is requested, the corresponding limit (i.e., min or max), is grown to
///   (next_higher_log2.max(other_limit)). The other limit is kept the same.
#[derive(Copy, Clone, PartialEq, Debug, Default)]
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

    pub fn width(&self) -> usize {
        (self.max_x() - self.min_x() + 1) as usize
    }

    pub fn height(&self) -> usize {
        (self.max_y() - self.min_y() + 1) as usize
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

    pub fn contains(&self, (x, y): (i64, i64)) -> bool {
        x >= self.min_x() && x <= self.max_x() && y >= self.min_y() && y <= self.max_y()
    }

    pub fn to_bounds(&self) -> Bounds {
        Bounds {
            min_x: self.min_x(),
            max_x: self.max_x(),
            min_y: self.min_y(),
            max_y: self.max_y(),
        }
    }
}

#[derive(Debug, Clone)]
pub struct PlanarVecInner<T> {
    bounds: ExponentialGrowingBounds,
    data: Vec<T>,
    width: usize,
    height: usize,
    // the position of (0, 0) coords in the data as col, row into data
    center_x: usize,
    center_y: usize,
}

impl<T> PlanarVecInner<T> {
    pub fn new(default: T) -> PlanarVecInner<T>
    where
        T: Clone,
    {
        let bounds = ExponentialGrowingBounds::new();
        let width = bounds.width();
        let height = bounds.height();
        let data = vec![default; width * height];
        let center_x = 0 + bounds.min_x().abs() as usize;
        let center_y = 0 + bounds.max_y().abs() as usize;
        PlanarVecInner {
            bounds,
            data,
            width,
            height,
            center_x,
            center_y,
        }
    }

    // fn get_index(&self, x: i64, y: i64) -> usize {
    //     let x = x + self.center_x as i64;
    //     let y = y + self.center_y as i64;
    //     let x = x - self.bounds.min_x();
    //     let y = y - self.bounds.min_y();
    //     (y as usize) * self.width + (x as usize)
    // }

    fn get_index(&self, x: i64, y: i64) -> usize {
        let x = x + self.center_x as i64;
        // how far from the top is y:
        let y_dist_top = self.bounds.max_y() - y;
        let y = y_dist_top;
        // let y = y - self.center_y as i64;
        // let x = x - self.bounds.min_x();
        // let y = y - self.bounds.min_y();
        (y as usize) * self.width + (x as usize)
    }

    pub fn get(&self, x: i64, y: i64) -> Option<&T> {
        if !self.bounds.contains((x, y)) {
            return None;
        }
        let idx = self.get_index(x, y);
        self.data.get(idx)
    }

    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut T> {
        if !self.bounds.contains((x, y)) {
            return None;
        }
        let idx = self.get_index(x, y);
        let res = self.data.get_mut(idx);
        // if res.is_none() {
        //     panic!("get_mut failed for center_x: {}, center_y: {}, x: {}, y: {}, idx: {}, bounds: {:?}, to_bounds: {:?}, width: {}, height: {}, total: {}",
        //         self.center_x,
        //         self.center_y,
        //         x,
        //         y,
        //         idx,
        //         self.bounds,
        //         self.bounds.to_bounds(),
        //         self.width,
        //         self.height,
        //         self.width * self.height,
        //     );
        // }
        res
    }

    fn grow_to_contain_bounds(&mut self, bounds: Bounds, default: T)
    where
        T: Clone,
    {
        let mut new_exp_bounds = self.bounds;
        new_exp_bounds.grow_to_contain((bounds.min_x, bounds.min_y));
        new_exp_bounds.grow_to_contain((bounds.max_x, bounds.max_y));
        if new_exp_bounds == self.bounds {
            return;
        }
        let new_width = new_exp_bounds.width();
        let new_height = new_exp_bounds.height();
        let mut new_data = vec![default; new_width * new_height];
        let diff_x_start = (new_exp_bounds.min_x() - self.bounds.min_x()).abs();
        // let diff_y_start = (new_exp_bounds.min_y() - self.bounds.min_y()).abs();
        let diff_y_end = (new_exp_bounds.max_y() - self.bounds.max_y()).abs();
        let new_center_x = self.center_x + diff_x_start as usize;
        // TODO: center_y is unused, so unclear if calculations a re correct
        let new_center_y = self.center_y + diff_y_end as usize;
        for y in 0..self.height {
            let src_start = y * self.width;
            let dst_start = (y + diff_y_end as usize) * new_width + diff_x_start as usize;
            let src_end = src_start + self.width;
            let dst_end = dst_start + self.width;
            new_data[dst_start..dst_end].clone_from_slice(&self.data[src_start..src_end]);
        }
        self.data = new_data;
        self.bounds = new_exp_bounds;
        self.width = new_width;
        self.height = new_height;
        self.center_x = new_center_x;
        self.center_y = new_center_y;
    }
}

// pretends to be a planarvec::PlanarVec
#[derive(Debug, Clone)]
pub struct PlanarVec<T> {
    inner: PlanarVecInner<T>,
    pseudo_bounds: Bounds,
}

impl<T> PlanarVec<T> {
    /// Creates a new `PlanarVec` with the given bounds and default value.
    pub fn new(bounds: Bounds, default: T) -> Self
    where
        T: Clone,
    {
        let mut inner = PlanarVecInner::new(default.clone());
        inner.grow_to_contain_bounds(bounds, default);
        Self {
            inner,
            pseudo_bounds: bounds,
        }
    }

    fn allocated_bounds(&self) -> Bounds {
        self.inner.bounds.to_bounds()
    }

    /// Returns the world bounds
    pub fn bounds(&self) -> Bounds {
        self.pseudo_bounds
    }

    /// Returns the x range
    pub fn x_range(&self) -> impl DoubleEndedIterator<Item = i64> + use<T> {
        self.pseudo_bounds.min_x..=self.pseudo_bounds.max_x
    }

    /// Returns the y range
    pub fn y_range(&self) -> impl DoubleEndedIterator<Item = i64> + use<T> {
        self.pseudo_bounds.min_y..=self.pseudo_bounds.max_y
    }

    /// Clears the entire `PlanarVec` and sets every value to the given default.
    pub fn clear(&mut self, default: T)
    where
        T: Clone,
    {
        self.inner.data.fill(default);
    }

    /// Gets the value at the given position, if it exists.
    pub fn get(&self, x: i64, y: i64) -> Option<&T> {
        if !self.pseudo_bounds.contains(x, y) {
            return None;
        }

        self.inner.get(x, y)
    }

    /// Gets the value at the given position mutably, if it exists.
    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut T> {
        if !self.pseudo_bounds.contains(x, y) {
            return None;
        }

        self.inner.get_mut(x, y)
    }

    fn get_mut_raw(&mut self, x: i64, y: i64) -> Option<&mut T> {
        self.inner.get_mut(x, y)
    }

    /// Expands the `PlanarVec` to at least contain the given bounds.
    ///
    /// If the passed bounds are outside the current bounds, the `PlanarVec` is expanded to
    /// the union of both bounds. The new cells are filled with the given default value.
    pub fn expand(&mut self, bounds: Bounds, default: T)
    where
        T: Clone,
    {
        let union_bounds = self.pseudo_bounds.union(bounds);

        if union_bounds == self.pseudo_bounds {
            return;
        }

        if self.allocated_bounds().contains_bounds(union_bounds) {
            // don't need to grow-allocate inner, just set default values of new cells
            // what if we don't do this?
            // for x in union_bounds.min_x..=union_bounds.max_x {
            //     for y in union_bounds.min_y..=union_bounds.max_y {
            //         if !self.pseudo_bounds.contains(x, y) {
            //             *self.get_mut_raw(x, y).unwrap() = default.clone();
            //         }
            //     }
            // }
            self.pseudo_bounds = union_bounds;
            return;
        }

        self.inner
            .grow_to_contain_bounds(union_bounds, default.clone());

        self.pseudo_bounds = union_bounds;
    }

    fn info_string(&self) -> String {
        format!(
            "PlanarVec: inner.bounds: {:?}, pseudo_bounds: {:?}, computed allocated bounds: {:?}",
            self.inner.bounds,
            self.pseudo_bounds,
            self.allocated_bounds()
        )
    }
}

impl<T> Index<(i64, i64)> for PlanarVec<T> {
    type Output = T;

    fn index(&self, (x, y): (i64, i64)) -> &Self::Output {
        // self.get(x, y).expect(format!("index out of bounds: ({}, {}), info: {}", x, y, self.info_string()).as_str())
        self.get(x, y).expect("index out of bounds")
    }
}

impl<T> IndexMut<(i64, i64)> for PlanarVec<T> {
    fn index_mut(&mut self, (x, y): (i64, i64)) -> &mut Self::Output {
        // let info_str = self.info_string();
        // self.get_mut(x, y).expect(format!("indexmut out of bounds: ({}, {}), info: {}", x, y, info_str).as_str())
        self.get_mut(x, y).expect("index mut out of bounds")
    }
}
