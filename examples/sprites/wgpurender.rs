mod learnwgpu;

use teng::components::Component;
use teng::rendering::renderer::Renderer;
use teng::{SetupInfo, SharedState, UpdateInfo};
use crate::GameState;

pub struct WgpuRenderComponent {
    state: learnwgpu::State,
}

impl WgpuRenderComponent {
    pub fn new() -> Self {
        let state = pollster::block_on(learnwgpu::State::new((10, 10)));

        Self {
            state,
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
        let game_state = &mut shared_state.custom;

        let hbd = &mut game_state.hbd;
        
        self.state.input(&shared_state.debounced_down_keys);
        self.state.update();
        
        shared_state.debug_info.custom.insert("adapter_info".to_string(), format!("{:?}", self.state.get_adapter_info()));

        // render to hbd
        self.state.render(hbd).unwrap()
    }
}