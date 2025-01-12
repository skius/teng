use std::time::Instant;

pub struct PhysicsBoard {
    // x-indexed lists of entities, one per column
    // inside a column they're sorted by y
    pub board: Vec<Vec<Entity>>,
    x_to_height_to_entities: Vec<Vec<Vec<usize>>>,
}

impl PhysicsBoard {
    pub fn new(width: usize) -> Self {
        Self {
            board: vec![vec![]; width],
            x_to_height_to_entities: vec![vec![]; width],
        }
    }

    pub fn clear(&mut self) {
        for col in self.board.iter_mut() {
            col.clear();
        }
    }

    pub fn add_entity(&mut self, x: usize, y: usize, c: char) {
        let res =
            self.board[x].binary_search_by(|entity| entity.y.partial_cmp(&(y as f64)).unwrap());
        let insert_idx = res.unwrap_or_else(|idx| idx);

        self.board[x].insert(
            insert_idx,
            Entity {
                x: x as f64,
                y: y as f64,
                vel_x: 0.0,
                vel_y: 0.0,
                c,
                time_at_bottom: 0.0,
                ttl: None,
            },
        );
    }

    pub fn update(&mut self, dt: f64, height: usize, mut write_debug: impl FnMut(String)) {
        for col in 0..self.board.len() {
            self.update_physics_col(col, dt, height, &mut write_debug);
        }
    }

    pub fn resize(&mut self, width: usize) {
        self.board.resize(width, vec![]);
        self.x_to_height_to_entities.resize(width, vec![]);
    }

    fn update_physics_col(
        &mut self,
        col_idx: usize,
        dt: f64,
        height: usize,
        mut write_debug: impl FnMut(String),
    ) {
        let physics_entities = &mut self.board[col_idx];
        // physics_entities.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());
        let height_to_entities = &mut self.x_to_height_to_entities[col_idx];
        height_to_entities.resize(height, vec![]);
        height_to_entities
            .iter_mut()
            .for_each(|entities| entities.clear());

        let earth_accel = 40.0;
        let damping = 0.5;
        let max_time_at_bottom = 3.0;

        // let collision_damp_factor = 1.0 - dt * dt * 0.1;
        let collision_damp_factor = 1.0;

        for entity in physics_entities.iter_mut() {
            entity.vel_y += earth_accel * dt;
            entity.y += entity.vel_y * dt;
            if entity.y < 0.0 {
                entity.y = 0.0;
                entity.vel_y = -entity.vel_y;
                continue;
            }
            if entity.y >= height as f64 {
                entity.y = height as f64 - 0.01;
                entity.vel_y = -entity.vel_y;
                entity.vel_y *= damping;
            }
            if entity.y.floor() >= height as f64 - 1.0 {
                entity.time_at_bottom += dt;
            } else {
                entity.time_at_bottom = 0.0;
            }
            if let Some(ttl) = entity.ttl.as_mut() {
                *ttl -= dt;
            }
        }

        physics_entities.retain(|entity| match entity.ttl {
            Some(ttl) => ttl > 0.0,
            None => true,
        });

        // physics_entities.sort_by(|a, b| a.y.partial_cmp(&b.y).unwrap());

        // Go from bottom to top, check collisions as we go along, and set new 'floors' for collision
        // handling when we have a perfect stack at the bottom.

        let mut hard_bottom_y = height as f64 - 0.01;
        let bottom_check_tol = 0.0;
        let mut move_next = 0.0;
        let entity_height = 1.0;
        for lower_idx in (0..physics_entities.len()).rev() {
            let mut lower_entity = physics_entities[lower_idx].clone();
            // lower_entity.y -= move_next;
            if lower_entity.y >= hard_bottom_y - bottom_check_tol {
                let new_y = hard_bottom_y;
                // the .min() here is important! well, maybe hard_bottom_y - entity_height would be fine, but definitely not just lower_entity.y - entity_height!!
                hard_bottom_y = (hard_bottom_y - entity_height).min(lower_entity.y - entity_height);
                // don't need to do anything else with this one, since it's already at the bottom
                lower_entity.y = new_y;
                lower_entity.vel_y = -lower_entity.vel_y.abs() * damping;
                physics_entities[lower_idx] = lower_entity;
                move_next = 0.0;
                continue;
            }

            if lower_idx == 0 {
                // no entity below, we're done
                break;
            }

            // the room the lower entity has to collide.
            let collide_buffer = hard_bottom_y - lower_entity.y;
            assert!(
                collide_buffer >= 0.0,
                "collide_buffer: {}, lower_idx: {}, total: {}",
                collide_buffer,
                lower_idx,
                physics_entities.len()
            );

            let upper_idx = lower_idx - 1;
            let mut upper_entity = physics_entities[upper_idx].clone();

            let mut dist = lower_entity.y - upper_entity.y;
            // assert!(dist >= 0.0, "dist: {}, lower_idx: {}, total: {}", dist, lower_idx, physics_entities.len());
            if dist < 0.0 {
                // due to moving the lower entity up a bit, the upper entity is now technically below.
                // let's just pretend they're magically shifted to the same position
                upper_entity.y = lower_entity.y;
                dist = 0.0;
            }
            if dist < entity_height - 0.0001 {
                //  move lower entity at most by the collide buffer it has available
                let move_total = entity_height - dist;
                let move_lower_dist = collide_buffer.min(move_total / 2.0);
                let move_upper_dist = move_total - move_lower_dist;
                lower_entity.y += move_lower_dist;
                upper_entity.y -= move_upper_dist;
                // move_next = move_upper_dist;
                // update velocities
                let (vel_y_a, vel_y_b) = (lower_entity.vel_y, upper_entity.vel_y);
                lower_entity.vel_y = vel_y_b;
                upper_entity.vel_y = vel_y_a;
                lower_entity.vel_y *= collision_damp_factor;
                upper_entity.vel_y *= collision_damp_factor;
                if lower_entity.y >= hard_bottom_y - bottom_check_tol {
                    // assert!(lower_entity.y == hard_bottom_y, "lower_entity.y: {}, hard_bottom_y: {}", lower_entity.y, hard_bottom_y);
                    // due to collision, it's now touching the bottom
                    let new_y = hard_bottom_y;

                    hard_bottom_y =
                        (hard_bottom_y - entity_height).min(lower_entity.y - entity_height);
                    lower_entity.y = new_y;
                    // so velocity needs to be changed again
                    lower_entity.vel_y = -lower_entity.vel_y.abs() * damping;
                }
                physics_entities[lower_idx] = lower_entity;
                physics_entities[upper_idx] = upper_entity;
            }
        }

        assert!(
            physics_entities.is_sorted_by(|a, b| a.y <= b.y),
            "{:?}, bottom: {}",
            physics_entities.iter().map(|e| e.y).collect::<Vec<_>>(),
            hard_bottom_y
        );

        return;

        // physics_entities.retain(|entity| entity.time_at_bottom < max_time_at_bottom);
        for (i, entity) in physics_entities.iter().enumerate() {
            let height = entity.y.floor() as usize;
            height_to_entities[height].push(i);
        }

        for (base_height, window) in height_to_entities.windows(2).enumerate().rev() {
            let height_a = base_height;
            let height_b = base_height + 1;
            // check all collisions of A x B, and A x A.
            for &idx_a in window[0].iter() {
                for &idx_b in window[1].iter() {
                    if physics_entities[idx_a].collides_with(&physics_entities[idx_b]) {
                        let mut entity_a = physics_entities[idx_a].clone();
                        let mut entity_b = physics_entities[idx_b].clone();
                        handle_collision(&mut entity_a, &mut entity_b);
                        physics_entities[idx_a] = entity_a;
                        physics_entities[idx_b] = entity_b;
                    }
                }
                for &idx_b in window[0].iter() {
                    if idx_a == idx_b {
                        continue;
                    }
                    if physics_entities[idx_a].collides_with(&physics_entities[idx_b]) {
                        let mut entity_a = physics_entities[idx_a].clone();
                        let mut entity_b = physics_entities[idx_b].clone();
                        handle_collision(&mut entity_a, &mut entity_b);
                        physics_entities[idx_a] = entity_a;
                        physics_entities[idx_b] = entity_b;
                    }
                }
            }
        }
        // Check collisions of last row with itself (the remaining B x B)
        if let Some(last_row) = height_to_entities.last() {
            for (i, &idx_a) in last_row.iter().enumerate() {
                for &idx_b in &last_row[i + 1..] {
                    if physics_entities[idx_a].collides_with(&physics_entities[idx_b]) {
                        let mut entity_a = physics_entities[idx_a].clone();
                        let mut entity_b = physics_entities[idx_b].clone();
                        handle_collision(&mut entity_a, &mut entity_b);
                        physics_entities[idx_a] = entity_a;
                        physics_entities[idx_b] = entity_b;
                    }
                }
            }
        }
    }
}

#[derive(Clone, Debug)]
pub struct Entity {
    pub x: f64,
    pub y: f64,
    pub vel_x: f64,
    pub vel_y: f64,
    pub c: char,
    pub time_at_bottom: f64,
    pub ttl: Option<f64>,
}

impl Entity {
    fn collides_with(&self, other: &Entity) -> bool {
        (self.y - other.y).abs() < 1.0
    }
}

/// Returns the distance by which both need to move in total
fn handle_collision(entity_a: &mut Entity, entity_b: &mut Entity) -> f64 {
    let (vel_y_a, vel_y_b) = (entity_a.vel_y, entity_b.vel_y);
    entity_a.vel_y = vel_y_b;
    entity_b.vel_y = vel_y_a;
    let collision_damp_factor = 0.8;
    entity_a.vel_y *= collision_damp_factor;
    entity_b.vel_y *= collision_damp_factor;

    let mut diff = 1.0 - (entity_a.y - entity_b.y).abs();
    // if entity_a.y < entity_b.y {
    //     // This is important, otherwise we would keep trying to push two entities together if the
    //     // relative positions are swapped
    //     diff = -diff;
    // }

    diff.abs()
    // entity_a.y += diff / 2.0;
    // entity_b.y -= diff / 2.0;
}
