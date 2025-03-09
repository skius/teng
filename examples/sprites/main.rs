mod sprite;
mod impulse;
mod animationcontroller;

use std::{io, thread};
use std::collections::HashMap;
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
use teng::components::debuginfo::DebugMessage;
use crate::animationcontroller::{AnimationController, KeyedAnimationResult};
use crate::sprite::{Animation, AnimationKind, CombinedAnimations};

#[derive(Debug, Default)]
struct GameState {

}

#[derive(Hash, Eq, PartialEq, Default, Clone, Copy)]
enum PlayerState {
    #[default]
    Idle,
    Walk,
    Axe,
}
// 
// 
// struct PlayerAnimations {
//     map: HashMap<PlayerState, CombinedAnimations>,
// }
// 
// impl PlayerAnimations {
//     fn new() -> Self {
//         let mut map = HashMap::new();
//         let speed = 0.1;
// 
//         let idle_anims;
//         {
//             let animation_base = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/IDLE/base_idle_strip9.png");
//             let animation_bowlhair = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/IDLE/bowlhair_idle_strip9.png");
//             let animation_tools = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/IDLE/tools_idle_strip9.png");
//             idle_anims = CombinedAnimations::new(vec![animation_base, animation_bowlhair, animation_tools], speed);
//         }
// 
//         let walk_anims;
//         {
//             let animation_base = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/WALKING/base_walk_strip8.png");
//             let animation_bowlhair = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/WALKING/bowlhair_walk_strip8.png");
//             let animation_tools = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/WALKING/tools_walk_strip8.png");
//             walk_anims = CombinedAnimations::new(vec![animation_base, animation_bowlhair, animation_tools], speed);
//         }
// 
//         map.insert(PlayerState::Idle, idle_anims);
//         map.insert(PlayerState::Walk, walk_anims);
//         Self {
//             map,
//         }
//     }
// 
//     fn set_flipped_x(&mut self, flipped_x: bool) {
//         for (_, anim) in &mut self.map {
//             anim.set_flipped_x(flipped_x);
//         }
//     }
// }

struct GameComponent {
    hbd: HalfBlockDisplayRender,
    animation_controller: AnimationController<PlayerState>,
    is_flipped_x: bool,
    character_pos: (f64, f64),
}

impl GameComponent {
    fn new() -> Self {
        let mut animation_controller = AnimationController::default();
        let speed = 0.1;

        {
            let animation_base = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/IDLE/base_idle_strip9.png");
            let animation_bowlhair = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/IDLE/bowlhair_idle_strip9.png");
            let animation_tools = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/IDLE/tools_idle_strip9.png");
            let idle_anims = CombinedAnimations::new(vec![animation_base, animation_bowlhair, animation_tools], speed);
            animation_controller.register_animation(PlayerState::Idle, idle_anims);
        }
        {
            let animation_base = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/WALKING/base_walk_strip8.png");
            let animation_bowlhair = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/WALKING/bowlhair_walk_strip8.png");
            let animation_tools = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/WALKING/tools_walk_strip8.png");
            let walk_anims = CombinedAnimations::new(vec![animation_base, animation_bowlhair, animation_tools], speed);
            animation_controller.register_animation(PlayerState::Walk, walk_anims);
        }
        // a one shot anim
        {
            let animation_base = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/AXE/base_axe_strip10.png");
            let animation_bowlhair = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/AXE/bowlhair_axe_strip10.png");
            let animation_tools = Animation::from_strip("examples/sprites/data/Sunnyside_World_Assets/Characters/Human/AXE/tools_axe_strip10.png");
            let mut axe_anims = CombinedAnimations::new(vec![animation_base, animation_bowlhair, animation_tools], speed);
            axe_anims.set_kind(AnimationKind::OneShot { trigger_frame: Some(7) });
            animation_controller.register_animation(PlayerState::Axe, axe_anims);
        }

        Self {
            hbd: HalfBlockDisplayRender::new(0, 0),
            animation_controller,
            is_flipped_x: false,
            character_pos: (0.0, 0.0),
        }
    }

    fn speed_from_distance(&self, distance: f64) -> f64 {
        // base distance is 10.0.
        assert!(distance >= 10.0);
        let normalized = distance - 10.0 + 1.0;
        // speed grows based on distance
        let speed = normalized.clamp(30.0, 80.0);
        speed
    }
}

impl Component<GameState> for GameComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<GameState>) {
        let x = setup_info.display_info.width() as i64 / 2;
        let y = setup_info.display_info.height() as i64 / 2 * 2; // * 2 because world is 2x taller than screen
        self.character_pos = (x as f64, y as f64);
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
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        let width = self.hbd.width();
        let height = self.hbd.height();

        // TODO: we really need a mouse_released struct (similar to mouse_pressed)
        // if shared_state.mouse_info.left_mouse_down {
        //     for anim in &mut self.animations {
        //         anim.set_flipped_x(true);
        //     }
        // } else {
        //     for anim in &mut self.animations {
        //         anim.set_flipped_x(false);
        //     }
        // }

        // check if mouse pos is on left or right half of screen, and flip accordingly
        let mouse_x = shared_state.mouse_info.last_mouse_pos.0;
        if (mouse_x as f64) < self.character_pos.0 {
            if !self.is_flipped_x {
                self.animation_controller.set_flipped_x(true);
                self.is_flipped_x = true;
            }
        } else {
            if self.is_flipped_x {
                self.animation_controller.set_flipped_x(false);
                self.is_flipped_x = false;
            }
        }

        // move character slowly to mouse pos
        let (mouse_x, mouse_y) = shared_state.mouse_info.last_mouse_pos;
        let mouse_x = mouse_x as i64;
        let mouse_y = mouse_y as i64 * 2; // world is 2x taller than screen
        let (char_x, char_y) = self.character_pos;

        let dx = mouse_x as f64 - char_x;
        let dy = mouse_y as f64 - char_y;
        let dist_sqr = (dx * dx + dy * dy);
        if dist_sqr > 10.0 * 10.0 {
            // move character
            let dist = dist_sqr.sqrt();
            let speed = self.speed_from_distance(dist);
            // panic!("speed: {}", speed);
            // let speed = 20.0;
            let dt = update_info.dt;
            let normalized = (dx / dist, dy / dist);
            let (dx, dy) = normalized;
            self.character_pos.0 += dx * speed * dt;
            self.character_pos.1 += dy * speed * dt;
            self.animation_controller.set_animation(PlayerState::Walk);
        } else {
            self.animation_controller.set_animation(PlayerState::Idle);
        }

        // render
        self.hbd.clear();
        let (draw_x, draw_y) = self.character_pos;
        let draw_x = draw_x.floor() as i64;
        let draw_y = draw_y.floor() as i64;
        // for animation in &self.animations {
        //     animation.render_to_hbd(draw_x, draw_y, &mut self.hbd, update_info.current_time);
        // }
        
        let anim_res = self.animation_controller.render_to_hbd(draw_x, draw_y, &mut self.hbd, update_info.current_time);
        if let Some(anim_res) = anim_res {
            match anim_res {
                KeyedAnimationResult::Triggered(state) => {
                    if state == PlayerState::Axe {
                        // axe animation was triggered
                        shared_state.debug_messages.push(DebugMessage::new_3s("Axe animation triggered!"));
                    }
                }
                KeyedAnimationResult::Finished(state) => {
                    if state == PlayerState::Axe {
                        // axe animation was finished
                        // TODO: this does not get triggered because the above blanket setting to ::Idle overrides the axe animation, since it's 'finished' so it can be overriden despite
                        // the 'finished' not being consumed. Though I guess that's fine? as long as our trigger is consumed...
                        shared_state.debug_messages.push(DebugMessage::new_3s("Axe animation finished!"));
                        self.animation_controller.set_animation(PlayerState::Idle);
                    }
                }
            }
        }
        
        if shared_state.mouse_pressed.left {
            // trigger axe animation
            self.animation_controller.set_animation_override(PlayerState::Axe);
        }

        // let anim = self.animations.map.get(&self.player_state).unwrap();
        // anim.render_to_hbd(draw_x, draw_y, &mut self.hbd, update_info.current_time);
        // rot test
        // let first_sprite = self.animations.map.get(&self.player_state).unwrap().animations[0].frames[0].clone();
        // let mut angle = {
        //     // compute diff from mosue to char
        //     let dx = mouse_x as f64 - char_x;
        //     let dy = mouse_y as f64 - char_y;
        //     let angle = dy.atan2(dx);
        //     angle.to_degrees()
        // };
        // if self.is_flipped_x {
        //     // flip angle
        //     angle = -angle + 180.0;
        // }
        // let rotated = first_sprite.get_rotated(angle);
        // rotated.render_to_hbd(draw_x, draw_y, &mut self.hbd);


        // draw entire red-green color space
        // for x in 0..=255 {
        //     for y in 0..=255 {
        //         let color = Color::Rgb([x as u8, y as u8, 0]);
        //         self.hbd.set_color(x, y, color);
        //     }
        //     // same for red-blue
        //     for y in 0..=255 {
        //         let color = Color::Rgb([x as u8, 0, y as u8]);
        //         self.hbd.set_color(x + 256, y, color);
        //     }
        // }
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
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
