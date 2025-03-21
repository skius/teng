//! This module contains functionality and data types used for anything GPU related.

use std::rc::Rc;
use std::time::Instant;
use teng::components::Component;
use teng::{SetupInfo, SharedState, UpdateInfo};
use teng::components::debuginfo::DebugMessage;
use crate::GameState;
use crate::gpu::animation::{Animation, AnimationKind, AnimationResult};
use crate::gpu::animationcontroller::{AnimationController, AnimationStateMachine};
use crate::gpu::rendering::Instance;
use crate::gpu::sprite::{AnimationKey, TextureAnimationAtlas};

pub mod texture;
pub mod sprite;
pub mod rendering;
mod animation;
mod instancewriter;
mod animationcontroller;

#[derive(Clone, Copy, Debug)]
enum PlayerTriggerData {
    SwordOne,
}

#[derive(Hash, Eq, PartialEq, Copy, Clone, Debug)]
enum PlayerState {
    Idle,
    Walk,
    Run,
    Jump,
    Roll,
    Sword,
    Axe,
}

struct PlayerAsm {
    atlas: Rc<TextureAnimationAtlas>,
}

impl AnimationStateMachine for PlayerAsm {
    type State = PlayerState;
    type TriggerData = PlayerTriggerData;

    fn get_animation(&self, state: &Self::State) -> Animation<Self::TriggerData> {
        let default_duration = 0.1;
        match state {
            PlayerState::Idle => {
                Animation::new(&self.atlas, AnimationKey::PLAYER_IDLE, default_duration)
            }
            PlayerState::Walk => {
                Animation::new(&self.atlas, AnimationKey::PLAYER_WALKING, default_duration)
            }
            PlayerState::Run => {
                Animation::new(&self.atlas, AnimationKey::PLAYER_RUN, default_duration)
            }
            PlayerState::Jump => {
                Animation::new(&self.atlas, AnimationKey::PLAYER_JUMP, default_duration)
            }
            PlayerState::Roll => {
                Animation::new(&self.atlas, AnimationKey::PLAYER_ROLL, default_duration)
            }
            PlayerState::Sword => {
                Animation::new(&self.atlas, AnimationKey::PLAYER_ATTACK, default_duration)
                    .with_trigger(5, PlayerTriggerData::SwordOne)
                    .with_kind(AnimationKind::Once)
            }
            PlayerState::Axe => {
                Animation::new(&self.atlas, AnimationKey::PLAYER_AXE, default_duration)
            }
        }
    }

    fn next_state(&self, current_state: &Self::State, result: &AnimationResult<Self::TriggerData>) -> Self::State {
        PlayerState::Idle
    }

    fn get_atlas(&self) -> &TextureAnimationAtlas {
        &self.atlas
    }
}


enum GpuPhase {
    TwoD,
    // enable perspective projection etc
    RedPill,
}

pub struct GpuComponent {
    state: rendering::State,
    phase: GpuPhase,
    active: bool,
    tex_atlas: TextureAnimationAtlas,
    animtest: animation::Animation<()>,
    animtest_start: Instant,
    animcontroller: AnimationController<PlayerAsm>,
}

impl GpuComponent {
    pub fn new() -> Self {
        let (tex_atlas, tex_atlas_img) = TextureAnimationAtlas::load("examples/sprites/data/texture_atlas.png", "examples/sprites/data/teng_atlas_meta.json", "examples/sprites/data/imgpack_atlas_meta.json");

        let controller = AnimationController::new(PlayerAsm {
            atlas: Rc::new(tex_atlas.clone()),
        }, PlayerState::Idle);
        
        let anim = animation::Animation::new(&tex_atlas, AnimationKey::PLAYER_IDLE, 0.1);

        let state = pollster::block_on(rendering::State::new((10, 10), tex_atlas_img));
        Self {
            state,
            phase: GpuPhase::TwoD,
            active: true,
            tex_atlas,
            animtest: anim,
            animtest_start: Instant::now(),
            animcontroller: controller,
        }
    }
}

impl Component<GameState> for GpuComponent {
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
        self.state.resize((width as u32, 2 * height as u32));
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        if shared_state.pressed_keys.did_press_char_ignore_case('t') {
            self.active = !self.active;
        }
        if !self.active {
            return;
        }

        self.animtest.update(self.animtest_start.elapsed().as_secs_f32());
        
        if shared_state.pressed_keys.did_press_char_ignore_case('p') {
            self.animcontroller.set_animation(PlayerState::Sword);
        }
        
        let result = self.animcontroller.update();
        for t in result.triggers {
            match t {
                PlayerTriggerData::SwordOne => { 
                    shared_state.debug_messages.push(DebugMessage::new_3s("SwordOne"));
                }
            }
        }
        



        if shared_state.mouse_info.left_mouse_down {
            let (x, y) = shared_state.mouse_info.last_mouse_pos;
            let y = 2 * y;

            {
                let mut iw = self.state.instance_writer();
                self.animcontroller.render([x as f32, y as f32].into(), 2, &mut iw)
            }
            
            self.state.update(x, y, &mut self.animtest, &self.tex_atlas, shared_state);
        }




        // if shared_state.pressed_keys.did_press_char_ignore_case('p') {
        //     let (width, height) = self.state.get_size();
        //     // let rand_x = rand::random::<u32>() % width;
        //     // let rand_y = rand::random::<u32>() % height;
        //     let (x, y) = shared_state.mouse_info.last_mouse_pos;
        //     let y = 2 * y;
        //
        //     let rand_x = x;
        //     let rand_y = y;
        //     for (idx, sprite) in self.tex_atlas.get_sprites_for_ca_with_frame("PlayerRun", 0).enumerate() {
        //         let instance = Instance {
        //             center_offset: [sprite.center_offset[0] as f32, sprite.center_offset[1] as f32],
        //             position: [rand_x as f32, rand_y as f32, 1.0 -0.1 * idx as f32],
        //             size: [sprite.size[0] as f32, sprite.size[1] as f32],
        //             sprite_tex_atlas_offset: [sprite.atlas_offset[0] as f32, sprite.atlas_offset[1] as f32],
        //         };
        //
        //         self.state.add_instance(instance);
        //     }
        // }

        let game_state = &mut shared_state.custom;

        let hbd = &mut game_state.hbd;


        shared_state.debug_info.custom.insert("adapter_info".to_string(), format!("{:?}", self.state.get_adapter_info()));

        // render to hbd
        self.state.render(hbd).unwrap()
    }
}