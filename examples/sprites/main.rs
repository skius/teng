mod animationcontroller;
mod goblin;
mod impulse;
mod player;
mod setandforgetanimations;
mod sprite;
mod wgpurender;

use crate::animationcontroller::{AnimationController, KeyedAnimationResult};
use crate::goblin::Goblin;
use crate::impulse::Trigger;
use crate::player::{Player, PlayerComponent};
use crate::setandforgetanimations::SetAndForgetAnimations;
use crate::sprite::{
    Animation, AnimationKind, AnimationRepository, AnimationRepositoryKey, CombinedAnimations,
    get_animation, init_animation_repository,
};
use rayon::prelude::*;
use std::collections::HashMap;
use std::{io, thread};
use std::time::Instant;
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
use teng::components::keyboard::KeypressDebouncerComponent;
use crate::wgpurender::{WgpuRenderComponent, WgpuShadertoyRenderComponent, WgpuSpriteRenderComponent};

enum HurtGroup {
    Player,
    Goblin,
}

struct HurtBox {
    min_x: f64,
    max_x: f64,
    min_y: f64,
    max_y: f64,

    start: Instant,
    duration: f64,
    hurt_tick_every_seconds: f64,
    
    // only entities in this group will be hurt
    hurt_group: HurtGroup,
    
    // every tick
    damage: f64,
}


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

struct GameComponent {}

impl GameComponent {
    fn new() -> Self {
        Self {}
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
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        // screen tearing test
        // let random_grey = rand::random::<u8>();
        // shared_state.custom.hbd.set_color(0, 0, Color::Rgb([random_grey; 3]));

        // let mut some_sprite = get_animation(AnimationRepositoryKey::PlayerIdle);
        // let (x, y) = shared_state.mouse_info.last_mouse_pos;
        // let y = 2*y;
        // some_sprite.render_to_hbd(x as i64, y as i64, &mut shared_state.custom.hbd, 0.0);
        // some_sprite.render_to_hbd(x as i64, y as i64 + 16, &mut shared_state.custom.hbd, 0.0);
        // some_sprite.render_to_hbd(x as i64, y as i64 + 32, &mut shared_state.custom.hbd, 0.0);
    }

    fn render(
        &self,
        renderer: &mut dyn Renderer,
        shared_state: &SharedState<GameState>,
        depth_base: i32,
    ) {
        // screen tearing test
        // let size_x = 30;
        // let size_y = 30;
        // let (x, y) = shared_state.mouse_info.last_mouse_pos;
        // 
        // for i in 0..size_x {
        //     for j in 0..size_y {
        //         // interesting. No screen tear when I'm not holding down LMB and moving the hbd pixels,
        //         // but screen tear when I'm holding down LMB and moving the hbd pixels.
        //         // Visually there is no difference, but I believe the hbd still sets the background color,
        //         // we just don't see it because the full block is drawn over it.
        //         renderer.render_pixel(x + i, y + j, Pixel::new('█'), depth_base);
        //     }
        // }

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
    game.add_component(Box::new(KeypressDebouncerComponent::new(70)));
    game.add_component(Box::new(GameComponent::new()));
    game.add_component(Box::new(PlayerComponent));
    game.add_component(Box::new(WgpuSpriteRenderComponent::new()));
    // game.add_component(Box::new(WgpuRenderComponent::new()));
    // game.add_component(Box::new(WgpuShadertoyRenderComponent::new()));
    game.add_component(Box::new(RendererComponent));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
