//! A visual tool to check the functionality of `Bounds::union` and `Bounds::subtract`

use std::io;
use std::io::stdout;
use std::time::Instant;
use teng::components::Component;
use teng::rendering::pixel::Pixel;
use teng::rendering::renderer::Renderer;
use teng::util::planarvec::Bounds;
use teng::util::planarvec2_experimental::ExponentialGrowingBounds;
use teng::{
    Game, SetupInfo, SharedState, UpdateInfo, install_panic_handler, terminal_cleanup,
    terminal_setup,
};

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new(stdout());
    game.install_recommended_components();
    game.add_component(Box::new(ExponentialBoundsCheckerComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}

pub struct ExponentialBoundsCheckerComponent {
    exp_bounds: ExponentialGrowingBounds,
    screen_width: usize,
    screen_height: usize,
}

impl ExponentialBoundsCheckerComponent {
    pub fn new() -> Self {
        Self {
            exp_bounds: ExponentialGrowingBounds::new(),
            screen_width: 0,
            screen_height: 0,
        }
    }

    fn center_screen(&self) -> (usize, usize) {
        (self.screen_width / 2, self.screen_height / 2)
    }

    fn screen_coords_to_bounds_coords(&self, x: usize, y: usize) -> (i64, i64) {
        let (center_x, center_y) = self.center_screen();
        let x = x as i64 - center_x as i64;
        let y = y as i64 - center_y as i64;
        (x, y)
    }

    fn bounds_coords_to_screen_coords(&self, x: i64, y: i64) -> (usize, usize) {
        let (center_x, center_y) = self.center_screen();
        let x = (x + center_x as i64) as usize;
        let y = (y + center_y as i64) as usize;
        (x, y)
    }
}

impl Component for ExponentialBoundsCheckerComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<()>) {
        self.on_resize(
            setup_info.display_info.width(),
            setup_info.display_info.height(),
            shared_state,
        );
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<()>) {
        self.screen_width = width;
        self.screen_height = height;
    }

    fn update(&mut self, _update_info: UpdateInfo, shared_state: &mut SharedState) {
        if shared_state.mouse_info.left_mouse_down {
            let (x, y) = shared_state.mouse_info.last_mouse_pos;
            let (x, y) = self.screen_coords_to_bounds_coords(x, y);
            self.exp_bounds.grow_to_contain((x, y));
        }

        if shared_state.mouse_pressed.right {
            self.exp_bounds = ExponentialGrowingBounds::new();
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        // Draw the bounds

        for b_x in self.exp_bounds.min_x()..=self.exp_bounds.max_x() {
            for b_y in self.exp_bounds.min_y()..=self.exp_bounds.max_y() {
                let (x, y) = self.bounds_coords_to_screen_coords(b_x, b_y);
                renderer.render_pixel(x, y, Pixel::new('█'), depth_base);
            }
        }
    }
}
