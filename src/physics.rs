use std::time::Instant;

pub struct PhysicsBoard {
    pub board: Vec<Vec<Entity>>
}

impl PhysicsBoard {
    pub fn new(max_width: usize) -> Self {

        Self { board: vec![vec![]; max_width] }
    }

    pub fn clear(&mut self) {
        for col in self.board.iter_mut() {
            col.clear();
        }
    }

    pub fn add_entity(&mut self, x: usize, y: usize, c: char) {
        self.board[x].push(Entity {
            x: x as f64,
            y: y as f64,
            vel_x: 0.0,
            vel_y: 0.0,
            c,
            time_at_bottom: 0.0,
        });
    }

    pub fn update(&mut self, dt: f64, height: usize, mut write_debug: impl FnMut(String)) {
        for col in self.board.iter_mut() {
            update_physics_col(col, dt, height, &mut write_debug);
        }
    }
}

#[derive(Clone)]
pub struct Entity {
    pub x: f64,
    pub y: f64,
    vel_x: f64,
    vel_y: f64,
    pub c: char,
    time_at_bottom: f64,
}

impl Entity {
    fn collides_with(&self, other: &Entity) -> bool {
        (self.y - other.y).abs() < 1.0
    }
}

fn handle_collision(entity_a: &mut Entity, entity_b: &mut Entity) {
    let (vel_y_a, vel_y_b) = (entity_a.vel_y, entity_b.vel_y);
    entity_a.vel_y = vel_y_b;
    entity_b.vel_y = vel_y_a;

    let mut diff = 1.0 - (entity_a.y - entity_b.y).abs();
    if entity_a.y < entity_b.y {
        // This is important, otherwise we would keep trying to push two entities together if the
        // relative positions are swapped
        diff = -diff;
    }
    entity_a.y += diff / 2.0;
    entity_b.y -= diff / 2.0;

}

fn update_physics_col(
    physics_entities: &mut Vec<Entity>,
    dt: f64,
    height: usize,
    mut write_debug: impl FnMut(String),
) {
    // 1 px/s/s
    let earth_accel = 40.0;
    let damping = 0.8;
    let max_time_at_bottom = 3.0;

    let mut height_to_entities = vec![vec![]; height];

    for  entity in physics_entities.iter_mut() {
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
    }

    physics_entities.retain(|entity| entity.time_at_bottom < max_time_at_bottom);
    let total_physics_entities = physics_entities.len();

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