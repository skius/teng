use serde::Deserialize;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Default, Hash, PartialEq, Eq)]
pub struct Cell {
    pub x: i64,
    pub y: i64,
}

pub trait SpatialHashable {
    fn get_aabb(&self) -> Aabb;
}

/// Axis-aligned bounding box, in world coordinates.
pub struct Aabb {
    pub min_x: i64,
    pub min_y: i64,
    pub max_x: i64,
    pub max_y: i64,
}

impl Aabb {
    fn get_cells(&self, cell_size: i64) -> impl Iterator<Item = Cell> + use<> {
        // we want to do a floor division here to get consistent behavior across sign changes
        let min_x = (self.min_x as f64 / cell_size as f64).floor() as i64;
        let min_y = (self.min_y as f64 / cell_size as f64).floor() as i64;
        let max_x = (self.max_x as f64 / cell_size as f64).floor() as i64;
        let max_y = (self.max_y as f64 / cell_size as f64).floor() as i64;
        (min_x..=max_x).flat_map(move |x| (min_y..=max_y).map(move |y| Cell { x, y }))
    }
}

pub struct SpatialHashGrid<T> {
    grid: HashMap<Cell, Vec<T>>,
    cell_size: i64,
}

impl<T> SpatialHashGrid<T> {
    pub fn new(cell_size: i64) -> Self {
        Self {
            grid: HashMap::new(),
            cell_size,
        }
    }

    pub fn insert(&mut self, item: T)
    where
        T: SpatialHashable + Clone,
    {
        let aabb = item.get_aabb();
        self.insert_with_aabb(item, aabb)
    }

    pub fn insert_with_aabb(&mut self, item: T, aabb: Aabb)
    where
        T: Clone,
    {
        for cell in aabb.get_cells(self.cell_size) {
            self.grid.entry(cell).or_default().push(item.clone());
        }
    }

    pub fn get(&self, cell: Cell) -> impl Iterator<Item = &T> {
        self.grid.get(&cell).into_iter().flat_map(|v| v.iter())
    }

    pub fn get_for_aabb(&self, aabb: Aabb) -> impl Iterator<Item = &T> {
        aabb.get_cells(self.cell_size)
            .flat_map(move |cell| self.get(cell))
    }
}
