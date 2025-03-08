//! The `planarvec` module provides data structures for working with 2D planes indexed by `i64` coordinates.
//!
//! - [`Bounds`]: Represents the boundaries of a 2D plane, defined by minimum and maximum x and y coordinates.
//! - [`PlanarVec`]: A 2D vector-like data structure capable of storing data of a growable 2D plane, efficiently growable in the x-dimension, and indexable by `(i64, i64)` tuples.

use crate::util::bidivec::BidiVec;
use std::ops::{Index, IndexMut};

/// Bounds for a 2D plane. Includes all indices.
///
/// `Bounds` defines a rectangular region in a 2D integer plane.
///
/// *   `x` values range from `min_x`..=`max_x` (inclusive)
/// *   `y` values range from `min_y`..=`max_y` (inclusive)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    /// Maximum x-coordinate of the bounds.
    pub max_x: i64,
    /// Minimum x-coordinate of the bounds.
    pub min_x: i64,
    /// Maximum y-coordinate of the bounds.
    pub max_y: i64,
    /// Minimum y-coordinate of the bounds.
    pub min_y: i64,
}

impl Default for Bounds {
    fn default() -> Self {
        Self::empty()
    }
}

impl Bounds {
    /// Creates an empty `Bounds`.
    ///
    /// An empty bounds contains no points.
    pub fn empty() -> Self {
        Self {
            min_x: 0,
            max_x: -1,
            min_y: 0,
            max_y: -1,
        }
    }

    /// Checks if `self` is empty (contains no points).
    pub fn is_empty(&self) -> bool {
        self.min_x > self.max_x || self.min_y > self.max_y
    }

    /// Checks if the given coordinates `(x, y)` are contained within `self`.
    pub fn contains(&self, x: i64, y: i64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }

    /// Checks if `self` fully contains another `Bounds`.
    pub fn contains_bounds(&self, other: Bounds) -> bool {
        if other.is_empty() {
            return true;
        }
        self.contains(other.min_x, other.min_y) && self.contains(other.max_x, other.max_y)
    }

    /// Checks if `self` intersects with another `Bounds`.
    pub fn intersects(&self, other: Bounds) -> bool {
        self.min_x <= other.max_x
            && self.max_x >= other.min_x
            && self.min_y <= other.max_y
            && self.max_y >= other.min_y
    }

    /// Returns the (at most) four bounds that can happen when the smaller `other` bounds is subtracted
    /// from the larger `self` bounds.
    ///
    /// Imagine subtracting a rectangular area (`other`) from another rectangular area (`self`).
    /// This function calculates the remaining rectangular areas (if any) after the subtraction.
    /// It can result in up to four disjoint rectangular regions. The unused bounds are empty.
    pub fn subtract(&self, other: Bounds) -> [Bounds; 4] {
        let mut bounds = [Bounds::empty(); 4];
        if other.is_empty() {
            bounds[0] = *self;
            return bounds;
        }
        if self.is_empty() {
            return bounds;
        }

        if other.min_x > self.min_x {
            bounds[0] = Bounds {
                min_x: self.min_x,
                max_x: self.max_x.min(other.min_x - 1),
                min_y: self.min_y,
                max_y: self.max_y,
            };
        }
        if other.max_x < self.max_x {
            bounds[1] = Bounds {
                min_x: self.min_x.max(other.max_x + 1),
                max_x: self.max_x,
                min_y: self.min_y,
                max_y: self.max_y,
            };
        }
        // for the top/bottom bounds, we don't want to double count the corners, so we need to
        // take into account the relative x bounds
        if other.min_y > self.min_y {
            bounds[2] = Bounds {
                min_x: self.min_x.max(other.min_x),
                max_x: self.max_x.min(other.max_x),
                min_y: self.min_y,
                max_y: self.max_y.min(other.min_y - 1),
            };
        }
        if other.max_y < self.max_y {
            bounds[3] = Bounds {
                min_x: self.min_x.max(other.min_x),
                max_x: self.max_x.min(other.max_x),
                min_y: self.min_y.max(other.max_y + 1),
                max_y: self.max_y,
            };
        }

        bounds
    }

    /// Returns the bounds containing both `self` and `other` bounds.
    pub fn union(&self, other: Bounds) -> Bounds {
        if self.is_empty() {
            return other;
        }
        if other.is_empty() {
            return *self;
        }

        Bounds {
            min_x: self.min_x.min(other.min_x),
            max_x: self.max_x.max(other.max_x),
            min_y: self.min_y.min(other.min_y),
            max_y: self.max_y.max(other.max_y),
        }
    }
}

/// A data structure capable of storing a 2D plane indexed by a pair of i64s, `(x, y)`.
///
/// `PlanarVec` efficiently stores and accesses data in a 2D grid, where coordinates can be negative.
/// It uses a nested `BidiVec` structure internally to allow for efficient growth in the x-dimension
/// and stable i64 indexing for both x and y coordinates.
///
/// Efficiently growable in the x dimension due to the ordering of the internal `BidiVec`s.
#[derive(Debug, Clone)]
pub struct PlanarVec<T> {
    // Outer index is x, inner index is y
    data: BidiVec<BidiVec<T>>,
    bounds: Bounds,
}

impl<T> Default for PlanarVec<T> {
    fn default() -> Self {
        Self {
            data: BidiVec::default(),
            bounds: Bounds::default(),
        }
    }
}

impl<T> PlanarVec<T> {
    /// Creates a new `PlanarVec` with the given bounds and default value.
    pub fn new(bounds: Bounds, default: T) -> Self
    where
        T: Clone,
    {
        // would be nice to dedup with PlanarVec::expand
        let mut data = BidiVec::new();
        data.grow(bounds.min_x..=bounds.max_x, BidiVec::new());
        for row in data.iter_mut() {
            row.grow(bounds.min_y..=bounds.max_y, default.clone());
        }

        Self { data, bounds }
    }

    /// Returns the world bounds
    pub fn bounds(&self) -> Bounds {
        self.bounds
    }

    /// Returns the x range
    pub fn x_range(&self) -> impl DoubleEndedIterator<Item = i64> + use<T> {
        self.bounds.min_x..=self.bounds.max_x
    }

    /// Returns the y range
    pub fn y_range(&self) -> impl DoubleEndedIterator<Item = i64> + use<T> {
        self.bounds.min_y..=self.bounds.max_y
    }

    /// Clears the entire `PlanarVec` and sets every value to the given default.
    pub fn clear(&mut self, default: T)
    where
        T: Clone,
    {
        for row in self.data.iter_mut() {
            row.fill(default.clone());
        }
    }

    /// Gets the value at the given position, if it exists.
    pub fn get(&self, x: i64, y: i64) -> Option<&T> {
        if !self.bounds.contains(x, y) {
            return None;
        }

        Some(&self.data[x][y])
    }

    /// Gets the value at the given position mutably, if it exists.
    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut T> {
        if !self.bounds.contains(x, y) {
            return None;
        }

        Some(&mut self.data[x][y])
    }

    /// Expands the `PlanarVec` to at least contain the given bounds.
    ///
    /// If the passed bounds are outside the current bounds, the `PlanarVec` is expanded to
    /// the union of both bounds. The new cells are filled with the given default value.
    pub fn expand(&mut self, bounds: Bounds, default: T)
    where
        T: Clone,
    {
        let union_bounds = Bounds {
            min_x: self.bounds.min_x.min(bounds.min_x),
            max_x: self.bounds.max_x.max(bounds.max_x),
            min_y: self.bounds.min_y.min(bounds.min_y),
            max_y: self.bounds.max_y.max(bounds.max_y),
        };

        if union_bounds == self.bounds {
            return;
        }

        self.data
            .grow(union_bounds.min_x..=union_bounds.max_x, BidiVec::new());
        for row in self.data.iter_mut() {
            row.grow(union_bounds.min_y..=union_bounds.max_y, default.clone());
        }

        self.bounds = union_bounds;
    }
}

impl<T> Index<(i64, i64)> for PlanarVec<T> {
    type Output = T;

    fn index(&self, (x, y): (i64, i64)) -> &Self::Output {
        self.get(x, y).expect("index out of bounds")
    }
}

impl<T> IndexMut<(i64, i64)> for PlanarVec<T> {
    fn index_mut(&mut self, (x, y): (i64, i64)) -> &mut Self::Output {
        self.get_mut(x, y).expect("index out of bounds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_planar_vec() {
        let bounds = Bounds {
            min_x: -1,
            max_x: 1,
            min_y: -1,
            max_y: 1,
        };
        let mut planar_vec = PlanarVec::new(bounds, 0);

        assert_eq!(planar_vec[(1, 1)], 0);

        planar_vec[(1, 1)] = 1;

        assert_eq!(planar_vec[(1, 1)], 1);

        planar_vec.expand(
            Bounds {
                min_x: -2,
                max_x: 2,
                min_y: 0,
                max_y: 0,
            },
            0,
        );

        assert_eq!(planar_vec[(1, 1)], 1);
        // y = 1 not OOB
        assert_eq!(planar_vec[(2, 1)], 0);
        assert_eq!(planar_vec.get(2, 2), None);
    }
}
