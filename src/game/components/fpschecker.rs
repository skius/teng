use crate::game::{Component, Pixel, Renderer, SharedState, UpdateInfo};
use std::time::Instant;

pub struct FpsCheckerComponent {
    start_time: Instant,
}

impl FpsCheckerComponent {
    pub fn new() -> Self {
        Self {
            start_time: Instant::now(),
        }
    }
}

impl Component for FpsCheckerComponent {
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
