use std::ops::{Index, IndexMut};
use crate::game::components::incremental::planarvec::{Bounds, PlanarVec};
use crate::game::components::incremental::player::NewPlayerComponent;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CollisionCell {
    Empty,
    Solid,
}

/// The object in question hits a wall at the given side. That means the object is 'free' on the
/// other side.
pub enum HorizontalCollision {
    Left,
    Right,
}

/// The object in question hits a wall at the given side. That means the object is 'free' on the
/// other side.
pub enum VerticalCollision {
    Top,
    Bottom,
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

    pub fn expand(&mut self, bounds: Bounds) {
        self.board.expand(bounds, CollisionCell::Empty);
    }

    pub fn get(&self, x: i64, y: i64) -> Option<&CollisionCell> {
        self.board.get(x, y)
    }

    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut CollisionCell> {
        self.board.get_mut(x, y)
    }

    pub fn collides_growing(&mut self, bounding_box: Bounds) -> bool {
        self.expand(bounding_box);
        let Bounds { min_x, max_x, min_y, max_y } = bounding_box;

        // let bottom_cutoff = min_y + (max_y - min_y) / 2;
        // let left_cutoff = min_x + (max_x - min_x) / 2;
        //
        // let mut horizontal_collision = None;
        // let mut vertical_collision = None;

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                if self[(x, y)] == CollisionCell::Solid {
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

pub struct PhysicsEntity2d {
    pub position: (f64, f64),
    pub velocity: (f64, f64),
    // These define the size of the bounding box of the entity relative to position.
    // For example, size_top = size_left = 0 would make the entity top-left corner centered.
    pub size_top: f64,
    pub size_bottom: f64,
    pub size_left: f64,
    pub size_right: f64,
}

impl PhysicsEntity2d {

    pub fn bounding_box(&self) -> Bounds {
        let (x, y) = self.position;
        let min_x = (x - self.size_left).floor() as i64;
        let max_x = (x + self.size_right).floor() as i64;
        let max_y = (y + self.size_top).floor() as i64;
        let min_y = (y - self.size_bottom).floor() as i64;

        Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    pub fn grounded(&self, collision_board: &mut CollisionBoard) -> bool {
        let floor_sensor = self.floor_sensor();
        collision_board.collides_growing(floor_sensor)
    }

    fn update_velocity(&mut self, dt: f64, acceleration: (f64, f64)) {
        let (ax, ay) = acceleration;
        let (vx, vy) = self.velocity;

        self.velocity = (vx + ax * dt, vy + ay * dt);
    }

    pub fn update(&mut self, dt: f64, collision_board: &mut CollisionBoard) {
        if collision_board.collides_growing(self.bounding_box()) {
            // We are already colliding with something, so we should try and escape by moving up
            self.position.1 += 1.0;
            return;
        }

        self.update_velocity(dt, (0.0, -40.0));

        let (x, mut y) = self.position;
        let (vx, vy) = self.velocity;

        let x_tile = x.floor() as i64;
        let y_tile = y.floor() as i64;

        let mut new_x = x + vx * dt;
        let mut new_y = y + vy * dt;

        // Check if moving would even cause a change in tile position
        let new_x_tile = new_x.floor() as i64;
        let new_y_tile = new_y.floor() as i64;

        let x_diff = new_x_tile - x_tile;
        let y_diff = new_y_tile - y_tile;

        if x_diff != 0 {
            // we're moving across a tile boundary, need to check if we need to adjust new_x
            let (mut bounds, xds) = if x_diff > 0 {
                (self.right_sensor(), 0..=x_diff)
            } else {
                (self.left_sensor(), 0..=x_diff.abs())
            };

            let mut collision = false;

            'outer: for xd in xds {
                let mut sensor_bb = bounds;
                sensor_bb.min_x += xd;
                sensor_bb.max_x += xd;
                if collision_board.collides_growing(sensor_bb) {
                    // Check step, but only if not jumping right now
                    if vy <= 0.0 {
                        for step in 1..=NewPlayerComponent::STEP_SIZE {
                            let mut step_sensor_bb = sensor_bb;
                            step_sensor_bb.min_y += step;
                            step_sensor_bb.max_y += step;
                            if !collision_board.collides_growing(step_sensor_bb) {
                                new_y += step as f64;
                                // we moved up, so our sensor should move up too
                                bounds.min_y += step;
                                bounds.max_y += step;
                                continue 'outer;
                            }
                        }
                    }


                    collision = true;
                    new_x = x + xd as f64;
                    break;
                }
            }
        }

        self.position.0 = new_x;

        // match x_diff {
        //     _ if x_diff == 0 => {
        //         // No horizontal movement, so we do not need to do any horizontal collision checking
        //         self.position.0 = new_x;
        //     }
        //     _ if x_diff > 0 => {
        //         // Moving right
        //         // let mut collision = false;
        //         // let right_sensor = self.right_sensor();
        //         // 'outer: for xd in 0..=x_diff {
        //         //     let mut future_right_sensor = right_sensor;
        //         //     future_right_sensor.min_x += xd;
        //         //     future_right_sensor.max_x += xd;
        //         //     if collision_board.collides_growing(future_right_sensor) {
        //         //         // Now check if a step is possible, if so, move it up by yd and continue
        //         //         for step in 1..=NewPlayerComponent::STEP_SIZE {
        //         //             let mut right_step_sensor = future_right_sensor;
        //         //             right_step_sensor.min_y += step;
        //         //             right_step_sensor.max_y += step;
        //         //             if !collision_board.collides_growing(right_step_sensor) {
        //         //                 // TODO: the sensors need to be recomputed now, and
        //         //                 // y should be persistent. maybe directly update y?
        //         //                 new_y = y + step as f64;
        //         //                 continue 'outer;
        //         //             }
        //         //         }
        //         //         new_x = x + xd as f64;
        //         //         collision = true;
        //         //         break;
        //         //     }
        //         // }
        //         //
        //         // self.position.0 = new_x;
        //
        //         let ci = handle_horizontal(collision_board, 0..=x_diff, self.right_sensor());
        //         if let Some(CollisionInformation { xd, yd }) = ci {
        //             if let Some(xd) = xd {
        //                 new_x = x + xd as f64;
        //             }
        //             if let Some(yd) = yd {
        //                 // TODO: directly update y?
        //                 new_y = y + yd as f64;
        //             }
        //         }
        //         self.position.0 = new_x;
        //     }
        //     _ if x_diff < 0 => {
        //         // Moving left
        //         let mut collision = false;
        //         let left_sensor = self.left_sensor();
        //         for xd in 0..=x_diff.abs() {
        //             let mut future_left_sensor = left_sensor;
        //             future_left_sensor.min_x -= xd;
        //             future_left_sensor.max_x -= xd;
        //             if collision_board.collides_growing(future_left_sensor) {
        //                 new_x = x - xd as f64;
        //                 collision = true;
        //                 break;
        //             }
        //         }
        //
        //         self.position.0 = new_x;
        //     }
        //     _ => unreachable!("x_diff is either 0, positive, or negative"),
        // }

        match y_diff {
            _ if y_diff == 0 => {
                // No vertical movement, so we do not need to do any vertical collision checking
                self.position.1 = new_y;
            }
            _ if y_diff > 0 => {
                // Moving up
                let mut collision = false;
                let top_sensor = self.top_sensor();
                for yd in 0..=y_diff {
                    let mut future_top_sensor = top_sensor;
                    future_top_sensor.min_y += yd;
                    future_top_sensor.max_y += yd;
                    if collision_board.collides_growing(future_top_sensor) {
                        new_y = y + yd as f64;
                        collision = true;
                        break;
                    }
                }

                self.position.1 = new_y;
            }
            _ if y_diff < 0 => {
                // Moving down
                let mut collision = false;
                let floor_sensor = self.floor_sensor();
                for yd in 0..=y_diff.abs() {
                    let mut future_floor_sensor = floor_sensor;
                    future_floor_sensor.min_y -= yd;
                    future_floor_sensor.max_y -= yd;
                    if collision_board.collides_growing(future_floor_sensor) {
                        new_y = y - yd as f64;
                        collision = true;
                        break;
                    }
                }

                self.position.1 = new_y;
            }
            _ => unreachable!("y_diff is either 0, positive, or negative"),
        }
    }

    /// Returns a bounding box that is one unit high and resides directly above the entity.
    pub fn top_sensor(&self) -> Bounds {
        let Bounds { min_x, max_x, mut min_y, mut max_y } = self.bounding_box();

        max_y += 1;
        min_y = max_y;

        Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    /// Returns a bounding box that is one unit high and resides directly below the entity.
    pub fn floor_sensor(&self) -> Bounds {
        let Bounds { min_x, max_x, mut min_y, mut max_y } = self.bounding_box();

        min_y -= 1;
        max_y = min_y;

        Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    /// Returns a bounding box that is one unit wide and resides directly to the left of the entity.
    pub fn left_sensor(&self) -> Bounds {
        let Bounds { mut min_x, mut max_x, min_y, max_y } = self.bounding_box();

        min_x -= 1;
        max_x = min_x;

        Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    /// Returns a bounding box that is one unit wide and resides directly to the right of the entity.
    pub fn right_sensor(&self) -> Bounds {
        let Bounds { mut min_x, mut max_x, min_y, max_y } = self.bounding_box();

        max_x += 1;
        min_x = max_x;

        Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }
}