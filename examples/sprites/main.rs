mod sprite;
mod impulse;
mod animationcontroller;
mod setandforgetanimations;

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
use crate::impulse::Trigger;
use crate::setandforgetanimations::SetAndForgetAnimations;
use crate::sprite::{get_animation, init_animation_repository, Animation, AnimationKind, AnimationRepository, AnimationRepositoryKey, CombinedAnimations};

#[derive(Debug, Default)]
struct GameState {

}

#[derive(Hash, Eq, PartialEq, Default, Clone, Copy)]
enum PlayerState {
    #[default]
    Idle,
    Walk,
    Run,
    Jump,
    Roll,
    Axe,
    Sword,
}

#[derive(Default)]
struct InputCache {
    lmb: Trigger,
    rmb: Trigger,
    space: Trigger,
    w: Trigger,
    a: Trigger,
    s: Trigger,
    d: Trigger,
}

impl InputCache {
    fn update(&mut self, shared_state: &SharedState<GameState>) {
        if shared_state.mouse_pressed.left {
            self.lmb.set();
        }
        if shared_state.mouse_pressed.right {
            self.rmb.set();
        }
        if shared_state.pressed_keys.did_press_char_ignore_case(' ') {
            self.space.set();
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('w') {
            self.w.set();
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('a') {
            self.a.set();
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('s') {
            self.s.set();
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('d') {
            self.d.set();
        }

    }
}

struct GameComponent {
    hbd: HalfBlockDisplayRender,
    animation_controller: AnimationController<PlayerState>,
    set_and_forget_animations: SetAndForgetAnimations,
    is_rolling: bool,
    roll_direction: (f64, f64),
    is_flipped_x: bool,
    character_pos: (f64, f64),
    // TODO: have a way to time out the input cache, so that a key press is not consumed if it's too old
    input_cache: InputCache,
}

impl GameComponent {
    fn new() -> Self {
        let mut animation_controller = AnimationController::default();

        {
            animation_controller.register_animation(PlayerState::Idle, get_animation(AnimationRepositoryKey::PlayerIdle));
        }
        {
            animation_controller.register_animation(PlayerState::Walk, get_animation(AnimationRepositoryKey::PlayerWalk));
        }
        // a one shot anim
        {
            animation_controller.register_animation(PlayerState::Axe, get_animation(AnimationRepositoryKey::PlayerAxe));
        }
        {
            animation_controller.register_animation(PlayerState::Sword, get_animation(AnimationRepositoryKey::PlayerSword));
        }
        {
            animation_controller.register_animation(PlayerState::Jump, get_animation(AnimationRepositoryKey::PlayerJump));
        }
        {
            animation_controller.register_animation(PlayerState::Roll, get_animation(AnimationRepositoryKey::PlayerRoll));
        }
        {
            animation_controller.register_animation(PlayerState::Run, get_animation(AnimationRepositoryKey::PlayerRun));
        }

        Self {
            hbd: HalfBlockDisplayRender::new(0, 0),
            animation_controller,
            set_and_forget_animations: SetAndForgetAnimations::default(),
            is_flipped_x: false,
            is_rolling: false,
            roll_direction: (0.0, 0.0),
            character_pos: (0.0, 0.0),
            input_cache: InputCache::default(),
        }
    }

    fn allows_flipping_x(&self) -> bool {
        match self.animation_controller.current_state() {
            PlayerState::Axe => false,
            // overriden by roll itself
            PlayerState::Roll => !self.is_rolling,
            _ => true,
        }
    }

    fn allows_moving(&self) -> bool {
        match self.animation_controller.current_state() {
            PlayerState::Axe => false,
            _ => true,
        }
    }

    fn allows_new_oneshot(&self) -> bool {
        match self.animation_controller.current_state() {
            PlayerState::Axe => false,
            PlayerState::Sword => false,
            PlayerState::Jump => false,
            PlayerState::Roll => !self.is_rolling,
            _ => true,
        }
    }

    fn speed_from_distance(&self, distance: f64) -> f64 {
        let mut min = 30.0;
        let mut max = 80.0;
        if self.animation_controller.current_state() == PlayerState::Sword {
            // if we're attacking, our max is slower, but at the same time we want to move as quickly as possible to our target
            // so our min is higher
            min = 50.0;
            max = 50.0;
        }

        // base distance is 10.0.
        assert!(distance >= 10.0);
        let normalized = distance - 10.0 + 1.0;
        // speed grows based on distance
        let speed = normalized.clamp(min, max);
        speed
    }

    fn set_flipped_x(&mut self, flipped_x: bool) {
        if self.is_flipped_x == flipped_x {
            return;
        }
        self.is_flipped_x = flipped_x;
        self.animation_controller.set_flipped_x(flipped_x);
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
        // check if mouse pos is on left or right half of screen, and flip accordingly
        if self.allows_flipping_x() {
            let mouse_x = shared_state.mouse_info.last_mouse_pos.0;
            if (mouse_x as f64) < self.character_pos.0 {
                self.set_flipped_x(true);
            } else {
                self.set_flipped_x(false);
            }
        }

        // cache input so that they can be applied as soon as the animation is over
        self.input_cache.update(shared_state);

        // move character slowly to mouse pos
        if self.allows_moving() {
            if self.is_rolling {
                // special movement
                let (dx, dy) = self.roll_direction;
                let speed = 200.0;
                let dt = update_info.dt;
                self.character_pos.0 += dx * speed * dt;
                self.character_pos.1 += dy * speed * dt;
            } else {
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
                    if speed > 50.0 {
                        self.animation_controller.set_animation(PlayerState::Run);
                    } else {
                        self.animation_controller.set_animation(PlayerState::Walk);
                    }
                } else {
                    self.animation_controller.set_animation(PlayerState::Idle);
                }
            }
        }

        // only allow other actions if we're done with the current one
        if self.allows_new_oneshot() {
            if self.input_cache.lmb.consume() {
                // trigger axe animation
                self.animation_controller.set_animation_override(PlayerState::Axe);
            }

            if self.input_cache.rmb.consume() {
                // trigger sword animation
                self.animation_controller.set_animation_override(PlayerState::Sword);
            }

            if self.input_cache.space.consume() {
                // trigger jump
                self.animation_controller.set_animation_override(PlayerState::Jump);
            }

            let mut roll_direction = None;
            if self.input_cache.w.consume() {
                roll_direction = Some((0.0, -1.0));
            }
            if self.input_cache.a.consume() {
                roll_direction = Some((-1.0, 0.0));
                self.set_flipped_x(true);
            }
            if self.input_cache.s.consume() {
                roll_direction = Some((0.0, 1.0));
            }
            if self.input_cache.d.consume() {
                roll_direction = Some((1.0, 0.0));
                self.set_flipped_x(false);
            }

            if let Some(roll_direction) = roll_direction {
                // trigger roll
                self.animation_controller.set_animation_override(PlayerState::Roll);
                self.is_rolling = true;
                self.roll_direction = roll_direction;
            }

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
                        // spawn animation, taking into consideration the x offset from the axe
                        let x_offset = if self.is_flipped_x { -20 } else { 20 };
                        let anim = get_animation(AnimationRepositoryKey::ChimneySmoke02);
                        self.set_and_forget_animations.add((draw_x + x_offset, draw_y - 10), anim);

                    }
                    if state == PlayerState::Roll {
                        // stop rolling
                        self.is_rolling = false;
                    }
                }
                KeyedAnimationResult::Finished(state) => {
                    if state == PlayerState::Axe {
                        // axe animation was finished
                        // TODO: this does not get triggered because the above blanket setting to ::Idle overrides the axe animation, since it's 'finished' so it can be overriden despite
                        // the 'finished' not being consumed. Though I guess that's fine? as long as our trigger is consumed...
                        shared_state.debug_messages.push(DebugMessage::new_3s("Axe animation finished!"));
                    }
                    self.animation_controller.set_animation(PlayerState::Idle);

                }
            }
        }
        // render all set and forget animations
        self.set_and_forget_animations.render_to_hbd(&mut self.hbd, update_info.current_time);



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

    init_animation_repository();

    let mut game = Game::new_with_custom_buf_writer();
    game.install_recommended_components();
    game.add_component(Box::new(GameComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
