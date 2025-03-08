mod math;
mod spatial_hash_grid;

use crate::math::Vec2;
use std::{io, thread};
use std::ops::{Index, IndexMut};
use rayon::prelude::*;
use teng::components::Component;
use teng::rendering::color::Color;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::{HalfBlockDisplayRender, Render};
use teng::rendering::renderer::Renderer;
use teng::util::fixedupdate::FixedUpdateRunner;
use teng::{
    Game, SetupInfo, SharedState, UpdateInfo, install_panic_handler, terminal_cleanup,
    terminal_setup,
};
use crate::spatial_hash_grid::{Aabb, SpatialHashGrid};

/// Ball-shaped collision entity
#[derive(Debug, Clone)]
struct Entity {
    pos: Vec2,
    vel: Vec2,
    accel: Vec2,
    radius: f64,
    mass: f64,
}

impl Entity {
    const DEFAULT_ACCEL: Vec2 = Vec2 { x: 0.0, y: -30.0 };

    fn new_at(x: f64, y: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::new(0.0, 0.0),
            accel: Self::DEFAULT_ACCEL,
            radius: 0.5,
            mass: 0.5 * 0.5 * std::f64::consts::PI,
        }
    }

    fn set_radius(&mut self, radius: f64) {
        self.radius = radius;
        self.mass = radius * radius * std::f64::consts::PI;
    }

    fn with_radius(mut self, radius: f64) -> Self {
        self.set_radius(radius);
        self
    }

    fn with_velocity(self, vel: Vec2) -> Self {
        Self { vel, ..self }
    }

    #[inline]
    fn new_accel(&self) -> Vec2 {
        // Derive new acceleration from current position. Avoid using anything except the current
        // position to get error bounds from Verlet.
        Self::DEFAULT_ACCEL
    }

    #[inline]
    fn update(&mut self, dt: f64) {
        // velocity Verlet integration: https://en.wikipedia.org/wiki/Verlet_integration#Velocity_Verlet
        self.pos += self.vel * dt + 0.5 * self.accel * dt * dt;
        let new_accel = self.new_accel();
        self.vel += 0.5 * (self.accel + new_accel) * dt;
        self.accel = new_accel;
    }

    #[inline]
    fn handle_world_collisions(&mut self, world_width: f64, world_height: f64) {
        let collision_loss = 0.9;
        if self.pos.x - self.radius < 0.0 {
            self.pos.x = self.radius;
            self.vel.x = -self.vel.x * collision_loss;
        }
        if self.pos.x + self.radius > world_width {
            self.pos.x = world_width - self.radius;
            self.vel.x = -self.vel.x * collision_loss;
        }
        if self.pos.y - self.radius < 0.0 {
            self.pos.y = self.radius;
            self.vel.y = -self.vel.y * collision_loss;
        }
        if self.pos.y + self.radius > world_height {
            self.pos.y = world_height - self.radius;
            self.vel.y = -self.vel.y * collision_loss;
        }
    }

    fn get_aabb(&self) -> Aabb {
        let min = self.pos - Vec2::new(self.radius, self.radius);
        let max = self.pos + Vec2::new(self.radius, self.radius);
        let (min_x, min_y) = min.floor_to_i64();
        let (max_x, max_y) = max.floor_to_i64();
        Aabb {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }

    // returns distance if collides, None otherwise
    fn collides_with(&self, other: &Entity) -> Option<f64> {
        let dist = (self.pos - other.pos).length();
        if dist < self.radius + other.radius {
            Some(dist)
        } else {
            None
        }
    }

    fn handle_collision(&mut self, other: &mut Entity, dist: f64) {
        let entity1 = self;
        let entity2 = other;
        // collision response, taking into account mass and coefficient of restitution
        let normal = (entity2.pos - entity1.pos).normalized();
        let relative_velocity = entity2.vel - entity1.vel;
        let impulse = 2.0
            * entity1.mass
            * entity2.mass
            * normal.dot(relative_velocity)
            / (entity1.mass + entity2.mass);
        entity1.vel += impulse * normal / entity1.mass;
        entity2.vel -= impulse * normal / entity2.mass;

        // move entities apart
        let overlap = entity1.radius + entity2.radius - dist;
        let move1 = -overlap * entity1.mass / (entity1.mass + entity2.mass);
        let move2 = overlap * entity2.mass / (entity1.mass + entity2.mass);
        entity1.pos += move1 * normal;
        entity2.pos += move2 * normal;

        // apply coefficient of restitution
        entity1.vel *= PhysicsComponent::COEFFICIENT_OF_RESTITUTION;
        entity2.vel *= PhysicsComponent::COEFFICIENT_OF_RESTITUTION;
    }
}

#[derive(Debug)]
struct PartitionedEntities {
    total_entities: usize,
    num_partitions: usize,
    // invariant: partitions.len() == num_partitions
    partitions: Vec<Vec<Entity>>,
}

impl PartitionedEntities {
    fn new(num_partitions: usize) -> Self {
        Self {
            total_entities: 0,
            num_partitions,
            partitions: vec![vec![]; num_partitions],
        }
    }

    fn len(&self) -> usize {
        self.total_entities
    }

    fn clear(&mut self) {
        self.total_entities = 0;
        self.partitions.clear();
    }

    fn iter(&self) -> impl Iterator<Item = &Entity> {
        self.partitions.iter().flat_map(|partition| partition.iter())
    }

    fn iter_mut(&mut self) -> impl Iterator<Item = &mut Entity> {
        self.partitions.iter_mut().flat_map(|partition| partition.iter_mut())
    }

    fn push(&mut self, entity: Entity) {
        let partition = self.total_entities % self.num_partitions;
        if partition >= self.partitions.len() {
            self.partitions.push(Vec::new());
        }
        self.partitions[partition].push(entity);
        self.total_entities += 1;
    }

    fn total_index_to_partition_and_partition_idx(&self, idx: usize) -> (usize, usize) {
        let partition = idx % self.num_partitions;
        let partition_idx = idx / self.num_partitions;
        (partition, partition_idx)
    }

    fn get_two_mut(&mut self, idx1: usize, idx2: usize) -> (&mut Entity, &mut Entity) {
        let (partition1, partition_idx1) = self.total_index_to_partition_and_partition_idx(idx1);
        let (partition2, partition_idx2) = self.total_index_to_partition_and_partition_idx(idx2);
        if partition1 == partition2 {
            let partition = &mut self.partitions[partition1];
            if partition_idx1 < partition_idx2 {
                let (p1s, p2s) = partition.split_at_mut(partition_idx2);
                (&mut p1s[partition_idx1], &mut p2s[0])
            } else {
                let (p1s, p2s) = partition.split_at_mut(partition_idx1);
                (&mut p2s[0], &mut p1s[partition_idx2])
            }
        } else {
            if partition1 < partition2 {
                let (p1s, p2s) = self.partitions.split_at_mut(partition2);
                let (p1, p2) = (&mut p1s[partition1], &mut p2s[0]);
                (&mut p1[partition_idx1], &mut p2[partition_idx2])
            } else {
                let (p1s, p2s) = self.partitions.split_at_mut(partition1);
                let (p1, p2) = (&mut p2s[0], &mut p1s[partition2]);
                (&mut p1[partition_idx1], &mut p2[partition_idx2])
            }
        }
    }
}

impl Index<usize> for PartitionedEntities {
    type Output = Entity;

    fn index(&self, index: usize) -> &Self::Output {
        let (partition, partition_idx) = self.total_index_to_partition_and_partition_idx(index);
        &self.partitions[partition][partition_idx]
    }
}

impl IndexMut<usize> for PartitionedEntities {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        let (partition, partition_idx) = self.total_index_to_partition_and_partition_idx(index);
        &mut self.partitions[partition][partition_idx]
    }
}

impl Default for PartitionedEntities {
    fn default() -> Self {
        Self::new(2 * 16)
    }
}

#[derive(Debug, Default)]
struct GameState {
    // entities: Vec<Entity>,
    entities: PartitionedEntities,
    world_height: f64,
    world_width: f64,
}

struct PhysicsComponent {
    fur: FixedUpdateRunner,
}

impl PhysicsComponent {
    const COEFFICIENT_OF_RESTITUTION: f64 = 1.0;
    const PHYSICS_TICK_RATE: f64 = 60.0;

    fn new() -> Self {
        Self {
            fur: FixedUpdateRunner::new_from_rate_per_second(Self::PHYSICS_TICK_RATE),
        }
    }

    fn entent_basic(&self, dt: f64, state: &mut GameState) {
        // Max: 5'400 entities at 60tps
        for idx1 in 0..state.entities.len() {
            for idx2 in idx1 + 1..state.entities.len() {
                // let (entities1, entities2) = state.entities.split_at_mut(idx2);
                // let entity1 = &mut entities1[idx1];
                // let entity2 = &mut entities2[0];
                let (entity1, entity2) = state.entities.get_two_mut(idx1, idx2);
                // check collision
                if let Some(dist) = entity1.collides_with(entity2) {
                    entity1.handle_collision(entity2, dist);
                }
            }
        }
    }

    fn entent_shg(&self, dt: f64, state: &mut GameState) {
        // cell size 1: Max: between 25'000 and 35'000 entities at 60tps
        // cell size 5: Max: between 35'000 and 40'000 at 60tps
        // cell size 10: less

        let mut shg = SpatialHashGrid::new(5);
        for (idx, entity) in state.entities.iter().enumerate() {
            shg.insert_with_aabb(idx, entity.get_aabb());
        }
        for idx1 in 0..state.entities.len() {
            for &idx2 in shg.get_for_aabb(state.entities[idx1].get_aabb()) {
                if idx1 == idx2 {
                    continue;
                }
                // let (idx1, idx2) = (idx1.min(idx2), idx1.max(idx2));
                // let (entities1, entities2) = state.entities.split_at_mut(idx2);
                // let entity1 = &mut entities1[idx1];
                // let entity2 = &mut entities2[0];
                let (entity1, entity2) = state.entities.get_two_mut(idx1, idx2);
                
                // check collision
                if let Some(dist) = entity1.collides_with(entity2) {
                    entity1.handle_collision(entity2, dist);
                }
            }
        }
    }

    fn entent_shg_multithreaded(&self, dt: f64, state: &mut GameState) {
        // Performance:
        // at 60tps worse than shg. like <25k.
        // but at 20tps it reaches closer to 120k entities before the death spiral
        // whereas singlethreaded shg reaches 97k at 20tps.

        // fn get_matching(num_threads: usize, matching_idx: usize) -> Vec<(usize, usize)> {
        //     let total_num_matchings = 2 * num_threads - 1;
        //     let mut matching = Vec::new();
        //     for j in 0..num_threads {
        //         let first = matching_idx;
        //         let second = if (j + matching_idx + 1) % total_num_matchings != 0 {
        //             (j + matching_idx + 1) % total_num_matchings
        //         } else {
        //             total_num_matchings
        //         };
        //         matching.push((first, second));
        //     }
        //     matching
        // }

        // TODO: this function is entirely wrong.
        // I *think* I want something like this: https://mathematica.stackexchange.com/questions/88085/find-all-the-possible-ways-of-partitioning-a-list-into-a-set-of-pairs-of-element
        // fn generate_matching(n: usize, index: usize) -> Vec<(usize, usize)> {
        //     let total = 2 * n;
        //
        //     let mut pairs = Vec::new();
        //     for j in 0..n {
        //         let a = j;
        //         let b = (j + index + 1) % (total - 1);
        //         let b = if b == 0 { total - 1 } else { b };
        //         pairs.push((a, b));
        //     }
        //
        //     pairs
        // }

        // it's just a 1-factorization of the complete graph K_{2k}.
        fn one_factorization(k: usize) -> Vec<Vec<(usize, usize)>> {
            let n = 2 * k;
            // Initialize the list of teams
            let mut teams: Vec<usize> = (0..n).collect();
            let mut rounds = Vec::with_capacity(n - 1);

            // There will be n-1 rounds (matchings)
            for _ in 0..(n - 1) {
                let mut round = Vec::with_capacity(k);
                // Pair the teams: first with last, second with second-last, etc.
                for i in 0..(n / 2) {
                    round.push((teams[i], teams[n - 1 - i]));
                }
                rounds.push(round);

                // Rotate the teams, keeping the first team fixed.
                let last = teams.pop().unwrap();
                teams.insert(1, last);
            }
            rounds
        }


        let num_threads = 16;
        let num_pairs = 2 * num_threads;
        let num_matchings = num_pairs - 1;

        // partition the balls into num_pairs partitions. each has its own spatial hashgrid
        let mut shgs = Vec::new();
        // let mut indices_of_partitions = Vec::new();
        let mut partitions = Vec::new();

        // let mut original_indicies_per_partition = Vec::new();
        for _ in 0..num_pairs {
            shgs.push(SpatialHashGrid::new(5));
            // indices_of_partitions.push(Vec::new());
            partitions.push(Some(Vec::new()));
            // original_indicies_per_partition.push(Vec::new());
        }
        for (idx, entity) in state.entities.iter().enumerate() {
            let partition = idx % num_pairs;

            let idx_in_partition = partitions[partition].as_ref().unwrap().len();
            partitions[partition].as_mut().unwrap().push(entity.clone());
            // original_indicies_per_partition[partition].push(idx);

            shgs[partition].insert_with_aabb(idx_in_partition, entity.get_aabb());
            // indices_of_partitions[partition].push(idx);
        }

        // first pass: handle collisions within each partition
        thread::scope(|s| {
            for (idx, partition) in partitions.iter_mut().enumerate() {
                let partition = partition.as_mut().unwrap();
                let shg = &shgs[idx];
                s.spawn(|| {
                    for idx1 in 0..partition.len() {
                        for &idx2 in shg.get_for_aabb(partition[idx1].get_aabb()) {
                            if idx1 == idx2 {
                                continue;
                            }
                            let (idx1, idx2) = (idx1.min(idx2), idx1.max(idx2));
                            let (entities1, entities2) = partition.split_at_mut(idx2);
                            let entity1 = &mut entities1[idx1];
                            let entity2 = &mut entities2[0];
                            // check collision
                            if let Some(dist) = entity1.collides_with(entity2) {
                                entity1.handle_collision(entity2, dist);
                            }
                        }
                    }
                });
            }
        });
        // for partition in 0..num_pairs {
        //     let shg = &shgs[partition];
        //     for idx1 in 0..partitions[partition].len() {
        //         for &idx2 in shg.get_for_aabb(partitions[partition][idx1].get_aabb()) {
        //             if idx1 == idx2 {
        //                 continue;
        //             }
        //             let (idx1, idx2) = (idx1.min(idx2), idx1.max(idx2));
        //             let (entities1, entities2) = partitions[partition].split_at_mut(idx2);
        //             let entity1 = &mut entities1[idx1];
        //             let entity2 = &mut entities2[0];
        //             // check collision
        //             if let Some(dist) = entity1.collides_with(entity2) {
        //                 entity1.handle_collision(entity2, dist);
        //             }
        //         }
        //     }
        // }

        // panic!("partition 0 is: {:?}", partitions[0]);

        // iterate over all pairs of partitions, and handle collisions between them
        let all_matchings = one_factorization(num_threads);
        for matching in all_matchings {
            // let matching = get_matching(num_threads, matching_idx);
            // let matching = generate_matching(num_threads, matching_idx);
            // panic!("matching: {:?}", matching);

            // create a new Vec here of (first, second) partitions, then iter_mut should work and we can spawn.
            let mut paired_partitions = Vec::new();
            for (first, second) in matching {
                // if first == 0 {
                //     panic!("first is 0, midx: {matching_idx}, first_partition: {fp:?}", fp = partitions[first]);
                // }
                // if partitions[first].is_none() {
                //     panic!("partition {} is None, midx: {matching_idx}", first);
                // }
                // if partitions[second].is_none() {
                //     panic!("partition {} is None, midx: {matching_idx}", second);
                // }
                let first_partition = partitions[first].take().unwrap();
                let second_partition = partitions[second].take().unwrap();
                paired_partitions.push(((first_partition, first), (second_partition, second)));
            }

            thread::scope(|s| {
                for ((first_partition, first), (second_partition, second_idx)) in paired_partitions.iter_mut() {
                    // let (first, second) = (first.min(second), first.max(second));
                    // let (partitions1, partitions2) = partitions.split_at_mut(second);
                    // let first_partition = &mut partitions1[first];
                    // let second_partition = &mut partitions2[0];

                    // let first_partition = first_partition.as_mut().unwrap();
                    // let second_partition = second_partition.as_mut().unwrap();


                    // let shg1 = &shgs[first];
                    let shg2 = &shgs[*second_idx];
                    s.spawn(|| {
                        // let first_partition = first_partition as *const Vec<Entity>;
                        // let second_partition = second_partition as *const Vec<Entity>;
                        // let first_partition: &mut Vec<Entity> = unsafe { &mut *(first_partition as *mut _) };
                        // let second_partition: &mut Vec<Entity> = unsafe { &mut *(second_partition as *mut _) };
                        for idx1 in 0..first_partition.len() {
                            for &idx2 in shg2.get_for_aabb(first_partition[idx1].get_aabb()) {
                                let entity1 = &mut first_partition[idx1];
                                let entity2 = &mut second_partition[idx2];
                                // check collision
                                if let Some(dist) = entity1.collides_with(entity2) {
                                    entity1.handle_collision(entity2, dist);
                                }
                            }
                        }
                    });
                }

            });

            // reinsert
            for ((first_partition, first), (second_partition, second)) in paired_partitions {
                partitions[first] = Some(first_partition);
                partitions[second] = Some(second_partition);
            }


        }
        // for matching_idx in 0..num_matchings {
        //     let matching = get_matching(num_threads, matching_idx);
        //     for (first, second) in matching {
        //         let (first, second) = (first.min(second), first.max(second));
        //         let (partitions1, partitions2) = partitions.split_at_mut(second);
        //         let first_partition = &mut partitions1[first];
        //         let second_partition = &mut partitions2[0];
        //
        //         // let shg1 = &shgs[first];
        //         let shg2 = &shgs[second];
        //         for idx1 in 0..first_partition.len() {
        //             for &idx2 in shg2.get_for_aabb(first_partition[idx1].get_aabb()) {
        //                 let entity1 = &mut first_partition[idx1];
        //                 let entity2 = &mut second_partition[idx2];
        //                 // check collision
        //                 if let Some(dist) = entity1.collides_with(entity2) {
        //                     entity1.handle_collision(entity2, dist);
        //                 }
        //             }
        //         }
        //     }
        //
        // }

        // move them back into the state
        // state.entities.clear();
        // for partition in partitions {
        //     state.entities.extend(partition.unwrap());
        // }

    }

    fn update_physics(&self, dt: f64, state: &mut GameState) {
        // Step 1: Update all entities individually, handle world collisions
        for entity in state.entities.iter_mut() {
            // Verlet
            entity.update(dt);
            // handle world bounds and collisions
            entity.handle_world_collisions(state.world_width, state.world_height);
        }
        // state.entities.par_iter_mut().for_each(|entity| {
        //     // Verlet
        //     entity.update(dt);
        //     // handle world bounds and collisions
        //     entity.handle_world_collisions(state.world_width, state.world_height);
        // });


        // Step 2: Handle entity-entity collisions
        // self.entent_basic(dt, state);
        // self.entent_shg(dt, state);
        self.entent_shg_multithreaded(dt, state);
    }
}

impl Component<GameState> for PhysicsComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        let mut total_iterations = 0;
        let mut total_duration_secs = 0.0;

        let dt = update_info.dt;
        self.fur.fuel(dt);
        while self.fur.has_gas() {
            self.fur.consume();
            let physics_dt = self.fur.fixed_dt();
            total_iterations += 1;
            let start = std::time::Instant::now();
            self.update_physics(physics_dt, &mut shared_state.custom);
            let duration = start.elapsed();
            total_duration_secs += duration.as_secs_f64();
        }
        if total_iterations > 0 {
            let avg = total_duration_secs / (total_iterations as f64);
            shared_state.debug_info.custom.insert(
                "average_physics_tick_ms_cost".to_string(),
                format!(
                    "{:.5}",
                    avg * 1000.0
                ),
            );
            if avg > self.fur.fixed_dt() {
                let key = "entity_len_at_first_slow_physics_tick";
                if !shared_state.debug_info.custom.contains_key(key) {
                    shared_state.debug_info.custom.insert(key.to_string(), shared_state.custom.entities.len().to_string());
                }
                // Log:
                // first impl, no shg: at 5400 entities.
            }
        }
    }
}

struct GameComponent {
    hbd: HalfBlockDisplayRender,
}

impl GameComponent {
    fn new() -> Self {
        Self {
            hbd: HalfBlockDisplayRender::new(0, 0),
        }
    }
}

impl Component<GameState> for GameComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<GameState>) {
        self.on_resize(
            setup_info.display_info.width(),
            setup_info.display_info.height(),
            shared_state,
        );
    }

    fn on_resize(
        &mut self,
        width: usize,
        height: usize,
        shared_state: &mut SharedState<GameState>,
    ) {
        self.hbd.resize_discard(width, 2 * height);
        shared_state.custom.world_width = width as f64;
        shared_state.custom.world_height = 2.0 * height as f64;
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        let height = self.hbd.height() as i64;
        let width = self.hbd.width() as i64;

        let (mouse_x, mouse_y) = shared_state.mouse_info.last_mouse_pos;
        let mouse_x = mouse_x as i64;
        let mouse_y = mouse_y as i64 * 2;
        let mouse_y = height - mouse_y;

        // add entity on mouse click
        if shared_state.mouse_info.left_mouse_down {
            // spawn 100 in a radius of 3 around the mouse
            for _ in 0..100 {
                let x = mouse_x + (rand::random::<i64>() % 3);
                let y = mouse_y + (rand::random::<i64>() % 3);
                shared_state
                    .custom
                    .entities
                    // .push(Entity::new_at(x as f64, y as f64).with_velocity((60.0, 0.0).into()).with_radius(2.0));
                .push(Entity::new_at(x as f64, y as f64).with_velocity((60.0, 0.0).into()));
            }
            shared_state.debug_info.custom.insert(
                "total entities".to_string(),
                format!("{}", shared_state.custom.entities.len()),
            );
        }

        // handle keyboard
        if shared_state.pressed_keys.did_press_char_ignore_case('c') {
            shared_state.custom.entities.clear();
        }

        // render entities
        self.hbd.clear();
        for entity in shared_state.custom.entities.iter() {
            let (x, y) = entity.pos.floor_to_i64();
            // swap y axis, entity y grows upwards
            let y = height - y;
            // ignore oob
            if x < 0 || x >= width || y < 0 || y >= height {
                continue;
            }
            self.hbd
                .set_color(x as usize, y as usize, Color::Rgb([255, 0, 0]));
        }
    }

    fn render(
        &self,
        renderer: &mut dyn Renderer,
        shared_state: &SharedState<GameState>,
        depth_base: i32,
    ) {
        self.hbd.render(renderer, 0, 0, depth_base);
    }
}

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();
    // we need to exit on panic, see TODO in teng::install_panic_handler
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        old_hook(panic_info);
        std::process::exit(1);
    }));

    let mut game = Game::new_with_custom_buf_writer();
    game.install_recommended_components();
    game.add_component(Box::new(GameComponent::new()));
    game.add_component(Box::new(PhysicsComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
