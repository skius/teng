mod learnwgpu;

use std::time::Instant;
use teng::components::Component;
use teng::rendering::renderer::Renderer;
use teng::{SetupInfo, SharedState, UpdateInfo};
use crate::GameState;

pub struct WgpuRenderComponent {
    state: learnwgpu::State,
    active: bool,
    // state: learnwgpu::shadertoy::State,
}

impl WgpuRenderComponent {
    pub fn new() -> Self {
        let state = pollster::block_on(learnwgpu::State::new((10, 10)));

        Self {
            state,
            active: true,
        }
    }
}

impl Component<GameState> for WgpuRenderComponent {
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
        
        let game_state = &mut shared_state.custom;

        let hbd = &mut game_state.hbd;

        self.state.input(&shared_state.debounced_down_keys);

        if shared_state.pressed_keys.did_press_char_ignore_case('r') {
            self.state.update_texture_to_hbd(hbd);
        }

        self.state.update();

        shared_state.debug_info.custom.insert("adapter_info".to_string(), format!("{:?}", self.state.get_adapter_info()));

        // render to hbd
        self.state.render(hbd).unwrap()
    }
}


pub struct WgpuShadertoyRenderComponent {
    state: learnwgpu::shadertoy::State,
    frame_count: i32,
    start_time: Instant,
    mouse_pos_at_press_time: (f32, f32),
}

impl WgpuShadertoyRenderComponent {
    pub fn new() -> Self {
        let state = pollster::block_on(learnwgpu::shadertoy::State::new((10, 10)));

        Self {
            state,
            frame_count: 0,
            start_time: Instant::now(),
            mouse_pos_at_press_time: (-1.0, -1.0),
        }
    }
}

impl Component<GameState> for WgpuShadertoyRenderComponent {
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
        if shared_state.mouse_pressed.left {
            let (x, y) = shared_state.mouse_info.last_mouse_pos;
            let x = x as f32;
            let y = y as f32 * 2.0;
            self.mouse_pos_at_press_time = (x, y);
        } else if !shared_state.mouse_info.left_mouse_down {
            // TODO: really need mouse_released.left...

            self.mouse_pos_at_press_time = (-1.0, -1.0);
        }
        let (x, y) = shared_state.mouse_info.last_mouse_pos;
        let x = x as f32;
        let y = y as f32 * 2.0;

        self.state.set_mouse_input((x, y), self.mouse_pos_at_press_time);




        let game_state = &mut shared_state.custom;

        let hbd = &mut game_state.hbd;

        self.state.update(self.start_time.elapsed().as_secs_f32(), self.frame_count);
        self.frame_count += 1;

        shared_state.debug_info.custom.insert("adapter_info".to_string(), format!("{:?}", self.state.get_adapter_info()));

        let do_alpha = shared_state.mouse_info.right_mouse_down;

        // render to hbd
        self.state.render(hbd, do_alpha).unwrap()
    }
}