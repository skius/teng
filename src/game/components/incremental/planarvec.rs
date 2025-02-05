use std::ops::{Index, IndexMut};
use crate::game::components::incremental::bidivec::BidiVec;

/// Bounds for a 2D plane. Includes all indices.
/// x values range from min_x..=max_x
/// y values range from min_y..=max_y
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Bounds {
    pub max_x: i64,
    pub min_x: i64,
    pub max_y: i64,
    pub min_y: i64,
}

impl Bounds {
    pub fn contains(&self, x: i64, y: i64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
    
    pub fn contains_bounds(&self, other: Bounds) -> bool {
        self.contains(other.min_x, other.min_y) && self.contains(other.max_x, other.max_y)
    }
}

/// A data structure capable of storing a 2D plane indexed by a pair of i64s, (x, y).
#[derive(Debug, Clone)]
pub struct PlanarVec<T> {
    // Outer index is x, inner index is y
    data: BidiVec<BidiVec<T>>,
    bounds: Bounds,
}

impl<T> PlanarVec<T> {
    /// Creates a new PlanarVec with the given bounds and default value.
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
    pub fn x_range(&self) -> impl DoubleEndedIterator<Item = i64> {
        self.bounds.min_x..=self.bounds.max_x
    }
    
    /// Returns the y range
    pub fn y_range(&self) -> impl DoubleEndedIterator<Item = i64> {
        self.bounds.min_y..=self.bounds.max_y
    }
    
    /// Clear the entire PlanarVec to the default value.
    pub fn clear(&mut self, default: T)
    where
        T: Clone,
    {
        for row in self.data.iter_mut() {
            row.fill(default.clone());
        }
    }
    
    
    /// Gets the value at the given position.
    pub fn get(&self, x: i64, y: i64) -> Option<&T> {
        if !self.bounds.contains(x, y) {
            return None;
        }

        Some(&self.data[x][y])
    }

    /// Gets the value at the given position mutably.
    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut T> {
        if !self.bounds.contains(x, y) {
            return None;
        }

        Some(&mut self.data[x][y])
    }

    /// Expands the planar v ec to at least contain the given bounds.
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

        self.data.grow(union_bounds.min_x..=union_bounds.max_x, BidiVec::new());
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
