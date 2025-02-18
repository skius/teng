//! Defines `BidiVec`, a bidirectional vector for efficient front and back insertions with stable i64 indexing.
//!
//! The `bidivec` module provides the `BidiVec` data structure, which is designed to be efficiently growable
//! at both the beginning and the end, while maintaining stable `i64` indices. This is achieved by internally
//! using two `Vec<T>` to store elements with positive and negative indices separately.

use std::ops::{Index, IndexMut, RangeBounds};

/// `BidiVec` (Bidirectional Vector) is a data structure similar to `Vec<T>`, but it allows efficient
/// insertion and access at both the beginning and end. It uses stable `i64` indices, meaning that
/// indices are preserved even after growing towards positive or negative infinity.
///
/// Internally, it uses two `Vec<T>`: one for positive indices (`pos`) and one for negative indices (`neg`).
/// Index `0` is the start of the `pos` vector, index `-1` is the start of the `neg` vector, and so on.

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BidiVec<T> {
    /// Vector for storing elements with non-negative indices (0, 1, 2, ...).
    pos: Vec<T>,
    /// Vector for storing elements with negative indices (-1, -2, -3, ...).
    neg: Vec<T>,
}

impl<T> Default for BidiVec<T> {
    fn default() -> Self {
        Self::new()
    }
}

impl<T> BidiVec<T> {
    /// Creates a new empty `BidiVec`.
    pub fn new() -> Self {
        Self {
            pos: Vec::new(),
            neg: Vec::new(),
        }
    }

    /// Returns the length of the `BidiVec`.
    pub fn len(&self) -> i64 {
        self.pos.len() as i64 + self.neg.len() as i64
    }

    /// Returns `true` if the `BidiVec` is empty.
    pub fn is_empty(&self) -> bool {
        self.pos.is_empty() && self.neg.is_empty()
    }

    /// Clears the `BidiVec`, removing all elements.
    pub fn clear(&mut self) {
        self.pos.clear();
        self.neg.clear();
    }

    /// Set the entire BidiVec to the given value.
    pub fn fill(&mut self, value: T)
    where
        T: Clone,
    {
        self.pos.fill(value.clone());
        self.neg.fill(value);
    }

    /// Returns a reference to the element at the given index, or `None` if the index is out of bounds.
    pub fn get(&self, index: i64) -> Option<&T> {
        if index >= 0 {
            self.pos.get(index as usize)
        } else {
            self.neg.get((-index - 1) as usize)
        }
    }

    /// Returns a mutable reference to the element at the given index, or `None` if the index is out of bounds.
    pub fn get_mut(&mut self, index: i64) -> Option<&mut T> {
        if index >= 0 {
            self.pos.get_mut(index as usize)
        } else {
            self.neg.get_mut((-index - 1) as usize)
        }
    }

    /// Grows the `BidiVec` to contain the given range of indices, filling with the provided default.
    /// Does not shrink.
    pub fn grow(&mut self, indices: impl RangeBounds<i64>, default: T)
    where
        T: Clone,
    {
        let start = match indices.start_bound() {
            std::ops::Bound::Included(&start) => start,
            std::ops::Bound::Excluded(&start) => start + 1,
            std::ops::Bound::Unbounded => 0,
        };
        let end = match indices.end_bound() {
            std::ops::Bound::Included(&end) => end,
            std::ops::Bound::Excluded(&end) => end - 1,
            std::ops::Bound::Unbounded => 0,
        };

        // Don't shrink.
        let start = start.min(-(self.neg.len() as i64));
        let end = end.max(self.pos.len() as i64 - 1);

        if start < 0 {
            self.neg.resize((-start) as usize, default.clone());
        }

        if end >= 0 {
            self.pos.resize((end + 1) as usize, default);
        }
    }

    /// Returns an iterator over the elements of the `BidiVec`.
    pub fn iter(&self) -> impl Iterator<Item = &T> {
        self.neg.iter().chain(self.pos.iter())
    }

    /// Returns a mutable iterator over the elements of the `BidiVec`.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = &mut T> {
        self.neg.iter_mut().chain(self.pos.iter_mut())
    }
}

impl<T> Index<i64> for BidiVec<T> {
    type Output = T;

    fn index(&self, index: i64) -> &T {
        self.get(index).expect("index out of bounds")
    }
}

impl<T> IndexMut<i64> for BidiVec<T> {
    fn index_mut(&mut self, index: i64) -> &mut T {
        self.get_mut(index).expect("index out of bounds")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bidi_vec() {
        let mut vec = BidiVec::new();
        vec.grow(-5..5, 0);

        for i in -5..5 {
            assert_eq!(vec[i], 0);
        }

        vec[-5] = 1;
        vec[4] = 2;

        assert_eq!(vec[-5], 1);
        assert_eq!(vec[4], 2);
    }
}
