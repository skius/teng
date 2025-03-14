//! This module contains functionality and data types used for anything GPU related.

use teng::components::Component;
use teng::{SetupInfo, SharedState, UpdateInfo};
use crate::GameState;
use crate::gpu::rendering::Instance;
use crate::gpu::sprite::TextureAnimationAtlas;

pub mod texture;
pub mod sprite;
pub mod rendering;

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
}

impl GpuComponent {
    pub fn new() -> Self {
        let (tex_atlas, tex_atlas_img) = TextureAnimationAtlas::load("examples/sprites/data/texture_atlas.png", "examples/sprites/data/teng_atlas_meta.json", "examples/sprites/data/imgpack_atlas_meta.json");

        let state = pollster::block_on(rendering::State::new((10, 10), tex_atlas_img));
        Self {
            state,
            phase: GpuPhase::TwoD,
            active: true,
            tex_atlas,
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

        if shared_state.mouse_info.left_mouse_down {
            let (x, y) = shared_state.mouse_info.last_mouse_pos;
            let y = 2 * y;
            self.state.update(x, y, &shared_state);
        }



        if shared_state.pressed_keys.did_press_char_ignore_case('p') {
            let (width, height) = self.state.get_size();
            // let rand_x = rand::random::<u32>() % width;
            // let rand_y = rand::random::<u32>() % height;
            let rand_x = 0;
            let rand_y = 0;
            for (idx, sprite) in self.tex_atlas.get_sprites_for_ca_with_frame("PlayerRun", 0).enumerate() {
                let instance = Instance {
                    center_offset: [sprite.center_offset[0] as f32, sprite.center_offset[1] as f32],
                    position: [rand_x as f32, rand_y as f32, 1.0 -0.1 * idx as f32],
                    size: [sprite.size[0] as f32, sprite.size[1] as f32],
                    sprite_tex_atlas_offset: [sprite.atlas_offset[0] as f32, sprite.atlas_offset[1] as f32],
                };

                self.state.add_instance(instance);
            }
        }

        let game_state = &mut shared_state.custom;

        let hbd = &mut game_state.hbd;


        shared_state.debug_info.custom.insert("adapter_info".to_string(), format!("{:?}", self.state.get_adapter_info()));

        // render to hbd
        self.state.render(hbd).unwrap()
    }
}