use crate::components::incremental::animation::CharAnimationSequence;
use crate::components::incremental::collisionboard::PhysicsEntity2d;
use crate::components::incremental::world::World;
use crate::components::incremental::{GamePhase, GameState, PlayerHistoryElement};
use crate::rendering::render::{Render, Sprite};
use crate::{Component, Renderer, SharedState, UpdateInfo};
use std::time::{Duration, Instant};
use crate::components::keyboard::PressedKeys;

#[derive(Debug)]
pub struct NewPlayerState {
    pub entity: PhysicsEntity2d,
    pub sprite: Sprite<3, 2>,
    pub dead_sprite: Sprite<5, 1>,
    pub render_sensors: bool,
    pub max_height_since_ground: f64,
    pub dead_time: Option<Instant>,
    pub next_sample_time: Instant,
    pub paused: bool,
}

impl NewPlayerState {
    pub fn new() -> Self {
        Self {
            entity: PhysicsEntity2d {
                position: (0.0, 0.0),
                velocity: (0.0, 0.0),
                size_top: 1.0,
                size_bottom: 0.0,
                size_left: 1.0,
                size_right: 1.0,
                y_accel: -40.0,
                x_drag: 1.0,
            },
            sprite: Sprite::new([['▁', '▄', '▁'], ['▗', '▀', '▖']], 1, 1),
            dead_sprite: Sprite::new([['▂', '▆', '▆', ' ', '▖']], 2, 0),
            render_sensors: false,
            max_height_since_ground: f64::MIN,
            dead_time: None,
            next_sample_time: Instant::now(),
            paused: false,
        }
    }

    fn spawn_ground_slam_animation(&self, world: &mut World) {
        // Add animation at collision point
        let x = self.entity.position.0.floor() as i64;
        let y = self.entity.position.1.floor() as i64;
        let animation1 = CharAnimationSequence {
            sequence: vec!['▄', '▟', '▞', '▝'],
            start_time: Instant::now(),
            time_per_frame: std::time::Duration::from_secs_f64(0.1),
        };
        world.add_animation(Box::new(animation1), x + 2, y);
        let animation2 = CharAnimationSequence {
            sequence: vec!['▄', '▙', '▚', '▘'],
            start_time: Instant::now(),
            time_per_frame: std::time::Duration::from_secs_f64(0.1),
        };
        world.add_animation(Box::new(animation2), x - 2, y);
    }

    fn on_death(fall_distance: f64, yvel_before: f64, game_state: &mut GameState) {
        let player = &mut game_state.new_player_state;

        let current_time = Instant::now();
        player.dead_time = Some(current_time);

        // make drag high
        player.entity.x_drag = 0.2;

        player.dead_sprite = if player.entity.velocity.0 >= 0.0 {
            Sprite::new([['▂', '▆', '▆', ' ', '▖']], 2, 0)
        } else {
            Sprite::new([['▗', ' ', '▆', '▆', '▂']], 2, 0)
        };

        let blocks_f64 = fall_distance.abs().ceil()
            * game_state.upgrades.block_height as f64
            * game_state.upgrades.player_weight as f64
            * yvel_before
                .abs()
                .powf(game_state.upgrades.velocity_exponent);
        let blocks = blocks_f64.ceil() as u128;
        game_state.received_blocks += blocks;
        game_state.received_blocks_base += blocks;
    }

    fn horizontal_inputs(&mut self, pressed_keys: &PressedKeys) {
        let slow_velocity = 10.0;
        let fast_velocity = 30.0;

        let mut did_move = false;

        if pressed_keys.did_press_char('a') {
            self.entity.velocity.0 = if self.entity.velocity.0 > 0.0 {
                0.0
            } else {
                -slow_velocity
            };
            did_move = true;
        } else if pressed_keys.did_press_char('d') {
            self.entity.velocity.0 = if self.entity.velocity.0 < 0.0 {
                0.0
            } else {
                slow_velocity
            };
            did_move = true;
        }

        if pressed_keys.did_press_char('A') {
            self.entity.velocity.0 = if self.entity.velocity.0 > 0.0 {
                0.0
            } else {
                -fast_velocity
            };
            did_move = true;
        } else if pressed_keys.did_press_char('D') {
            self.entity.velocity.0 = if self.entity.velocity.0 < 0.0 {
                0.0
            } else {
                fast_velocity
            };
            did_move = true;
        }

        if did_move {
            // reset drag
            self.entity.x_drag = 1.0;
        }
    }
}

pub struct NewPlayerComponent {}

impl NewPlayerComponent {
    const DEATH_HEIGHT: f64 = 3.5;
    const DEATH_RESPAWN_TIME: f64 = 2.0;

    pub fn new() -> Self {
        Self {}
    }
}

impl<S> Component<S> for NewPlayerComponent {
    fn is_active(&self, shared_state: &SharedState<S>) -> bool {
        shared_state.extensions.get::<GameState>().unwrap().phase == GamePhase::Moving
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        let current_time = update_info.current_time;
        let dt = update_info.dt;

        let game_state = shared_state.extensions.get_mut::<GameState>().unwrap();

        if let Some(dead_time) = game_state.new_player_state.dead_time {
            let time_since_death = (current_time - dead_time).as_secs_f64();
            // if time_since_death >= Self::DEATH_STOP_X_MOVE_TIME {
            //     game_state.new_player_state.entity.velocity.0 = 0.0;
            // }
            if time_since_death >= Self::DEATH_RESPAWN_TIME {
                game_state.phase = GamePhase::MoveToBuilding;
                game_state.new_player_state.dead_time = None;
                game_state.new_player_state.entity.x_drag = 1.0;
                game_state.new_player_state.entity.velocity = (0.0, 0.0);
                // for all ghosts that did not die, add 1 block
                for ghost in &game_state.player_ghosts {
                    if ghost.death_time.is_none() {
                        game_state.received_blocks += game_state.upgrades.ghost_cuteness;
                    }
                }
            }
        }

        if game_state.new_player_state.dead_time.is_none() {
            game_state
                .new_player_state
                .horizontal_inputs(&shared_state.pressed_keys);
        }

        let y_accel = if game_state.new_player_state.entity.velocity.1 < 0.0 {
            -40.0 * game_state.upgrades.fall_speed_factor
        } else {
            -40.0
        };
        game_state.new_player_state.entity.y_accel = y_accel;

        // only update if not paused
        if !game_state.new_player_state.paused {
            // {
            //     // test responsiveness of debounced keys
            //     // result: another problem is also that there is at most one down key at a time,
            //     // because the terminal will not press-repeat multiple keys, only the latest one.
            //     if shared_state.debounced_down_keys.contains(&KeyCode::Right) {
            //         game_state.new_player_state.entity.position.0 += 1.0;
            //     }
            //     if shared_state.debounced_down_keys.contains(&KeyCode::Left) {
            //         game_state.new_player_state.entity.position.0 -= 1.0;
            //     }
            //     if shared_state.debounced_down_keys.contains(&KeyCode::Up) {
            //         game_state.new_player_state.entity.position.1 += 1.0;
            //     }
            //     if shared_state.debounced_down_keys.contains(&KeyCode::Down) {
            //         game_state.new_player_state.entity.position.1 -= 1.0;
            //     }
            //
            // }

            let step_size = if game_state.new_player_state.dead_time.is_some() {
                0
            } else {
                1
            };
            let yvel_before = game_state.new_player_state.entity.velocity.1;
            let collision_info = game_state.new_player_state.entity.update(
                dt,
                step_size,
                &mut game_state.world.collision_board,
            );

            if !collision_info.hit_bottom {
                game_state.new_player_state.max_height_since_ground = game_state
                    .new_player_state
                    .max_height_since_ground
                    .max(game_state.new_player_state.entity.position.1);
            } else {
                let fall_distance = game_state.new_player_state.max_height_since_ground
                    - game_state.new_player_state.entity.position.1;
                if fall_distance > 7.0 {
                    game_state
                        .new_player_state
                        .spawn_ground_slam_animation(&mut game_state.world);
                }

                if fall_distance >= Self::DEATH_HEIGHT
                    && game_state.new_player_state.dead_time.is_none()
                {
                    NewPlayerState::on_death(fall_distance, yvel_before, game_state);
                }

                game_state.new_player_state.max_height_since_ground =
                    game_state.new_player_state.entity.position.1;
            }

            if game_state.new_player_state.dead_time.is_none() {
                // Now jump input since we need grounded information
                if shared_state.pressed_keys.did_press_char(' ') {
                    if game_state
                        .new_player_state
                        .entity
                        .grounded(&mut game_state.world.collision_board)
                    {
                        game_state.new_player_state.entity.velocity.1 =
                            20.0 * game_state.upgrades.player_jump_boost_factor;
                    }
                }
            }
        }

        // Update camera
        game_state.world.camera_follow(
            game_state.new_player_state.entity.position.0.floor() as i64,
            game_state.new_player_state.entity.position.1.floor() as i64,
        );

        if current_time >= game_state.new_player_state.next_sample_time {
            game_state.new_player_state.next_sample_time =
                game_state.new_player_state.next_sample_time
                    + Duration::from_secs_f64(1.0 / PlayerGhost::SAMPLE_RATE);
            let phe = PlayerHistoryElement {
                x: game_state.new_player_state.entity.position.0.floor() as i64,
                y: game_state.new_player_state.entity.position.1.floor() as i64,
                dead: game_state.new_player_state.dead_time.is_some(),
            };
            game_state.player_history.push(phe);

            // only keep as many history samples as we need according to the amount of ghosts we have
            // and their delay
            let max_history_size_needed =
                game_state.player_ghosts.len() as f64 * game_state.curr_ghost_delay;

            let current_history_size_secs =
                game_state.player_history.len() as f64 / PlayerGhost::SAMPLE_RATE;

            if current_history_size_secs > max_history_size_needed.ceil() {
                game_state.player_history.remove(0);
            }
        }
        for ghost in &mut game_state.player_ghosts {
            let (just_died, _expired) = ghost.update(&game_state.player_history);
            if just_died {
                game_state.received_blocks += game_state.received_blocks_base;
            }
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<S>, depth_base: i32) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        let player = &game_state.new_player_state;
        let player_world_x = player.entity.position.0.floor() as i64;
        let player_world_y = player.entity.position.1.floor() as i64;

        let ghost_depth = depth_base;
        let player_depth = ghost_depth + 1;
        let debug_depth = player_depth + 1;

        let player_screen_pos = game_state
            .world
            .to_screen_pos(player_world_x, player_world_y);
        if let Some((x, y)) = player_screen_pos {
            if player.dead_time.is_some() {
                player.dead_sprite.render(renderer, x, y, player_depth);
            } else {
                player.sprite.render(renderer, x, y, player_depth);
            }
        }

        for ghost in &game_state.player_ghosts {
            ghost.render(
                renderer,
                shared_state,
                ghost_depth,
                &game_state.new_player_state.sprite,
                &game_state.new_player_state.dead_sprite,
            );
        }

        if player.render_sensors {
            let sensors = [
                player.entity.right_sensor(),
                player.entity.left_sensor(),
                player.entity.top_sensor(),
                player.entity.floor_sensor(),
            ];
            for bounds in sensors {
                for x in bounds.min_x..=bounds.max_x {
                    for y in bounds.min_y..=bounds.max_y {
                        let screen_pos = game_state.world.to_screen_pos(x, y);
                        if let Some((x, y)) = screen_pos {
                            '░'
                                .with_color([0, 0, 200])
                                .render(renderer, x, y, debug_depth);
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
            //             '█'.with_color([200,50,50]).render(&mut renderer, x, y, debug_depth);
            //         }
            //     }
            // }
            // let screen_pos = game_state.world.to_screen_pos(player_world_x, player_world_y);
            // if let Some((x, y)) = screen_pos {
            //     'X'.with_color([200,0,0]).render(&mut renderer, x, y, debug_depth);
            // }
        }
    }
}

#[derive(Debug)]
pub struct PlayerGhost {
    pub offset_secs: f64,
    pub was_dead: bool,
    pub death_time: Option<Instant>,
}

impl PlayerGhost {
    const SAMPLE_RATE: f64 = 160.0;

    pub fn new(offset_secs: f64) -> Self {
        Self {
            offset_secs,
            was_dead: false,
            death_time: None,
        }
    }

    fn offset_as_samples(&self) -> usize {
        (self.offset_secs * Self::SAMPLE_RATE) as usize
    }

    // returns true if it just died, and if it expired
    fn update(&mut self, history: &[PlayerHistoryElement]) -> (bool, bool) {
        let current_time = Instant::now();
        let history_size = history.len();
        let offset_samples = self.offset_as_samples();
        if history_size <= offset_samples {
            // doesn't exist yet
            return (false, false);
        }
        let render_state = &history[history_size - offset_samples - 1];
        let dead = render_state.dead;
        let just_died = dead && !self.was_dead;
        self.was_dead = dead;
        if just_died {
            self.death_time = Some(current_time);
        }

        let expired = if let Some(death_time) = self.death_time {
            let time_since_death = (current_time - death_time).as_secs_f64();
            time_since_death >= NewPlayerComponent::DEATH_RESPAWN_TIME
        } else {
            false
        };

        (just_died, expired)
    }

    fn render<S>(
        &self,
        renderer: &mut dyn Renderer,
        shared_state: &SharedState<S>,
        depth_base: i32,
        player_sprite: &Sprite<3, 2>,
        death_sprite: &Sprite<5, 1>,
    ) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        let current_time = Instant::now();
        let history = &game_state.player_history;
        let history_size = history.len();
        let offset_samples = self.offset_as_samples();
        if history_size <= offset_samples {
            return;
        }
        let render_state = &history[history_size - offset_samples - 1];
        let cuteness = game_state.upgrades.ghost_cuteness;

        let screen_pos = game_state
            .world
            .to_screen_pos(render_state.x, render_state.y);

        if let Some((x, y)) = screen_pos {
            if render_state.dead {
                death_sprite
                    .with_color([130, 130, 130])
                    .render(renderer, x, y, depth_base);
            } else {
                player_sprite
                    .with_color([130u8.saturating_add(cuteness as u8), 130, 130])
                    .render(renderer, x, y, depth_base);
            }
        }
    }
}
