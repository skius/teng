mod animationcontroller;
mod impulse;
mod setandforgetanimations;
mod sprite;
mod goblin;
mod player;

use crate::animationcontroller::{AnimationController, KeyedAnimationResult};
use crate::impulse::Trigger;
use crate::setandforgetanimations::SetAndForgetAnimations;
use crate::sprite::{
    Animation, AnimationKind, AnimationRepository, AnimationRepositoryKey, CombinedAnimations,
    get_animation, init_animation_repository,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::{io, thread};
use teng::components::Component;
use teng::components::debuginfo::DebugMessage;
use teng::rendering::color::Color;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::{HalfBlockDisplayRender, Render};
use teng::rendering::renderer::Renderer;
use teng::util::fixedupdate::FixedUpdateRunner;
use teng::{
    Game, SetupInfo, SharedState, UpdateInfo, install_panic_handler, terminal_cleanup,
    terminal_setup,
};
use crate::goblin::Goblin;
use crate::player::{Player, PlayerComponent};

#[derive(Debug)]
struct GameState {
    goblins: Vec<Goblin>,
    player: Player,
    set_and_forget_animations: SetAndForgetAnimations,
    hbd: HalfBlockDisplayRender,
}

impl Default for GameState {
    fn default() -> Self {
        Self {
            goblins: Vec::new(),
            player: Player::default(),
            set_and_forget_animations: SetAndForgetAnimations::default(),
            hbd: HalfBlockDisplayRender::new(0, 0),
        }
    }
}

struct GameComponent {
}

impl GameComponent {
    fn new() -> Self {
        Self {
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
        shared_state.custom.hbd.resize_discard(width, 2 * height); // * 2 because world is 2x taller than screen
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        shared_state.custom.hbd.clear();
    }

    fn render(
        &self,
        renderer: &mut dyn Renderer,
        shared_state: &SharedState<GameState>,
        depth_base: i32,
    ) {
        // #6ac84f
        // renderer.set_default_bg_color([0x6a, 0xc8, 0x4f]);
        // #65bd4f
        renderer.set_default_bg_color([0x65, 0xbd, 0x4f]);

        // self.hbd.render(renderer, 0, 0, depth_base);
    }
}

// Just renders the hbd from GameState as the last component
struct RendererComponent;

impl Component<GameState> for RendererComponent {
    fn render(
        &self,
        renderer: &mut dyn Renderer,
        shared_state: &SharedState<GameState>,
        depth_base: i32,
    ) {
        shared_state.custom.hbd.render(renderer, 0, 0, depth_base);
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

    init_animation_repository();

    let mut game = Game::new_with_custom_buf_writer();
    game.install_recommended_components();
    game.add_component(Box::new(GameComponent::new()));
    game.add_component(Box::new(PlayerComponent));
    game.add_component(Box::new(RendererComponent));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
