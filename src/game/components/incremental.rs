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
//!
//! Ideas:
//! - Pet that also gives you more blocks when you die
//! - Option to draw something (RMB mechanics of not becoming physics objects until release) and if
//! if it's a valid shape, you get some special effect
//! - Option to buy permanent blocks that don't get reset when you die
//! - Every round there could be random events, for example special entities that fall from the top
//! on top of your world
//! - Upgrade that lets you redraw the world as it was last time
//! - Upgrade that lets you place blocks in the moving phase
//! - Upgrade that makes the game autoplay and the player jump immediately
//! - Upgrade that allows the player to record themselves, which can then be autoplayed.
//!    (-- how? do we just treat the player as a ghost? or replay inputs somehow? inputs seem hard.
//!    (-- ghost would only work if the world hasn't changed since last time, which would need a specific upgrade.
//! - Obvious upgrades like blocks per block fallen you receive, ghost block multipliers, blocks per alive ghost, etc.
//! - Maybe a way to make ghosts die before you?

use crate::game::components::{Bullet, DecayElement, MouseTrackerComponent};
use crate::game::{
    BreakingAction, Component, DebugMessage, MouseInfo, Pixel, Render, Renderer, SetupInfo,
    SharedState, Sprite, UpdateInfo, WithBgColor, WithColor,
};
use anymap::any::Any;
use crossterm::event::{Event, KeyCode};
use smallvec::SmallVec;
use std::time::{Duration, Instant};

#[derive(Default, Debug, PartialEq, Clone, Copy)]
enum GamePhase {
    #[default]
    MoveToBuilding,
    Building,
    BuildingToMoving,
    Moving,
}

#[derive(Debug)]
struct PlayerHistoryElement {
    x: usize,
    y: usize,
    dead: bool,
}

#[derive(Debug)]
struct Upgrades {
    auto_play: Option<bool>,
    block_height: usize,
    player_weight: usize,
    player_jump_boost_factor: f64,
    ghost_cuteness: usize,
}

impl Upgrades {
    fn new() -> Self {
        Self {
            auto_play: None,
            block_height: 1,
            player_weight: 1,
            player_jump_boost_factor: 1.0,
            ghost_cuteness: 1,
        }
    }
}

#[derive(Debug)]
struct GameState {
    phase: GamePhase,
    blocks: usize,
    max_blocks: usize,
    received_blocks: usize,
    // The amount of blocks the main player received, ignoring ghosts.
    received_blocks_base: usize,
    max_blocks_per_round: usize,
    last_received_blocks: usize,
    last_round_time: f64,
    player_state: PlayerState,
    player_history: Vec<PlayerHistoryElement>,
    player_ghosts: Vec<PlayerGhost>,
    curr_ghost_delay: f64,
    upgrades: Upgrades,
    start_of_round: Instant,
}

impl GameState {
    fn new(width: usize, height: usize) -> Self {
        Self {
            phase: GamePhase::default(),
            blocks: 0,
            max_blocks: 0,
            received_blocks: 0,
            received_blocks_base: 0,
            max_blocks_per_round: 0,
            last_received_blocks: 0,
            player_state: PlayerState::new(1, height - UiBarComponent::HEIGHT),
            player_history: Vec::new(),
            player_ghosts: vec![],
            curr_ghost_delay: 1.0,
            upgrades: Upgrades::new(),
            start_of_round: Instant::now(),
            last_round_time: 0.0,
        }
    }
}

pub struct GameComponent {}

impl GameComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for GameComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        shared_state
            .components_to_add
            .push(Box::new(PlayerComponent::new()));
        shared_state
            .components_to_add
            .push(Box::new(BuildingDrawComponent::new()));
        shared_state
            .components_to_add
            .push(Box::new(UiBarComponent::new()));
        shared_state
            .extensions
            .insert(GameState::new(setup_info.width, setup_info.height));
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let mut game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
        match game_state.phase {
            GamePhase::MoveToBuilding => {
                game_state.phase = GamePhase::Building;
                game_state.max_blocks += game_state.received_blocks;
                game_state.last_received_blocks = game_state.received_blocks;
                game_state.last_round_time = (update_info.current_time - game_state.start_of_round).as_secs_f64();
                game_state.max_blocks_per_round = game_state
                    .max_blocks_per_round
                    .max(game_state.received_blocks);
                game_state.received_blocks = 0;
                game_state.received_blocks_base = 0;
                game_state.blocks = game_state.max_blocks;
                shared_state.physics_board.clear();
            }
            GamePhase::Building => {
                if shared_state.pressed_keys.contains_key(&KeyCode::Char(' '))
                    || game_state.upgrades.auto_play == Some(true)
                {
                    // hack to have a frame of delay, so that we don't immediately jump due to the space bar press
                    game_state.phase = GamePhase::BuildingToMoving;
                }
            }
            GamePhase::BuildingToMoving => {
                game_state.phase = GamePhase::Moving;
                game_state.start_of_round = update_info.current_time;
                game_state.player_state.y =
                    shared_state.display_info.height() as f64 - 1.0 - UiBarComponent::HEIGHT as f64;
                game_state.player_state.x = 1.0;
                game_state.player_history.clear();
                for ghost in &mut game_state.player_ghosts {
                    ghost.death_time = None;
                    ghost.was_dead = false;
                }
                if let Some(true) = game_state.upgrades.auto_play {
                    shared_state.pressed_keys.insert(KeyCode::Char(' '), 1);
                    shared_state.pressed_keys.insert(KeyCode::Char('d'), 1);
                }
            }
            GamePhase::Moving => {}
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
    max_height_since_last_ground_touch: f64,
    next_sample_time: Instant,
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
            next_sample_time: Instant::now(),
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
        Self {}
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
                // for all ghosts that did not die, add 1 block
                for ghost in &game_state.player_ghosts {
                    if ghost.death_time.is_none() {
                        game_state.received_blocks += game_state.upgrades.ghost_cuteness;
                    }
                }
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
            for y in (y_u - 1)..=y_u {
                for x in ((x - 4)..=x).rev() {
                    if x < 0 || x >= width as i32 {
                        break;
                    }
                    if shared_state.collision_board[(x as usize, y)] {
                        if left_wall < x as f64 + 1.0 {
                            left_idx = Some(x as usize);
                            left_wall = x as f64 + 1.0; // plus 1.0 because we define collision on <x differently?
                        }
                        break;
                    }
                }
            }
        }
        {
            // Check right
            let x = x_u as i32 + 1;
            for y in (y_u - 1)..=y_u {
                for x in x..=(x + 4) {
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
        if game_state.player_state.x - 1.0 < left_wall {
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
                game_state.player_state.x = left_wall + 1.0;
            }
            // game_state.x_vel = 0.0;
        } else if game_state.player_state.x + 1.0 >= right_wall {
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
            for x in (x - 1)..=(x + 1) {
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
            game_state.player_state.max_height_since_last_ground_touch = game_state
                .player_state
                .max_height_since_last_ground_touch
                .min(game_state.player_state.y);
            game_state.player_state.max_height_since_last_ground_touch = game_state
                .player_state
                .max_height_since_last_ground_touch
                .floor();
        } else {
            let fall_distance = game_state.player_state.y.floor()
                - game_state.player_state.max_height_since_last_ground_touch;
            if fall_distance >= Self::DEATH_HEIGHT {
                // Player died
                game_state.player_state.dead_time = Some(current_time);
                // add blocks proportional to fall distance
                let blocks = fall_distance.abs().ceil() as usize
                    * game_state.upgrades.block_height
                    * game_state.upgrades.player_weight;
                shared_state.debug_messages.push(DebugMessage::new(
                    format!(
                        "You fell from {} blocks high and earned {} blocks",
                        fall_distance, blocks
                    ),
                    current_time + Duration::from_secs(5),
                ));
                game_state.received_blocks += blocks;
                game_state.received_blocks_base += blocks;
            }
            game_state.player_state.max_height_since_last_ground_touch = game_state.player_state.y;
        }

        if game_state.player_state.dead_time.is_none() {
            // Now jump input since we need grounded information
            if shared_state.pressed_keys.contains_key(&KeyCode::Char(' ')) {
                if grounded {
                    // set y pos to be exactly the bottom wall so we have consistent jump heights hopefully
                    game_state.player_state.y = bottom_wall - 1.0;
                    game_state.player_state.y_vel =
                        -20.0 * game_state.upgrades.player_jump_boost_factor;
                }
            }
        }
        shared_state.debug_info.player_y = game_state.player_state.y;
        shared_state.debug_info.player_x = game_state.player_state.x;
        shared_state.debug_info.left_wall = left_wall;
        shared_state.debug_info.bottom_wall = bottom_wall;
        shared_state.debug_info.y_vel = game_state.player_state.y_vel;

        // Sample player
        if current_time >= game_state.player_state.next_sample_time {
            let player_history_element = PlayerHistoryElement {
                x: game_state.player_state.x.floor() as usize,
                y: game_state.player_state.y.floor() as usize,
                dead: game_state.player_state.dead_time.is_some(),
            };
            game_state.player_history.push(player_history_element);
            game_state.player_state.next_sample_time = game_state.player_state.next_sample_time
                + Duration::from_secs_f64(1.0 / PlayerGhost::SAMPLE_RATE);
            if game_state.player_history.len() as f64 / PlayerGhost::SAMPLE_RATE
                > PlayerGhost::HISTORY_SIZE_SECS
            {
                game_state.player_history.remove(0);
            }
            // NOTE: expiry check is not really necessary, as right now ghosts cannot expire:
            // their death time will come after the player's, at which point we're not in the moving phase
            // anymore so this code is not run.
            // A game mechanic could be making the ghosts faster or maybe change the death respawn time
            // so that all ghosts have time to die. because if the player dies before all ghosts are dead,
            // the ghosts won't give the player any points.
            // another way to solve this is  to reduce the delay between individual ghosts.
            // basically, we just want to reduce the offset from the player to the slowest ghost.
            for ghost in &mut game_state.player_ghosts {
                let (just_died, _expired) = ghost.update(&game_state.player_history);
                if just_died {
                    game_state.received_blocks += game_state.received_blocks_base;
                }
            }
        }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let ghost_depth = depth_base;
        let player_base_depth = ghost_depth + 1;

        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        // Set bg color depending on positive y velocity
        let max_bg_color = 100;
        let bg_color = match (
            game_state.player_state.dead_time,
            game_state.player_state.y_vel,
        ) {
            (Some(dead_time), _) => {
                let time_since_death = (Instant::now() - dead_time).as_secs_f64();
                if time_since_death < 0.05 && game_state.upgrades.auto_play != Some(true) {
                    [200, 150, 150]
                } else {
                    [0, 0, 0]
                }
            }
            (None, y_vel) => [y_vel.min(max_bg_color as f64) as u8, 0, 0],
        };

        renderer.set_default_bg_color(bg_color);

        if game_state.player_state.dead_time.is_some() {
            game_state.player_state.dead_sprite.render(
                &mut renderer,
                game_state.player_state.x.floor() as usize,
                game_state.player_state.y.floor() as usize,
                player_base_depth,
            );
        } else {
            game_state.player_state.sprite.render(
                &mut renderer,
                game_state.player_state.x.floor() as usize,
                game_state.player_state.y.floor() as usize,
                player_base_depth,
            );
        }

        for player_ghost in &game_state.player_ghosts {
            player_ghost.render(
                &mut renderer,
                shared_state,
                ghost_depth,
                &game_state.player_state.sprite,
                &game_state.player_state.dead_sprite,
            );
        }
    }
}

#[derive(Debug)]
struct PlayerGhost {
    offset_secs: f64,
    was_dead: bool,
    death_time: Option<Instant>,
}

impl PlayerGhost {
    const SAMPLE_RATE: f64 = 160.0;
    const HISTORY_SIZE_SECS: f64 = 10.0;

    fn new(offset_secs: f64) -> Self {
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
            time_since_death >= PlayerComponent::DEATH_RESPAWN_TIME
        } else {
            false
        };

        (just_died, expired)
    }

    fn render(
        &self,
        mut renderer: &mut dyn Renderer,
        shared_state: &SharedState,
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
        if render_state.dead {
            WithColor([130, 130, 130], death_sprite).render(
                &mut renderer,
                render_state.x,
                render_state.y,
                depth_base,
            );
        } else {
            WithColor([130 + cuteness as u8, 130, 130], player_sprite).render(
                &mut renderer,
                render_state.x,
                render_state.y,
                depth_base,
            );
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
                        if y >= shared_state.display_info.height() as u16
                            - UiBarComponent::HEIGHT as u16
                        {
                            return;
                        }
                        if self.draw_queue.contains(&(x, y)) {
                            return;
                        }
                        // if decay board already has this pixel, we don't need to count it towards our blocks
                        let exists_already =
                            shared_state.decay_board[(x as usize, y as usize)].c != ' ';
                        // draw only if it either exists, or we have enough blocks
                        if exists_already
                            || shared_state.extensions.get::<GameState>().unwrap().blocks > 0
                        {
                            if !exists_already {
                                shared_state
                                    .extensions
                                    .get_mut::<GameState>()
                                    .unwrap()
                                    .blocks -= 1;
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
        // our mouse info gets outdated because we're not active all the time.
        // so we copy it from the shared state where the responsible component is hopefully active all the time.
        // another fix would be to never deactivate this component, and just have it check in update()
        // if it should draw or not.
        self.last_mouse_info = shared_state.mouse_info;
        for (x, y) in self.draw_queue.drain(..) {
            shared_state.decay_board[(x as usize, y as usize)] =
                DecayElement::new_with_time('█', update_info.current_time);
        }
        // also current pixel, in case we're holding the button and not moving
        if self.last_mouse_info.left_mouse_down {
            let (x, y) = self.last_mouse_info.last_mouse_pos;
            if y < (shared_state.display_info.height() - UiBarComponent::HEIGHT)
                && shared_state.decay_board[(x, y)].c != ' '
            {
                // refresh the decay time
                shared_state.decay_board[(x, y)] =
                    DecayElement::new_with_time('█', update_info.current_time);
            }
        }
    }
}

// TODO: resize ui button should move the x of ui buttons. Maybe just handle that in render?
// could have an enum for offset types, like Bottom(usize), Top(usize), Left(usize), Right(usize)
trait UiButton: Any {
    fn help_text(&self) -> &'static str;

    fn bbox(&self) -> (usize, usize, usize, usize) {
        panic!("Need to implement mouse_hover is bbox is not provided")
    }

    fn mouse_hover(&self, mouse_x: usize, mouse_y: usize) -> bool {
        let (x, y, width, height) = self.bbox();
        mouse_x >= x && mouse_x < x + width && mouse_y >= y && mouse_y < y + height
    }

    fn on_click(&mut self, shared_state: &mut SharedState);

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32);
}

macro_rules! new_button {
   (
        $name:ident,
        cost_growth: $cost_growth:literal,
        help_text: $help_text:literal,
        allow_in_moving: $allow_in_moving:literal,
        on_click: |$self:ident, $game_state:ident| $on_click:block,
        render: |$self2:ident, $game_state2:ident| $render:block,
        $( $field:ident: $field_type:ty = $field_default:expr ),*
    ) => {
        new_button!(
            $name,
            cost_growth: $cost_growth,
            cost_start: 1,
            help_text: $help_text,
            allow_in_moving: $allow_in_moving,
            on_click: |$self, $game_state| $on_click,
            render: |$self2, $game_state2| $render,
            $( $field: $field_type = $field_default ),*
        );
    };
    (
        $name:ident,
        cost_growth: $cost_growth:literal,
        cost_start: $cost_start:literal,
        help_text: $help_text:literal,
        allow_in_moving: $allow_in_moving:literal,
        on_click: |$self:ident, $game_state:ident| $on_click:block,
        render: |$self2:ident, $game_state2:ident| $render:block,
        $( $field:ident: $field_type:ty = $field_default:expr ),*
    ) => {
        struct $name {
            x: usize,
            y: usize,
            width: usize,
            height: usize,
            button_text: String,
            cost: usize,
            $( $field: $field_type ),*
        }

        impl $name {
            fn new(x: usize, y: usize) -> Self {
                Self {
                    x,
                    y,
                    width: 3,
                    height: 1,
                    button_text: "Buy".to_string(),
                    cost: $cost_start,
                    $( $field: $field_default ),*
                }
            }

            #[allow(unused)]
            fn change_button_text(&mut self, new_text: &str) {
                // adjust width
                let change_x = new_text.len() as i32 - self.button_text.len() as i32;
                self.x = (self.x as i32 - change_x) as usize;
                self.width = new_text.len();
                self.button_text = new_text.to_string();

            }
        }

        impl UiButton for $name {
            fn help_text(&self) -> &'static str {
                $help_text
            }

            fn bbox(&self) -> (usize, usize, usize, usize) {
                (self.x, self.y, self.width, self.height)
            }

            fn on_click(&mut $self, shared_state: &mut SharedState) {
                let $game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
                if $game_state.phase != GamePhase::Building && !$allow_in_moving {
                    return;
                }
                if $game_state.max_blocks >= $self.cost {
                    // TODO: add shared shopmanager
                    $game_state.max_blocks -= $self.cost;
                    $game_state.blocks -= $self.cost;
                    $on_click
                    $self.cost = (($self.cost as f64) * $cost_growth).ceil() as usize;
                }
            }

            fn render(&$self2, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
                let $game_state2 = shared_state.extensions.get::<GameState>().unwrap();
                let is_hover = $self2.mouse_hover(shared_state.mouse_info.last_mouse_pos.0, shared_state.mouse_info.last_mouse_pos.1);
                let lmb_down = shared_state.mouse_info.left_mouse_down;

                let fg_color = [0, 0, 0];
                let enough_blocks = $game_state2.max_blocks >= $self2.cost;
                let deactivated_color = [100, 100, 100];
                let mut bg_color = if $allow_in_moving {
                    match (is_hover, lmb_down) {
                        (true, true) => [200, 200, 255],
                        (true, false) => [255, 255, 255],
                        (false, _) => [200, 200, 200],
                    }
                } else {
                    match (is_hover, lmb_down, $game_state2.phase) {
                        (_, _, phase) if phase != GamePhase::Building => deactivated_color,
                        (true, true, _) => [200, 200, 255],
                        (true, false, _) => [255, 255, 255],
                        (false, _, _) => [200, 200, 200],
                    }
                };
                if !enough_blocks {
                    bg_color = deactivated_color;
                }
                WithColor(fg_color, WithBgColor(bg_color, &$self2.button_text)).render(
                    &mut renderer,
                    $self2.x,
                    $self2.y,
                    depth_base,
                );
                let left_text = $render;
                // render to the left
                let len = left_text.len();
                left_text.render(&mut renderer, $self2.x - len as usize, $self2.y, depth_base);
            }
        }
    };
}

new_button!(
    BlockHeightButton,
    cost_growth: 2.0,
    cost_start: 2000,
    help_text: "Help: Increase the height of blocks by 1.",
    allow_in_moving: false,
    on_click: |self, game_state| {
        game_state.upgrades.block_height += 1;
    },
    render: |self, game_state| {
        format!(
            "Block Height ({}) for {} ",
            game_state.upgrades.block_height, self.cost
        )
    },
);

new_button!(
    PlayerWeightButton,
    cost_growth: 3.0,
    cost_start: 20_000,
    help_text: "Help: Increase the weight of the player.",
    allow_in_moving: false,
    on_click: |self, game_state| {
        game_state.upgrades.player_weight += 1;
    },
    render: |self, game_state| {
        format!(
            "Player Weight ({}) for {} ",
            game_state.upgrades.player_weight, self.cost
        )
    },
);

new_button!(
    PlayerJumpHeightButton,
    cost_growth: 3.0,
    cost_start: 15,
    help_text: "Help: Increase the jump height of the player.",
    allow_in_moving: false,
    on_click: |self, game_state| {
        game_state.upgrades.player_jump_boost_factor += 0.1;
    },
    render: |self, game_state| {
        format!(
            "Jump Height ({:.1}) for {} ",
            game_state.upgrades.player_jump_boost_factor, self.cost
        )
    },
);

new_button!(
    GhostBuyButton,
    cost_growth: 1.4,
    cost_start: 80,
    help_text: "Help: Ghosts give the same amount of blocks on death as the player and 1 block\nif they are alive at the end of the round.",
    allow_in_moving: false,
    on_click: |self, game_state| {
        let new_offset = if let Some(player_ghost) = game_state.player_ghosts.last() {
            player_ghost.offset_secs + game_state.curr_ghost_delay
        } else {
            game_state.curr_ghost_delay
        };
        game_state.player_ghosts.push(PlayerGhost::new(new_offset));
    },
    render: |self, game_state| {
        format!(
            "Player Ghosts ({}) for {} ",
            game_state.player_ghosts.len(),
            self.cost
        )
    },
);

new_button!(
    GhostCutenessButton,
    cost_growth: 1.1,
    cost_start: 100,
    help_text: "Help: Ghosts give more blocks if they're alive at the end of a round.",
    allow_in_moving: false,
    on_click: |self, game_state| {
        game_state.upgrades.ghost_cuteness += 1;
    },
    render: |self, game_state| {
        format!(
            "Ghost Cuteness ({}) for {} ",
            game_state.upgrades.ghost_cuteness,
            self.cost
        )
    },
);

new_button!(
    GhostDelayButton,
    cost_growth: 1.8,
    cost_start: 600,
    help_text: "Help: Decrease the delay between player and ghost movement.",
    allow_in_moving: false,
    on_click: |self, game_state| {
        game_state.curr_ghost_delay /= 1.2;
        let mut curr_offset = game_state.curr_ghost_delay;
        for ghost in &mut game_state.player_ghosts {
            ghost.offset_secs = curr_offset;
            curr_offset += game_state.curr_ghost_delay;
        }
    },
    render: |self, game_state| {
        format!(
            "Ghost Delay ({:.3}) for {} ",
            game_state.curr_ghost_delay, self.cost
        )
    },
);

new_button!(
    AutoPlayButton,
    cost_growth: 1.0,
    cost_start: 1000,
    help_text: "Help: Automatically start rounds and make the player jump.",
    allow_in_moving: true,
    on_click: |self, game_state| {
        if let Some(auto_play) = game_state.upgrades.auto_play {
            game_state.upgrades.auto_play = Some(!auto_play);
        } else {
            game_state.upgrades.auto_play = Some(false);
            self.change_button_text("Toggle");
            self.cost = 0;
        }

    },
    render: |self, game_state| {
        if self.cost > 0 {
            format!(
                "Auto Play for {} ",
                self.cost
            )
        } else {
            format!(
                "Auto Play ({}) ",
                if game_state.upgrades.auto_play.unwrap() { "On" } else { "Off" }
            )
        }
    },
);

pub struct UiBarComponent {
    buttons: Vec<Box<dyn UiButton>>,
    hover_button: Option<usize>,
}

impl UiBarComponent {
    pub const HEIGHT: usize = 11;
    const BUILDING_PHASE_COLOR: [u8; 3] = [0, 200, 0];
    const MOVING_PHASE_COLOR: [u8; 3] = [200, 0, 0];

    pub fn new() -> Self {
        Self {
            buttons: vec![],
            hover_button: None,
        }
    }
}

impl Component for UiBarComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        let mut y = setup_info.height - Self::HEIGHT + 1;
        let text = "Buy".to_string();
        let x = setup_info.width - 1 - text.len();
        self.buttons
            .push(Box::new(PlayerJumpHeightButton::new(x, y)));
        y += 1;
        self.buttons.push(Box::new(GhostBuyButton::new(x, y)));
        y += 1;
        self.buttons.push(Box::new(GhostCutenessButton::new(x, y)));
        y += 1;
        self.buttons.push(Box::new(GhostDelayButton::new(x, y)));
        y += 1;
        self.buttons.push(Box::new(AutoPlayButton::new(x, y)));
        y += 1;
        self.buttons.push(Box::new(BlockHeightButton::new(x, y)));
        y += 1;
        self.buttons.push(Box::new(PlayerWeightButton::new(x, y)));
        y += 1;
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        let last_mouse_info = shared_state.mouse_info;
        // Check if we're hovering a button
        let (x, y) = last_mouse_info.last_mouse_pos;
        let mut hovering = false;
        for (i, button) in self.buttons.iter().enumerate() {
            if button.mouse_hover(x, y) {
                self.hover_button = Some(i);
                hovering = true;
                break;
            }
        }
        if !hovering {
            self.hover_button = None;
        }
        if shared_state.mouse_pressed.left {
            // we pressed a button, if we're hovering
            if let Some(hover_button) = self.hover_button {
                self.buttons[hover_button].on_click(shared_state);
            }
        }
    }

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
        "─"
            .repeat(width - 2)
            .chars()
            .enumerate()
            .for_each(|(i, c)| {
                renderer.render_pixel(i + 1, top_y, Pixel::new(c), depth_base);
            });
        let bottom_y = top_y + Self::HEIGHT - 1;
        renderer.render_pixel(0, bottom_y, Pixel::new('└'), depth_base);
        renderer.render_pixel(width - 1, bottom_y, Pixel::new('┘'), depth_base);
        // draw bottom line
        "─"
            .repeat(width - 2)
            .chars()
            .enumerate()
            .for_each(|(i, c)| {
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
            format!(
                "Blocks: {:width$}/{} + {received_blocks}",
                blocks, max_blocks
            )
        } else {
            // let recv_s = if game_state.phase == GamePhase::Building {
            //     format!(" (received: {}, per second: {:.2})", game_state.last_received_blocks, bps)
            // } else {
            //     "".to_string()
            // };
            // format!("Blocks: {:width$}/{} {recv_s}", blocks, max_blocks)
            format!("Blocks: {:width$}/{}", blocks, max_blocks)
        };
        block_s.render(&mut renderer, x, y, depth_base);
        y += 1;
        x = 1;
        // TODO: factor in building time to bps?
        // TODO: keep track of max bps overall?
        let bps = game_state.last_received_blocks as f64 / game_state.last_round_time;
        format!("Last round: {} at {:.2}/s", game_state.last_received_blocks, bps).render(&mut renderer, x, y, depth_base);
        y += 1;
        x = 1;
        let received_blocks_str = format!("High Score: {}", max_received_blocks);
        received_blocks_str.render(&mut renderer, x, y, depth_base);
        y += 1;
        x = 1;
        let controls_str = match (phase, self.hover_button) {
            (_, Some(hover_button)) => self.buttons[hover_button].help_text(),
            (GamePhase::Building | GamePhase::MoveToBuilding, _) => {
                "Controls: LMB to place blocks, Space to start round\n\
            Goal: Build a map for the character to die from falling from increasing heights"
            }
            (GamePhase::Moving | GamePhase::BuildingToMoving, _) => {
                "Controls: A/D to move, Space to jump\n\
            Goal: Die from falling from increasing heights to earn more blocks"
            }
        };
        controls_str.render(&mut renderer, x, y, depth_base);

        // render buttons
        for button in &self.buttons {
            button.render(&mut renderer, shared_state, depth_base);
        }
    }
}
