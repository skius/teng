use std::time::{Duration, Instant};
use crossterm::event::{Event, KeyCode};
use crate::game::components::incremental::collisionboard::PhysicsEntity2d;
use crate::game::{BreakingAction, Component, DebugMessage, Render, Renderer, SharedState, Sprite, UpdateInfo};
use crate::game::components::incremental::{GamePhase, GameState, PlayerState};
use crate::game::components::incremental::animation::CharAnimationSequence;

pub struct NewPlayerComponent {
    entity: PhysicsEntity2d,
    sprite: Sprite<3, 2>,
    dead_sprite: Sprite<5, 1>,
    render_sensors: bool,
    max_height_since_ground: f64,
    dead_time: Option<Instant>,
    next_sample_time: Instant,
}

impl NewPlayerComponent {
    const DEATH_HEIGHT: f64 = 3.5;
    const DEATH_RESPAWN_TIME: f64 = 2.0;
    const DEATH_STOP_X_MOVE_TIME: f64 = 0.5;

    pub fn new() -> Self {
        Self {
            entity: PhysicsEntity2d {
                position: (0.0, 0.0),
                velocity: (0.0, 0.0),
                size_top: 1.0,
                size_bottom: 0.0,
                size_left: 1.0,
                size_right: 1.0,
            },
            sprite: Sprite::new([['▁', '▄', '▁'], ['▗', '▀', '▖']], 1, 1),
            dead_sprite: Sprite::new([['▂', '▆', '▆', ' ', '▖']], 2, 0),
            render_sensors: false,
            max_height_since_ground: f64::MIN,
            dead_time: None,
            next_sample_time: Instant::now(),
        }
    }
}

impl NewPlayerComponent {
    fn spawn_ground_slam_animation(&self, game_state: &mut GameState) {
        // Add animation at collision point
        let x =self.entity.position.0.floor() as i64;
        let y = self.entity.position.1.floor() as i64;
        let animation1 = CharAnimationSequence {
            sequence: vec!['▄', '▟', '▞', '▝'],
            start_time: Instant::now(),
            time_per_frame: std::time::Duration::from_secs_f64(0.1),
        };
        game_state.world.add_animation(Box::new(animation1), x+2, y);
        let animation2 = CharAnimationSequence {
            sequence: vec!['▄', '▙', '▚', '▘'],
            start_time: Instant::now(),
            time_per_frame: std::time::Duration::from_secs_f64(0.1),
        };
        game_state.world.add_animation(Box::new(animation2), x-2, y);
    }

    fn on_death(&mut self, fall_distance: f64, yvel_before: f64, game_state: &mut GameState) {
        let current_time = Instant::now();
        self.dead_time = Some(current_time);

        self.dead_sprite = if self.entity.velocity.0 >= 0.0 {
            Sprite::new([['▂', '▆', '▆', ' ', '▖']], 2, 0)
        } else {
            Sprite::new([['▗', ' ', '▆', '▆', '▂']], 2, 0)
        };

        let blocks_f64 = fall_distance.abs().ceil()
            * game_state.upgrades.block_height as f64
            * game_state.upgrades.player_weight as f64
            * yvel_before.powf(game_state.upgrades.velocity_exponent);
        let blocks = blocks_f64.ceil() as usize;
        game_state.received_blocks += blocks;
        game_state.received_blocks_base += blocks;
    }

    fn horizontal_inputs(&mut self, pressed_keys: &micromap::Map<KeyCode, u8, 16>) {
        let slow_velocity = 10.0;
        let fast_velocity = 30.0;

        if pressed_keys.contains_key(&KeyCode::Char('a')) {
            self.entity.velocity.0 = if self.entity.velocity.0 > 0.0 { 0.0 } else { -slow_velocity };
        } else if pressed_keys.contains_key(&KeyCode::Char('d')) {
            self.entity.velocity.0 = if self.entity.velocity.0 < 0.0 { 0.0 } else { slow_velocity };
        }

        if pressed_keys.contains_key(&KeyCode::Char('A')) {
            self.entity.velocity.0 = if self.entity.velocity.0 > 0.0 { 0.0 } else { -fast_velocity };
        } else if pressed_keys.contains_key(&KeyCode::Char('D')) {
            self.entity.velocity.0 = if self.entity.velocity.0 < 0.0 { 0.0 } else { fast_velocity };
        }
    }
}

impl Component for NewPlayerComponent {
    fn is_active(&self, shared_state: &SharedState) -> bool {
        shared_state.extensions.get::<GameState>().unwrap().phase == GamePhase::Moving
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let current_time = update_info.current_time;
        let dt = current_time - update_info.last_time;
        let dt = dt.as_secs_f64();

        let game_state = shared_state.extensions.get_mut::<GameState>().unwrap();

        if let Some(dead_time) = self.dead_time {
            let time_since_death = (current_time - dead_time).as_secs_f64();
            if time_since_death >= Self::DEATH_STOP_X_MOVE_TIME {
                self.entity.velocity.0 = 0.0;
            }
            if time_since_death >= Self::DEATH_RESPAWN_TIME {
                game_state.phase = GamePhase::MoveToBuilding;
                self.dead_time = None;
                // for all ghosts that did not die, add 1 block
                for ghost in &game_state.player_ghosts {
                    if ghost.death_time.is_none() {
                        game_state.received_blocks += game_state.upgrades.ghost_cuteness;
                    }
                }
            }
        }

        if self.dead_time.is_none() {
            self.horizontal_inputs(&shared_state.pressed_keys);
        }

        let step_size = if self.dead_time.is_some() { 0 } else { 1 };
        let yvel_before = self.entity.velocity.1;
        let collision_info = self.entity.update(dt, step_size, &mut game_state.world.collision_board);

        if !collision_info.hit_bottom {
            self.max_height_since_ground = self.max_height_since_ground.max(self.entity.position.1);
        } else {
            let fall_distance = self.max_height_since_ground - self.entity.position.1;
            if fall_distance > 7.0 {
                self.spawn_ground_slam_animation(game_state);
            }

            if fall_distance >= Self::DEATH_HEIGHT {
                self.on_death(fall_distance, yvel_before, game_state);
            }

            self.max_height_since_ground = self.entity.position.1;
        }

        if self.dead_time.is_none() {
            // Now jump input since we need grounded information
            if shared_state.pressed_keys.contains_key(&KeyCode::Char(' ')) {
                if self.entity.grounded(&mut game_state.world.collision_board) {
                    self.entity.velocity.1 = 20.0;
                }
            }
        }

        // Update camera
        game_state.world.camera_follow(self.entity.position.0.floor() as i64, self.entity.position.1.floor() as i64);

    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        let player_world_x = self.entity.position.0.floor() as i64;
        let player_world_y = self.entity.position.1.floor() as i64;

        let player_screen_pos = game_state.world.to_screen_pos(player_world_x, player_world_y);
        if let Some((x,y)) = player_screen_pos {
            if self.dead_time.is_some() {
                self.dead_sprite.render(&mut renderer, x, y, depth_base);
            } else {
                self.sprite.render(&mut renderer, x, y, depth_base);
            }
        }

        if self.render_sensors {
            let sensors = [
                self.entity.right_sensor(),
                self.entity.left_sensor(),
                self.entity.top_sensor(),
                self.entity.floor_sensor(),
            ];
            for bounds in sensors {
                for x in bounds.min_x..=bounds.max_x {
                    for y in bounds.min_y..=bounds.max_y {
                        let screen_pos = game_state.world.to_screen_pos(x, y);
                        if let Some((x, y)) = screen_pos {
                            '░'.with_color([0,0,200]).render(&mut renderer, x, y, depth_base + 1);
                        }
                    }
                }
            }

            // also render position of entity
            // let entity_bb = self.entity.bounding_box();
            // for x in entity_bb.min_x..=entity_bb.max_x {
            //     for y in entity_bb.min_y..=entity_bb.max_y {
            //         let screen_pos = game_state.world.to_screen_pos(x, y);
            //         if let Some((x, y)) = screen_pos {
            //             '█'.with_color([200,50,50]).render(&mut renderer, x, y, depth_base + 1);
            //         }
            //     }
            // }
            // let screen_pos = game_state.world.to_screen_pos(player_world_x, player_world_y);
            // if let Some((x, y)) = screen_pos {
            //     'X'.with_color([200,0,0]).render(&mut renderer, x, y, depth_base + 2);
            // }
        }
    }
}