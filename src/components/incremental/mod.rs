//! Game name ideas:
//! - Terminally Ill (exe: termill?)
//! - Untitled Terminal Incremental Game (UTIG, GITU(reverse))?
//!
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
//! - Needs some kind of final purchase that's really expensive. Maybe 100B or so?
//!    would be cool if that unlocked some new feature that made the numbers go up even more...
//! - Ooh! Add a "mirror" that is just an entire player character incl. all upgrades like ghosts, that just mirrors
//!    the movement on the other side of the screen. Immediately doubles the bps.
//! - There could be an upgrade that ends the round immediately when all ghosts are dead. This could be
//!     a late upgrade, for when the number of ghosts is typically the limiting factor instead of ghost delay.
//! - Procedurally generated world!!!
//! TODOs before playtests:
//! - Disable unneeded components
//! - Fix resize (player falling and button locations)
//! - Maybe make player able to go above the screen (disable y = 0 collision)
//! - At high fall gravities, the red background screen starts flashing when the player is on the floor.
//!    maybe add override for when the player is on the floor and skip it?

use crate::components::incremental::player::{NewPlayerComponent, NewPlayerState, PlayerGhost};
use crate::components::incremental::slingshot::SlingshotComponent;
use crate::components::incremental::titlescreen::TitleScreenComponent;
use crate::components::incremental::ui::UiBarComponent;
use crate::components::incremental::world::{World, WorldComponent};
use crate::components::incremental::worldmap::WorldMapComponent;
use crate::{Component, DebugMessage, SetupInfo, SharedState, UpdateInfo};
use crossterm::event::KeyCode;
use std::time::Instant;

mod animation;
mod collisionboard;
pub mod falling;
mod player;
mod slingshot;
pub mod titlescreen;
pub mod ui;
pub mod world;
pub mod worldmap;

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
    x: i64,
    y: i64,
    dead: bool,
}

#[derive(Debug)]
struct Upgrades {
    auto_play: Option<bool>,
    block_height: u128,
    player_weight: u128,
    player_jump_boost_factor: f64,
    fall_speed_factor: f64,
    ghost_cuteness: u128,
    velocity_exponent: f64,
    slingshot: bool,
}

impl Upgrades {
    fn new() -> Self {
        Self {
            auto_play: None,
            block_height: 1,
            player_weight: 1,
            player_jump_boost_factor: 1.0,
            fall_speed_factor: 1.0,
            ghost_cuteness: 1,
            velocity_exponent: 0.0,
            slingshot: false,
        }
    }
}

#[derive(Debug)]
struct GameState {
    phase: GamePhase,
    blocks: u128,
    max_blocks: u128,
    received_blocks: u128,
    // The amount of blocks the main player received, ignoring ghosts.
    received_blocks_base: u128,
    max_blocks_per_round: u128,
    last_received_blocks: u128,
    last_round_time: f64,
    new_player_state: NewPlayerState,
    player_history: Vec<PlayerHistoryElement>,
    player_ghosts: Vec<PlayerGhost>,
    curr_ghost_delay: f64,
    upgrades: Upgrades,
    start_of_round: Instant,
    start_of_game: Instant,
    world: World,
}

impl GameState {
    fn new(setup_info: &SetupInfo) -> Self {
        let height = setup_info.height;
        Self {
            phase: GamePhase::default(),
            blocks: 0,
            max_blocks: 0,
            received_blocks: 0,
            received_blocks_base: 0,
            max_blocks_per_round: 0,
            last_received_blocks: 0,
            new_player_state: NewPlayerState::new(),
            player_history: Vec::new(),
            player_ghosts: vec![],
            curr_ghost_delay: 1.0,
            upgrades: Upgrades::new(),
            start_of_round: Instant::now(),
            last_round_time: 0.0,
            start_of_game: Instant::now(),
            world: World::new(setup_info),
        }
    }
}

pub struct GameComponent {
    install_title_screen: bool,
}

impl GameComponent {
    pub fn new(install_title_screen: bool) -> Self {
        Self {
            install_title_screen,
        }
    }
}

impl<S: 'static> Component<S> for GameComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<S>) {
        shared_state
            .components_to_add
            .push(Box::new(WorldComponent::new()));
        // shared_state
        //     .components_to_add
        //     .push(Box::new(PlayerComponent::new()));
        shared_state
            .components_to_add
            .push(Box::new(NewPlayerComponent::new()));
        // shared_state
        //     .components_to_add
        //     .push(Box::new(BuildingDrawComponent::new()));
        shared_state
            .components_to_add
            .push(Box::new(SlingshotComponent::new()));
        // shared_state
        //     .components_to_add
        //     .push(Box::new(UiBarComponent::new()));
        shared_state
            .components_to_add
            .push(Box::new(WorldMapComponent::new(30, 30, 600, 600, 50)));
        if self.install_title_screen {
            shared_state
                .components_to_add
                .push(Box::new(TitleScreenComponent::new()));
        }

        shared_state.extensions.insert(GameState::new(setup_info));
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        let game_state = shared_state.extensions.get_mut::<GameState>().unwrap();

        // cheats
        if shared_state.pressed_keys.did_press_char_ignore_case('b') {
            game_state.max_blocks += 1;
            game_state.max_blocks *= 1000000;
            game_state.blocks = game_state.max_blocks;
            shared_state
                .debug_messages
                .push(DebugMessage::new_3s("Cheated blocks!"));
        }

        match game_state.phase {
            GamePhase::MoveToBuilding => {
                game_state.phase = GamePhase::Building;
                game_state.max_blocks += game_state.received_blocks;
                game_state.last_received_blocks = game_state.received_blocks;
                game_state.last_round_time =
                    (update_info.current_time - game_state.start_of_round).as_secs_f64();
                game_state.max_blocks_per_round = game_state
                    .max_blocks_per_round
                    .max(game_state.received_blocks);
                game_state.received_blocks = 0;
                game_state.received_blocks_base = 0;
                game_state.blocks = game_state.max_blocks;
                // Note: archived the physics board. See in archive
                // shared_state.physics_board.clear();
            }
            GamePhase::Building => {
                if shared_state.pressed_keys.did_press_char(' ')
                    || game_state.upgrades.auto_play == Some(true)
                {
                    // hack to have a frame of delay, so that we don't immediately jump due to the space bar press
                    game_state.phase = GamePhase::BuildingToMoving;
                }
            }
            GamePhase::BuildingToMoving => {
                game_state.phase = GamePhase::Moving;
                game_state.start_of_round = update_info.current_time;
                game_state.player_history.clear();
                for ghost in &mut game_state.player_ghosts {
                    ghost.death_time = None;
                    ghost.was_dead = false;
                }
                if let Some(true) = game_state.upgrades.auto_play {
                    shared_state.pressed_keys.insert(KeyCode::Char(' '));
                    // shared_state.pressed_keys.insert(KeyCode::Char('d'), 1);
                }
            }
            GamePhase::Moving => {}
        }
    }
}

struct BuildingDrawComponent {}

impl BuildingDrawComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl<S> Component<S> for BuildingDrawComponent {
    fn is_active(&self, shared_state: &SharedState<S>) -> bool {
        shared_state.extensions.get::<GameState>().unwrap().phase == GamePhase::Building
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        let game_state = shared_state.extensions.get_mut::<GameState>().unwrap();

        // We use _sticky, because we want to keep drawing a pixel if we keep the mouse pressed.
        shared_state.mouse_events.for_each_linerp_sticky(|mi| {
            if mi.left_mouse_down {
                let x = mi.last_mouse_pos.0;
                let y = mi.last_mouse_pos.1;

                if let Some((w_x, w_y)) = game_state.world.to_world_pos(x, y) {
                    if game_state.world[(w_x, w_y)].is_solid() {
                        return;
                    }
                } else {
                    return;
                }

                if y >= shared_state.display_info.height() - UiBarComponent::HEIGHT {
                    return;
                }

                // Note: Removed the decay component. Check in archive if we want to readd it.

                // if decay board already has this pixel, we don't need to count it towards our blocks
                // let exists_already = shared_state.decay_board[(x as usize, y as usize)].c != ' ';
                // draw only if it either exists, or we have enough blocks
                // if exists_already || game_state.blocks > 0 {
                //     if !exists_already {
                //         game_state.blocks -= 1;
                //     }
                // Note: Removed the decay component. Check in archive if we want to readd it.
                // shared_state.decay_board[(x, y)] =
                //     DecayElement::new_with_time('█', update_info.current_time);
                // }
            }
        });
    }
}
