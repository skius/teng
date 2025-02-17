use std::io;
use std::io::stdout;
use std::time::Instant;
use teng::rendering::pixel::Pixel;
use teng::rendering::renderer::Renderer;
use teng::{
    install_panic_handler, terminal_cleanup, terminal_setup, Component, Game, SharedState,
    UpdateInfo,
};

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new(stdout());
    game.install_recommended_components();
    // game.add_component(Box::new(FpsCheckerComponent::new()));
    game.add_component(Box::new(FpsCheckerFrameCountComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}

/// Targets a movement speed of 144 pixels per second
pub struct FpsCheckerFrameTimeComponent {
    start_time: Instant,
}

impl FpsCheckerFrameTimeComponent {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

impl Component for FpsCheckerFrameTimeComponent {
    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        // render a block at half height and x corresponding to the position at 144 blocks per second
        let elapsed = Instant::now() - self.start_time;
        let time_per_frame = 1.0 / 144.0;
        let frame = (elapsed.as_secs_f64() / time_per_frame) as usize;
        let y = shared_state.display_info.height() / 2;
        // bounce x back and forth
        let x = frame % (shared_state.display_info.width() * 2);
        let pixel = Pixel::new('█');
        if x < shared_state.display_info.width() {
            renderer.render_pixel(x, y, pixel, i32::MAX);
        } else {
            renderer.render_pixel(
                shared_state.display_info.width() * 2 - x - 1,
                y,
                pixel,
                i32::MAX,
            );
        }
    }
}

/// Movement speed is tied to framerate. Moves 1px/frame.
pub struct FpsCheckerFrameCountComponent {
    count: usize,
}

impl FpsCheckerFrameCountComponent {
    pub fn new() -> Self {
        Self { count: 0 }
    }
}

impl Component for FpsCheckerFrameCountComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        self.count += 1;
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let y = shared_state.display_info.height() / 2;
        // bounce x back and forth
        let x = self.count % (shared_state.display_info.width() * 2);
        let pixel = Pixel::new('█');
        if x < shared_state.display_info.width() {
            renderer.render_pixel(x, y, pixel, i32::MAX);
        } else {
            renderer.render_pixel(
                shared_state.display_info.width() * 2 - x - 1,
                y,
                pixel,
                i32::MAX,
            );
        }
    }
}
