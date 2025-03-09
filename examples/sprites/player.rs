use crate::GameState;
use crate::animationcontroller::{AnimationController, KeyedAnimationResult};
use crate::impulse::Trigger;
use crate::setandforgetanimations::SetAndForgetAnimations;
use crate::sprite::{AnimationRepositoryKey, get_animation};
use teng::components::Component;
use teng::components::debuginfo::DebugMessage;
use teng::components::keyboard::PressedKeys;
use teng::components::mouse::MousePressedInfo;
use teng::rendering::render::HalfBlockDisplayRender;
use teng::{SetupInfo, SharedState, UpdateInfo};

// Handles updating the player struct from the global gamestate
pub struct PlayerComponent;

impl Component<GameState> for PlayerComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<GameState>) {
        let x = setup_info.display_info.width() as i64 / 2;
        let y = setup_info.display_info.height() as i64 / 2 * 2; // * 2 because world is 2x taller than screen
        shared_state.custom.player.character_pos = (x as f64, y as f64);
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        let player = &mut shared_state.custom.player;
        player
            .input_cache
            .update(&shared_state.pressed_keys, &shared_state.mouse_pressed);

        Player::update(update_info, shared_state);
    }
}

#[derive(Debug, Hash, Eq, PartialEq, Default, Clone, Copy)]
pub enum PlayerState {
    #[default]
    Idle,
    Walk,
    Run,
    Jump,
    Roll,
    Axe,
    Sword,
}

#[derive(Debug, Default)]
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
    // TODO: extract all input related things into a sub-struct of SharedState?
    fn update(&mut self, pressed_keys: &PressedKeys, mouse_pressed: &MousePressedInfo) {
        if mouse_pressed.left {
            self.lmb.set();
        }
        if mouse_pressed.right {
            self.rmb.set();
        }
        if pressed_keys.did_press_char_ignore_case(' ') {
            self.space.set();
        }
        if pressed_keys.did_press_char_ignore_case('w') {
            self.w.set();
        }
        if pressed_keys.did_press_char_ignore_case('a') {
            self.a.set();
        }
        if pressed_keys.did_press_char_ignore_case('s') {
            self.s.set();
        }
        if pressed_keys.did_press_char_ignore_case('d') {
            self.d.set();
        }
    }
}

#[derive(Debug)]
pub struct Player {
    animation_controller: AnimationController<PlayerState>,
    is_rolling: bool,
    roll_direction: (f64, f64),
    is_flipped_x: bool,
    character_pos: (f64, f64),
    // TODO: have a way to time out the input cache, so that a key press is not consumed if it's too old
    input_cache: InputCache,
}

impl Default for Player {
    fn default() -> Self {
        let mut animation_controller = AnimationController::default();

        animation_controller.register_animations_from_repository(vec![
            (PlayerState::Idle, AnimationRepositoryKey::PlayerIdle),
            (PlayerState::Walk, AnimationRepositoryKey::PlayerWalk),
            (PlayerState::Axe, AnimationRepositoryKey::PlayerAxe),
            (PlayerState::Sword, AnimationRepositoryKey::PlayerSword),
            (PlayerState::Jump, AnimationRepositoryKey::PlayerJump),
            (PlayerState::Roll, AnimationRepositoryKey::PlayerRoll),
            (PlayerState::Run, AnimationRepositoryKey::PlayerRun),
        ]);

        Self {
            animation_controller,
            is_flipped_x: false,
            is_rolling: false,
            roll_direction: (0.0, 0.0),
            character_pos: (0.0, 0.0),
            input_cache: InputCache::default(),
        }
    }
}

impl Player {
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

    fn update(update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        let player = &mut shared_state.custom.player;
        let hbd = &mut shared_state.custom.hbd;

        // check if mouse pos is on left or right half of screen, and flip accordingly
        if player.allows_flipping_x() {
            let mouse_x = shared_state.mouse_info.last_mouse_pos.0;
            if (mouse_x as f64) < player.character_pos.0 {
                player.set_flipped_x(true);
            } else {
                player.set_flipped_x(false);
            }
        }

        // move character slowly to mouse pos
        if player.allows_moving() {
            if player.is_rolling {
                // special movement
                let (dx, dy) = player.roll_direction;
                let speed = 200.0;
                let dt = update_info.dt;
                player.character_pos.0 += dx * speed * dt;
                player.character_pos.1 += dy * speed * dt;
            } else {
                let (mouse_x, mouse_y) = shared_state.mouse_info.last_mouse_pos;
                let mouse_x = mouse_x as i64;
                let mouse_y = mouse_y as i64 * 2; // world is 2x taller than screen
                let (char_x, char_y) = player.character_pos;

                let dx = mouse_x as f64 - char_x;
                let dy = mouse_y as f64 - char_y;
                let dist_sqr = (dx * dx + dy * dy);
                if dist_sqr > 10.0 * 10.0 {
                    // move character
                    let dist = dist_sqr.sqrt();
                    let speed = player.speed_from_distance(dist);
                    // panic!("speed: {}", speed);
                    // let speed = 20.0;
                    let dt = update_info.dt;
                    let normalized = (dx / dist, dy / dist);
                    let (dx, dy) = normalized;
                    player.character_pos.0 += dx * speed * dt;
                    player.character_pos.1 += dy * speed * dt;
                    if speed > 50.0 {
                        player.animation_controller.set_animation(PlayerState::Run);
                    } else {
                        player.animation_controller.set_animation(PlayerState::Walk);
                    }
                } else {
                    player.animation_controller.set_animation(PlayerState::Idle);
                }
            }
        }

        // only allow other actions if we're done with the current one
        if player.allows_new_oneshot() {
            if player.input_cache.lmb.consume() {
                // trigger axe animation
                player
                    .animation_controller
                    .set_animation_override(PlayerState::Axe);
            }

            if player.input_cache.rmb.consume() {
                // trigger sword animation
                player
                    .animation_controller
                    .set_animation_override(PlayerState::Sword);
            }

            if player.input_cache.space.consume() {
                // trigger jump
                player
                    .animation_controller
                    .set_animation_override(PlayerState::Jump);
            }

            let mut roll_direction = None;
            if player.input_cache.w.consume() {
                roll_direction = Some((0.0, -1.0));
            }
            if player.input_cache.a.consume() {
                roll_direction = Some((-1.0, 0.0));
                player.set_flipped_x(true);
            }
            if player.input_cache.s.consume() {
                roll_direction = Some((0.0, 1.0));
            }
            if player.input_cache.d.consume() {
                roll_direction = Some((1.0, 0.0));
                player.set_flipped_x(false);
            }

            if let Some(roll_direction) = roll_direction {
                // trigger roll
                player
                    .animation_controller
                    .set_animation_override(PlayerState::Roll);
                player.is_rolling = true;
                player.roll_direction = roll_direction;
            }
        }

        // render
        hbd.clear();
        let (draw_x, draw_y) = player.character_pos;
        let draw_x = draw_x.floor() as i64;
        let draw_y = draw_y.floor() as i64;
        // for animation in &player.animations {
        //     animation.render_to_hbd(draw_x, draw_y, &mut player.hbd, update_info.current_time);
        // }

        let anim_res = player.animation_controller.render_to_hbd(
            draw_x,
            draw_y,
            hbd,
            update_info.current_time,
        );
        if let Some(anim_res) = anim_res {
            match anim_res {
                KeyedAnimationResult::Triggered(state) => {
                    if state == PlayerState::Axe {
                        // axe animation was triggered
                        shared_state
                            .debug_messages
                            .push(DebugMessage::new_3s("Axe animation triggered!"));
                        // spawn animation, taking into consideration the x offset from the axe
                        let x_offset = if player.is_flipped_x { -20 } else { 20 };
                        let anim = get_animation(AnimationRepositoryKey::ChimneySmoke02);
                        shared_state
                            .custom
                            .set_and_forget_animations
                            .add((draw_x + x_offset, draw_y - 10), anim);
                    }
                    if state == PlayerState::Roll {
                        // stop rolling
                        player.is_rolling = false;
                    }
                }
                KeyedAnimationResult::Finished(state) => {
                    if state == PlayerState::Axe {
                        // axe animation was finished
                        // TODO: this does not get triggered because the above blanket setting to ::Idle overrides the axe animation, since it's 'finished' so it can be overriden despite
                        // the 'finished' not being consumed. Though I guess that's fine? as long as our trigger is consumed...
                        shared_state
                            .debug_messages
                            .push(DebugMessage::new_3s("Axe animation finished!"));
                    }
                    player.animation_controller.set_animation(PlayerState::Idle);
                }
            }
        }
        // render all set and forget animations
        shared_state
            .custom
            .set_and_forget_animations
            .render_to_hbd(hbd, update_info.current_time);
    }
}
