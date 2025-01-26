use std::time::Instant;
use crossterm::event::{Event, KeyCode};
use crate::game::components::incremental::collisionboard::PhysicsEntity2d;
use crate::game::{BreakingAction, Component, DebugMessage, Render, Renderer, SharedState, Sprite, UpdateInfo};
use crate::game::components::incremental::{GameState, PlayerState};
use crate::game::components::incremental::animation::CharAnimationSequence;
// pub struct PlayerState {
//     entity: PhysicsEntity2d,
//     sprite: Sprite<3, 2>,
//     dead_sprite: Sprite<5, 1>,
//     dead_time: Option<Instant>,
//     render_sensors: bool,
//     max_height_since_ground: f64,
//     next_sample_time: Instant,
// }

pub struct NewPlayerComponent {
    entity: PhysicsEntity2d,
    sprite: Sprite<3, 2>,
    render_sensors: bool,
    max_height_since_ground: f64,
}

impl NewPlayerComponent {
    pub const STEP_SIZE: i64 = 1;

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
            // dead_sprite: Sprite::new([['▂', '▆', '▆', ' ', '▖']], 2, 0),
            render_sensors: false,
            max_height_since_ground: f64::MIN,
        }
    }
}

impl Component for NewPlayerComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let dt = update_info.current_time - update_info.last_time;
        let dt = dt.as_secs_f64();

        let game_state = shared_state.extensions.get_mut::<GameState>().unwrap();

        let slow_velocity = 10.0;
        let fast_velocity = 30.0;

        if shared_state.pressed_keys.contains_key(&KeyCode::Char('a')) {
            if self.entity.velocity.0 > 0.0 {
                self.entity.velocity.0 = 0.0;
            } else if self.entity.velocity.0 == 0.0 {
                self.entity.velocity.0 = -slow_velocity;
            } else {
                self.entity.velocity.0 = -slow_velocity;
            }
        } else if shared_state.pressed_keys.contains_key(&KeyCode::Char('d')) {
            if self.entity.velocity.0 < 0.0 {
                self.entity.velocity.0 = 0.0;
            } else if self.entity.velocity.0 == 0.0 {
                self.entity.velocity.0 = slow_velocity;
            } else {
                self.entity.velocity.0 = slow_velocity;
            }
        }
        if shared_state.pressed_keys.contains_key(&KeyCode::Char('A')) {
            if self.entity.velocity.0 > 0.0 {
                self.entity.velocity.0 = 0.0;
            } else if self.entity.velocity.0 == 0.0 {
                self.entity.velocity.0 = -fast_velocity;
            } else {
                self.entity.velocity.0 = -fast_velocity;
            }
        } else if shared_state.pressed_keys.contains_key(&KeyCode::Char('D')) {
            if self.entity.velocity.0 < 0.0 {
                self.entity.velocity.0 = 0.0;
            } else if self.entity.velocity.0 == 0.0 {
                self.entity.velocity.0 = fast_velocity;
            } else {
                self.entity.velocity.0 = fast_velocity;
            }
        }

        let collision_info = self.entity.update(dt, &mut game_state.world.collision_board);

        if !collision_info.hit_bottom {
            self.max_height_since_ground = self.max_height_since_ground.max(self.entity.position.1);
        } else {
            let fall_distance = self.max_height_since_ground - self.entity.position.1;
            if fall_distance > 4.0 {
                shared_state.debug_messages.push(DebugMessage::new(format!("You fell {} units", fall_distance), Instant::now() + std::time::Duration::from_secs(2)));
            }
            if fall_distance > 7.0 {
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
            self.max_height_since_ground = self.entity.position.1;

        }

        // Now jump input since we need grounded information
        if shared_state.pressed_keys.contains_key(&KeyCode::Char(' ')) {
            if self.entity.grounded(&mut game_state.world.collision_board) {
                self.entity.velocity.1 = 20.0;
            }
        }

        // Update camera
        // The camera should move if the player is less than 30% from the edge of the screen
        let camera_bounds = game_state.world.camera_window();
        let camera_width = camera_bounds.max_x - camera_bounds.min_x;
        let camera_height = camera_bounds.max_y - camera_bounds.min_y;
        let x_threshold = (camera_width as f64 * 0.3) as i64;
        let y_threshold = (camera_height as f64 * 0.3) as i64;
        let player_world_x = self.entity.position.0 as i64;
        let player_world_y = self.entity.position.1 as i64;
        if player_world_x < camera_bounds.min_x + x_threshold {
            let move_by = camera_bounds.min_x - player_world_x + x_threshold;
            game_state.world.move_camera(-move_by, 0);
        } else if player_world_x > camera_bounds.max_x - x_threshold {
            let move_by = player_world_x - camera_bounds.max_x + x_threshold;
            game_state.world.move_camera(move_by, 0);
        }
        if player_world_y < camera_bounds.min_y + y_threshold {
            let move_by = camera_bounds.min_y - player_world_y + y_threshold;
            game_state.world.move_camera(0, -move_by);
        } else if player_world_y > camera_bounds.max_y - y_threshold {
            let move_by = player_world_y - camera_bounds.max_y + y_threshold;
            game_state.world.move_camera(0, move_by);
        }

    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        let player_world_x = self.entity.position.0.floor() as i64;
        let player_world_y = self.entity.position.1.floor() as i64;

        let player_screen_pos = game_state.world.to_screen_pos(player_world_x, player_world_y);
        if let Some((x,y)) = player_screen_pos {
            self.sprite.render(&mut renderer, x, y, depth_base);
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