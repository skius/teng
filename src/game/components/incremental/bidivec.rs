use std::ops::{Index, IndexMut, RangeBounds};

/// A vector that can be efficiently appended to both the front and back and has stable i64 indices.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BidiVec<T> {
    pos: Vec<T>,
    neg: Vec<T>,
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

        if start < 0 {
            self.neg.resize((-start) as usize, default.clone());
        }

        if end >= 0 {
            self.pos.resize((end + 1) as usize, default);
        }
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
