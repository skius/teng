use std::time::Instant;
use crossterm::event::KeyCode;
use crate::game::components::Bullet;
use crate::game::{Component, Pixel, Render, Renderer, SharedState, Sprite, UpdateInfo};

pub struct PlayerComponent {
    x: f64,
    y: f64,
    x_vel: f64,
    y_vel: f64,
    sprite: Sprite<3, 2>,
    dead_sprite: Sprite<5, 1>,
    dead_time: Option<Instant>,
    max_height_since_last_ground_touch: f64,
}

impl PlayerComponent {
    const DEATH_HEIGHT: f64 = 25.0;
    const DEATH_RESPAWN_TIME: f64 = 2.0;
    const DEATH_STOP_X_MOVE_TIME: f64 = 1.0;

    pub fn new(x: usize, y: usize) -> Self {
        Self {
            x: x as f64,
            y: y as f64,
            x_vel: 0.0,
            y_vel: 0.0,
            sprite: Sprite::new([['▁', '▄', '▁'], ['▗', '▀', '▖']], 1, 1),
            dead_sprite: Sprite::new([['▂', '▆', '▆', ' ', '▖']], 2, 0),
            dead_time: None,
            max_height_since_last_ground_touch: y as f64,
        }
    }
}

impl Component for PlayerComponent {
    // fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
    //     match event {
    //         Event::Key(ke) => {
    //             assert_eq!(ke.kind, crossterm::event::KeyEventKind::Press);
    //             match ke.code {
    //                 KeyCode::Char('w' | 'W') => {
    //                     self.y = self.y.saturating_sub(1);
    //                 }
    //                 KeyCode::Char('s' | 'S') => {
    //                     self.y = self.y.saturating_add(1);
    //                 }
    //                 KeyCode::Char(c@('a' | 'A')) => {
    //                     self.x = self.x.saturating_sub(1 + c.is_ascii_uppercase() as usize);
    //                 }
    //                 KeyCode::Char(c@('d' | 'D')) => {
    //                     self.x = self.x.saturating_add(1 + c.is_ascii_uppercase() as usize);
    //                 }
    //                 _ => {}
    //             }
    //         }
    //         _ => {}
    //     }
    //     None
    // }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let current_time = update_info.current_time;

        if shared_state.pressed_keys.contains_key(&KeyCode::Char('k')) {
            self.dead_time = Some(current_time);
        }

        if let Some(dead_time) = self.dead_time {
            let time_since_death = (current_time - dead_time).as_secs_f64();
            if time_since_death >= Self::DEATH_STOP_X_MOVE_TIME {
                self.x_vel = 0.0;
            }
            if time_since_death >= Self::DEATH_RESPAWN_TIME {
                self.dead_time = None;
            }
        }

        let dt = update_info
            .current_time
            .saturating_duration_since(update_info.last_time)
            .as_secs_f64();

        // Player inputs, only if not dead
        if self.dead_time.is_none() {
            if shared_state.pressed_keys.contains_key(&KeyCode::Char('a')) {
                if self.x_vel > 0.0 {
                    self.x_vel = 0.0;
                } else if self.x_vel == 0.0 {
                    self.x_vel = -10.0;
                } else {
                    self.x_vel = -10.0;
                }
            } else if shared_state.pressed_keys.contains_key(&KeyCode::Char('d')) {
                if self.x_vel < 0.0 {
                    self.x_vel = 0.0;
                } else if self.x_vel == 0.0 {
                    self.x_vel = 10.0;
                } else {
                    self.x_vel = 10.0;
                }
            }
            if shared_state.pressed_keys.contains_key(&KeyCode::Char('A')) {
                if self.x_vel > 0.0 {
                    self.x_vel = 0.0;
                } else if self.x_vel == 0.0 {
                    self.x_vel = -20.0;
                } else {
                    self.x_vel = -20.0
                }
            } else if shared_state.pressed_keys.contains_key(&KeyCode::Char('D')) {
                if self.x_vel < 0.0 {
                    self.x_vel = 0.0;
                } else if self.x_vel == 0.0 {
                    self.x_vel = 20.0;
                } else {
                    self.x_vel = 20.0;
                }
            }
        }

        // Player physics
        let height = shared_state.display_info.height() as f64;
        let width = shared_state.display_info.width() as f64;

        let gravity = 40.0;

        self.y_vel += gravity * dt;
        self.x += self.x_vel * dt;

        let mut bottom_wall = height;
        let mut left_wall = 0.0f64;
        let mut right_wall = width;
        let mut left_idx = None;
        let mut right_idx = None;
        let step_size = 1;

        // find a physics entity below us
        let mut x_u = self.x.floor() as usize;
        let mut y_u = self.y.floor() as usize;
        if y_u >= height as usize {
            y_u = height as usize - 1;
        }


        {
            // Check left
            let x = x_u as i32 - 1;
            for y in (y_u-1)..=y_u {
                for x in ((x-4)..=x).rev() {
                    if x < 0 || x >= width as i32 {
                        break;
                    }
                    if shared_state.collision_board[(x as usize, y)] {
                        if left_wall < x as f64 + 1.0 {
                            left_idx = Some(x as usize);
                            left_wall = x as f64 + 1.0;// plus 1.0 because we define collision on <x differently?
                        }
                        break;
                    }
                }

            }
        }
        {
            // Check right
            let x = x_u as i32 + 1;
            for y in (y_u-1)..=y_u {
                for x in x..=(x+4) {
                    if x < 0 || x >= width as i32 {
                        break;
                    }
                    if shared_state.collision_board[(x as usize, y)] {
                        if right_wall > x as f64 {
                            right_idx = Some(x as usize);
                            right_wall = x as f64;
                        }
                        break;
                    }
                }
            }
        }

        // -1.0 etc to account for size of sprite
        if self.x-1.0 < left_wall {
            // Check if we can do a step
            // initialize false because if there is no left_idx, we can't do a step
            let mut do_step = false;
            if let Some(left_idx) = left_idx {
                for base_check in 0..step_size {
                    // if there is one, we assume true
                    do_step = true;
                    let check_y = self.y.floor() as usize - 1 - base_check;
                    // TODO: saturation
                    for y in (check_y - 1)..=check_y {
                        if shared_state.collision_board[(left_idx, y)] {
                            do_step = false;
                            break;
                        }
                    }
                    if do_step {
                        break;
                    }
                }

            }
            if !do_step {
                self.x = left_wall+1.0;
            }
            // self.x_vel = 0.0;
        } else if self.x+1.0 >= right_wall {
            // Check if we can do a step
            let mut do_step = false;
            if let Some(right_idx) = right_idx {
                for base_check in 0..step_size {
                    do_step = true;
                    let check_y = self.y.floor() as usize - 1 - base_check;
                    for y in (check_y - 1)..=check_y {
                        if shared_state.collision_board[(right_idx, y)] {
                            do_step = false;
                            break;
                        }
                    }
                    if do_step {
                        break;
                    }
                }

            }
            if !do_step {
                self.x = right_wall - 2.0;
            }

            // self.x_vel = 0.0;
        }

        // need to update for bottom checking, since x checking can clamp x and change the bottom check result
        let mut x_u = self.x.floor() as usize;
        // and only update y here, because otherwise x checking will think we're inside the floor block
        self.y += self.y_vel * dt;
        let mut y_u = self.y.floor() as usize;
        if y_u >= height as usize {
            y_u = height as usize - 1;
        }

        {
            // Check below
            let x = x_u as i32;
            let y = y_u;

            // TODO: should be dynamic due to sprite size
            for x in (x-1)..=(x+1) {
                if x < 0 || x >= width as i32 {
                    continue;
                }
                for y in y..(height as usize).min(y + 4) as usize {
                    if shared_state.collision_board[(x as usize, y)] {
                        bottom_wall = bottom_wall.min(y as f64);
                        break;
                    }
                }
            }
        }

        // TODO: sprite size should be taken into account for top wall checking
        if self.y < 0.0 {
            self.y = 0.0;
            self.y_vel = 0.0;
        } else if self.y >= bottom_wall {
            self.y = bottom_wall - 1.0;
            // if we're going up, don't douch the jump velocity.
            if self.y_vel >= 0.0 {
                self.y_vel = 0.0;
            }

        }

        let grounded = self.y >= bottom_wall - 1.2;
        if !grounded {
            self.max_height_since_last_ground_touch = self.max_height_since_last_ground_touch.min(self.y);
        } else {
            if self.y - self.max_height_since_last_ground_touch > Self::DEATH_HEIGHT {
                self.dead_time = Some(current_time);
            }
            self.max_height_since_last_ground_touch = self.y;
        }

        // Now jump input since we need grounded information
        if shared_state.pressed_keys.contains_key(&KeyCode::Char(' ')) {
            if grounded {
                self.y_vel = -20.0;
            }
        }
        shared_state.debug_info.player_y = self.y;
        shared_state.debug_info.player_x = self.x;
        shared_state.debug_info.left_wall = left_wall;
        shared_state.debug_info.bottom_wall = bottom_wall;
        shared_state.debug_info.y_vel = self.y_vel;
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        if self.dead_time.is_some() {
            self.dead_sprite
                .render(&mut renderer, self.x.floor() as usize, self.y.floor() as usize, depth_base);
        } else {
            self.sprite
                .render(&mut renderer, self.x.floor() as usize, self.y.floor() as usize, depth_base);
        }
    }
}