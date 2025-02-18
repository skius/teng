use std::io;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::Render;
use teng::rendering::renderer::Renderer;
use teng::{install_panic_handler, terminal_cleanup, terminal_setup, Game, SharedState};
use teng::components::Component;

struct MyComponent;

impl Component for MyComponent {
    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let width = shared_state.display_info.width();
        let height = shared_state.display_info.height();
        let x = width / 2;
        let y = height / 2;
        let pixel = Pixel::new('█').with_color([0, 255, 0]);
        renderer.render_pixel(x, y, pixel, depth_base);

        "Hello World"
            .with_bg_color([255, 0, 0])
            .render(renderer, x, y + 1, depth_base);
    }
}

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new_with_custom_buf_writer();
    // If you don't install the recommended components, you will need to have your own
    // component that exits the process, since Ctrl-C does not work in raw mode.
    game.install_recommended_components();
    game.add_component(Box::new(MyComponent));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
