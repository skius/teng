//! Game description:
//! Your goal is to make the player fall from increasing heights and die.
//! Your current 'currency' is the amount of blocks you can place.
//!
//! There is a building phase, and a moving phase.
//! In the building phase you place the blocks you have available.
//! In the moving phase you walk around until you die, at which point you get more blocks for the next phase.
//! The game resets to the building phase and you're awarded all your blocks back plus the additional ones you earned.
//!
//! You start with no blocks, and the player's death height is barely enough to die when jumping.

use std::time::{Duration, Instant};
use crossterm::event::{Event, KeyCode};
use smallvec::SmallVec;
use crate::game::components::{Bullet, DecayElement, MouseTrackerComponent};
use crate::game::{BreakingAction, Component, DebugMessage, MouseInfo, Pixel, Render, Renderer, SetupInfo, SharedState, Sprite, UpdateInfo, WithColor};

#[derive(Default, Debug, PartialEq)]
enum GamePhase {
    #[default]
    MoveToBuilding,
    Building,
    BuildingToMoving,
    Moving,
}

#[derive(Debug)]
struct GameState {
    phase: GamePhase,
    blocks: usize,
    max_blocks: usize,
    received_blocks: usize,
    max_blocks_per_round: usize,
    player_state: PlayerState,
}

impl GameState {
    fn new(width: usize, height: usize) -> Self {
        Self {
            phase: GamePhase::default(),
            blocks: 0,
            max_blocks: 0,
            received_blocks: 0,
            max_blocks_per_round: 0,
            player_state: PlayerState::new(1, height-UiBarComponent::HEIGHT),
        }
    }
}

pub struct GameComponent {

}

impl GameComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for GameComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        shared_state.components_to_add.push(Box::new(PlayerComponent::new()));
        shared_state.components_to_add.push(Box::new(BuildingDrawComponent::new()));
        shared_state.components_to_add.push(Box::new(UiBarComponent::new()));
        shared_state.extensions.insert(GameState::new(setup_info.width, setup_info.height));
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let mut game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
        match game_state.phase {
            GamePhase::MoveToBuilding => {
                game_state.phase = GamePhase::Building;
                game_state.max_blocks += game_state.received_blocks;
                game_state.max_blocks_per_round = game_state.max_blocks_per_round.max(game_state.received_blocks);
                game_state.received_blocks = 0;
                game_state.blocks = game_state.max_blocks;
                shared_state.physics_board.clear();
            }
            GamePhase::Building => {
                if shared_state.pressed_keys.contains_key(&KeyCode::Char(' ')) {
                    // hack to have a frame of delay, so that we don't immediately jump due to the space bar press
                    game_state.phase = GamePhase::BuildingToMoving;
                }
            }
            GamePhase::BuildingToMoving => {
                game_state.phase = GamePhase::Moving;
                game_state.player_state.y = shared_state.display_info.height() as f64 - 1.0 - UiBarComponent::HEIGHT as f64;
                game_state.player_state.x = 1.0;
            }
            GamePhase::Moving => {

            }
        }
    }
}

#[derive(Debug)]
pub struct PlayerState {
    x: f64,
    y: f64,
    x_vel: f64,
    y_vel: f64,
    sprite: Sprite<3, 2>,
    dead_sprite: Sprite<5, 1>,
    dead_time: Option<Instant>,
    max_height_since_last_ground_touch: f64
}

impl PlayerState {
    fn new(x: usize, y: usize) -> Self {
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

pub struct PlayerComponent {
    // player_state: PlayerState,
}

impl PlayerComponent {
    const DEATH_HEIGHT: f64 = 3.5;
    const DEATH_RESPAWN_TIME: f64 = 2.0;
    const DEATH_STOP_X_MOVE_TIME: f64 = 0.5;

    pub fn new() -> Self {
        Self {
        }
    }
}

impl Component for PlayerComponent {
    fn is_active(&self, shared_state: &SharedState) -> bool {
        shared_state.extensions.get::<GameState>().unwrap().phase == GamePhase::Moving
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let mut game_state = shared_state.extensions.get_mut::<GameState>().unwrap();

        let current_time = update_info.current_time;

        if shared_state.pressed_keys.contains_key(&KeyCode::Char('k')) {
            game_state.player_state.dead_time = Some(current_time);
        }

        if let Some(dead_time) = game_state.player_state.dead_time {
            let time_since_death = (current_time - dead_time).as_secs_f64();
            if time_since_death >= Self::DEATH_STOP_X_MOVE_TIME {
                game_state.player_state.x_vel = 0.0;
            }
            if time_since_death >= Self::DEATH_RESPAWN_TIME {
                game_state.phase = GamePhase::MoveToBuilding;
                game_state.player_state.dead_time = None;
            }
        }

        let dt = update_info
            .current_time
            .saturating_duration_since(update_info.last_time)
            .as_secs_f64();

        // Player inputs, only if not dead
        if game_state.player_state.dead_time.is_none() {
            if shared_state.pressed_keys.contains_key(&KeyCode::Char('a')) {
                if game_state.player_state.x_vel > 0.0 {
                    game_state.player_state.x_vel = 0.0;
                } else if game_state.player_state.x_vel == 0.0 {
                    game_state.player_state.x_vel = -10.0;
                } else {
                    game_state.player_state.x_vel = -10.0;
                }
            } else if shared_state.pressed_keys.contains_key(&KeyCode::Char('d')) {
                if game_state.player_state.x_vel < 0.0 {
                    game_state.player_state.x_vel = 0.0;
                } else if game_state.player_state.x_vel == 0.0 {
                    game_state.player_state.x_vel = 10.0;
                } else {
                    game_state.player_state.x_vel = 10.0;
                }
            }
            if shared_state.pressed_keys.contains_key(&KeyCode::Char('A')) {
                if game_state.player_state.x_vel > 0.0 {
                    game_state.player_state.x_vel = 0.0;
                } else if game_state.player_state.x_vel == 0.0 {
                    game_state.player_state.x_vel = -20.0;
                } else {
                    game_state.player_state.x_vel = -20.0
                }
            } else if shared_state.pressed_keys.contains_key(&KeyCode::Char('D')) {
                if game_state.player_state.x_vel < 0.0 {
                    game_state.player_state.x_vel = 0.0;
                } else if game_state.player_state.x_vel == 0.0 {
                    game_state.player_state.x_vel = 20.0;
                } else {
                    game_state.player_state.x_vel = 20.0;
                }
            }
        }

        // Player physics
        let height = shared_state.display_info.height() as f64 - UiBarComponent::HEIGHT as f64;
        let width = shared_state.display_info.width() as f64;

        let gravity = 40.0;

        game_state.player_state.y_vel += gravity * dt;
        game_state.player_state.x += game_state.player_state.x_vel * dt;

        let mut bottom_wall = height;
        let mut left_wall = 0.0f64;
        let mut right_wall = width;
        let mut left_idx = None;
        let mut right_idx = None;
        let step_size = 1;

        // find a physics entity below us
        let mut x_u = game_state.player_state.x.floor() as usize;
        let mut y_u = game_state.player_state.y.floor() as usize;
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
        if game_state.player_state.x-1.0 < left_wall {
            // Check if we can do a step
            // initialize false because if there is no left_idx, we can't do a step
            let mut do_step = false;
            if let Some(left_idx) = left_idx {
                for base_check in 0..step_size {
                    // if there is one, we assume true
                    do_step = true;
                    let check_y = game_state.player_state.y.floor() as usize - 1 - base_check;
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
                game_state.player_state.x = left_wall+1.0;
            }
            // game_state.x_vel = 0.0;
        } else if game_state.player_state.x+1.0 >= right_wall {
            // Check if we can do a step
            let mut do_step = false;
            if let Some(right_idx) = right_idx {
                for base_check in 0..step_size {
                    do_step = true;
                    let check_y = game_state.player_state.y.floor() as usize - 1 - base_check;
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
                game_state.player_state.x = right_wall - 2.0;
            }

            // self.x_vel = 0.0;
        }

        // need to update for bottom checking, since x checking can clamp x and change the bottom check result
        let mut x_u = game_state.player_state.x.floor() as usize;
        // and only update y here, because otherwise x checking will think we're inside the floor block
        game_state.player_state.y += game_state.player_state.y_vel * dt;
        let mut y_u = game_state.player_state.y.floor() as usize;
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
        if game_state.player_state.y < 0.0 {
            game_state.player_state.y = 0.0;
            game_state.player_state.y_vel = 0.0;
        } else if game_state.player_state.y >= bottom_wall {
            game_state.player_state.y = bottom_wall - 1.0;
            // if we're going up, don't douch the jump velocity.
            if game_state.player_state.y_vel >= 0.0 {
                game_state.player_state.y_vel = 0.0;
            }

        }

        let grounded = game_state.player_state.y >= bottom_wall - 1.2;
        if !grounded {
            game_state.player_state.max_height_since_last_ground_touch = game_state.player_state.max_height_since_last_ground_touch.min(game_state.player_state.y);
            game_state.player_state.max_height_since_last_ground_touch = game_state.player_state.max_height_since_last_ground_touch.floor();
        } else {
            let fall_distance = game_state.player_state.y.floor() - game_state.player_state.max_height_since_last_ground_touch;
            if fall_distance >= Self::DEATH_HEIGHT {
                // Player died
                game_state.player_state.dead_time = Some(current_time);
                // add blocks proportional to fall distance
                let blocks = (fall_distance).abs().ceil() as usize;
                shared_state.debug_messages.push(DebugMessage::new(format!("You fell from {} blocks high and earned {} blocks", fall_distance, blocks), current_time + Duration::from_secs(5)));
                game_state.received_blocks += blocks;
            }
            game_state.player_state.max_height_since_last_ground_touch = game_state.player_state.y;
        }

        if game_state.player_state.dead_time.is_none() {
            // Now jump input since we need grounded information
            if shared_state.pressed_keys.contains_key(&KeyCode::Char(' ')) {
                if grounded {
                    game_state.player_state.y_vel = -20.0;
                }
            }
        }
        shared_state.debug_info.player_y = game_state.player_state.y;
        shared_state.debug_info.player_x = game_state.player_state.x;
        shared_state.debug_info.left_wall = left_wall;
        shared_state.debug_info.bottom_wall = bottom_wall;
        shared_state.debug_info.y_vel = game_state.player_state.y_vel;
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        // Set bg color depending on positive y velocity
        let max_bg_color = 100;
        let bg_color = if game_state.player_state.y_vel > 0.0 {
            [game_state.player_state.y_vel.min(max_bg_color as f64) as u8, 0, 0]
        } else {
            [0, 0, 0]
        };
        renderer.set_default_bg_color(bg_color);

        if game_state.player_state.dead_time.is_some() {
            game_state.player_state.dead_sprite
                .render(&mut renderer, game_state.player_state.x.floor() as usize, game_state.player_state.y.floor() as usize, depth_base);
        } else {
            game_state.player_state.sprite
                .render(&mut renderer, game_state.player_state.x.floor() as usize, game_state.player_state.y.floor() as usize, depth_base);
        }
    }
}


struct BuildingDrawComponent {
    last_mouse_info: MouseInfo,
    // small queue for multiple events in one frame
    draw_queue: SmallVec<[(u16, u16); 20]>,
}

impl BuildingDrawComponent {
    pub fn new() -> Self {
        Self {
            last_mouse_info: MouseInfo::default(),
            draw_queue: SmallVec::new(),
        }
    }
}

impl Component for BuildingDrawComponent {
    fn is_active(&self, shared_state: &SharedState) -> bool {
        shared_state.extensions.get::<GameState>().unwrap().phase == GamePhase::Building
    }

    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        if let Event::Mouse(event) = event {
            let mut new_mouse_info = self.last_mouse_info;
            MouseTrackerComponent::fill_mouse_info(event, &mut new_mouse_info);
            MouseTrackerComponent::smooth_two_updates(
                self.last_mouse_info,
                new_mouse_info,
                |mouse_info| {
                    if mouse_info.left_mouse_down {
                        let x = mouse_info.last_mouse_pos.0 as u16;
                        let y = mouse_info.last_mouse_pos.1 as u16;
                        if y >= shared_state.display_info.height() as u16 - UiBarComponent::HEIGHT as u16 {
                            return;
                        }
                        if self.draw_queue.contains(&(x, y)) {
                            return;
                        }
                        // if decay board already has this pixel, we don't need to count it towards our blocks
                        let exists_already = shared_state.decay_board[(x as usize, y as usize)].c != ' ';
                        // draw only if it either exists, or we have enough blocks
                        if exists_already || shared_state.extensions.get::<GameState>().unwrap().blocks > 0 {
                            if !exists_already {
                                shared_state.extensions.get_mut::<GameState>().unwrap().blocks -= 1;
                            }
                            self.draw_queue.push((x, y));
                        }
                    }
                },
            );
            self.last_mouse_info = new_mouse_info;
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        for (x, y) in self.draw_queue.drain(..) {
            shared_state.decay_board[(x as usize, y as usize)] =
                DecayElement::new_with_time('█', update_info.current_time);
        }
        // also current pixel, in case we're holding the button and not moving
        if self.last_mouse_info.left_mouse_down {
            let (x, y) = self.last_mouse_info.last_mouse_pos;
            if y < (shared_state.display_info.height() - UiBarComponent::HEIGHT) && shared_state.decay_board[(x, y)].c != ' ' {
                // refresh the decay time
                shared_state.decay_board[(x, y)] =
                    DecayElement::new_with_time('█', update_info.current_time);
            }
        }
    }
}

pub struct UiBarComponent {

}

impl UiBarComponent {
    pub const HEIGHT: usize = 7;
    const BUILDING_PHASE_COLOR: [u8; 3] = [0, 200, 0];
    const MOVING_PHASE_COLOR: [u8; 3] = [200, 0, 0];

    pub fn new() -> Self {
        Self {}
    }
}

impl Component for UiBarComponent {
    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        let blocks = game_state.blocks;
        let max_blocks = game_state.max_blocks;
        let received_blocks = game_state.received_blocks;
        let max_received_blocks = game_state.max_blocks_per_round;
        let phase = &game_state.phase;
        let (phase_str, phase_color) = match phase {
            GamePhase::MoveToBuilding => ("Building", Self::BUILDING_PHASE_COLOR),
            GamePhase::Building => ("Building", Self::BUILDING_PHASE_COLOR),
            GamePhase::BuildingToMoving => ("Moving", Self::MOVING_PHASE_COLOR),
            GamePhase::Moving => ("Moving", Self::MOVING_PHASE_COLOR),
        };


        // Draw outline of UI
        let top_y = shared_state.display_info.height() - Self::HEIGHT;
        let width = shared_state.display_info.width();
        // draw top corners
        renderer.render_pixel(0, top_y, Pixel::new('┌'), depth_base);
        renderer.render_pixel(width - 1, top_y, Pixel::new('┐'), depth_base);
        // draw top line
        "─".repeat(width - 2).chars().enumerate().for_each(|(i, c)| {
            renderer.render_pixel(i + 1, top_y, Pixel::new(c), depth_base);
        });
        let bottom_y = top_y + Self::HEIGHT - 1;
        renderer.render_pixel(0, bottom_y, Pixel::new('└'), depth_base);
        renderer.render_pixel(width - 1, bottom_y, Pixel::new('┘'), depth_base);
        // draw bottom line
        "─".repeat(width - 2).chars().enumerate().for_each(|(i, c)| {
            renderer.render_pixel(i + 1, bottom_y, Pixel::new(c), depth_base);
        });
        // Draw connecting lines
        for y in (top_y + 1)..bottom_y {
            renderer.render_pixel(0, y, Pixel::new('│'), depth_base);
            renderer.render_pixel(width - 1, y, Pixel::new('│'), depth_base);
        }

        let mut x = 1;
        let mut y = top_y + 1;
        let mut s = "Phase: ";
        s.render(&mut renderer, x, y, depth_base);
        x += s.len();
        s = phase_str;
        WithColor(phase_color, s).render(&mut renderer, x, y, depth_base);
        x = 1;
        y += 1;
        // render block numbers constant sized
        let max_blocks_str = format!("{}", max_blocks);
        let width = max_blocks_str.len();
        let block_s = if received_blocks > 0 {
            format!("Blocks: {:width$}/{} + {received_blocks}", blocks, max_blocks)
        } else {
            format!("Blocks: {:width$}/{}", blocks, max_blocks)
        };
        block_s.render(&mut renderer, x, y, depth_base);
        y += 1;
        x = 1;
        let received_blocks_str = format!("High Score: {}", max_received_blocks);
        received_blocks_str.render(&mut renderer, x, y, depth_base);
        y += 1;
        x = 1;
        let controls_str = match phase {
            GamePhase::Building | GamePhase::MoveToBuilding => "Controls: LMB to place blocks, Space to start round\n\
            Goal: Build a map for the character to die from falling from increasing heights",
            GamePhase::Moving | GamePhase::BuildingToMoving  => "Controls: A/D to move, Space to jump\n\
            Goal: Die from falling from increasing heights to earn more blocks",
        };
        controls_str.render(&mut renderer, x, y, depth_base);

    }
}