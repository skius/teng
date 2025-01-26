use std::ops::{Index, IndexMut};
use crate::game::components::incremental::planarvec::{Bounds, PlanarVec};

pub trait CornerPairExtension {
    fn to_corner(self) -> (CornerIdx, CornerIdx);
    fn adjacent_corners(self) -> [(CornerIdx, CornerIdx); 4];
}

impl CornerPairExtension for (i64, i64) {
    fn to_corner(self) -> (CornerIdx, CornerIdx) {
        (CornerIdx(self.0), CornerIdx(self.1))
    }

    fn adjacent_corners(self) -> [(CornerIdx, CornerIdx); 4] {
        let (x, y) = self;
        [
            (x, y).to_corner(),
            (x + 1, y).to_corner(),
            (x, y + 1).to_corner(),
            (x + 1, y + 1).to_corner(),
        ]
    }
}

/// An index used when specifically targeting some corner of a tile.
/// The corner indices of a tile at indices (x, y) are:
/// Top left: (x, y)
/// Top right: (x + 1, y)
/// Bottom left: (x, y + 1)
/// Bottom right: (x + 1, y + 1)
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CornerIdx(pub i64);

/// Bounding boxes are defined by their top-left and bottom-right corners.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BoundingBox {
    pub top_left: (CornerIdx, CornerIdx),
    pub bottom_right: (CornerIdx, CornerIdx),
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollisionCell {
    Empty,
    Solid,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CornerCollision {
    Empty,
    Surface,
    Inside,
}

#[derive(Debug, Clone)]
pub struct CollisionBoard {
    board: PlanarVec<CollisionCell>
}

impl CollisionBoard {
    pub fn new(bounds: Bounds) -> Self {
        Self {
            board: PlanarVec::new(bounds, CollisionCell::Empty)
        }
    }

    pub fn get(&self, x: i64, y: i64) -> Option<&CollisionCell> {
        self.board.get(x, y)
    }

    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut CollisionCell> {
        self.board.get_mut(x, y)
    }

    pub fn adjacent_tiles(&self, x: CornerIdx, y: CornerIdx) -> impl Iterator<Item = CollisionCell> + '_ {
        // The corner (x,y) is the top-left corner of tile (x,y)
        let x = x.0;
        let y = y.0;
        let x_range = x - 1..=x;
        let y_range = y - 1..=y;

        x_range.flat_map(move |x| y_range.clone().map(move |y| self.board.get(x, y).copied().unwrap_or(CollisionCell::Empty)))
    }

    pub fn get_corner(&self, (x, y): (CornerIdx, CornerIdx)) -> CollisionCell {
        if self.adjacent_tiles(x, y).any(|cell| cell == CollisionCell::Solid) {
            CollisionCell::Solid
        } else {
            CollisionCell::Empty
        }
    }

    pub fn collides(&self, bounding_box: BoundingBox) -> bool {
        let BoundingBox { top_left, bottom_right } = bounding_box;
        let min_x = top_left.0.0;
        let max_x = bottom_right.0.0;
        let min_y = bottom_right.1.0;
        let max_y = top_left.1.0;
        for x in min_x..=max_x {
            for y in min_y..=max_y {
                if self.get_corner((x, y).to_corner()) == CollisionCell::Solid {
                    return true;
                }
            }
        }
        false
    }
}

impl Index<(i64, i64)> for CollisionBoard {
    type Output = CollisionCell;

    fn index(&self, (x, y): (i64, i64)) -> &Self::Output {
        &self.board[(x, y)]
    }
}

impl IndexMut<(i64, i64)> for CollisionBoard {
    fn index_mut(&mut self, (x, y): (i64, i64)) -> &mut Self::Output {
        &mut self.board[(x, y)]
    }
}