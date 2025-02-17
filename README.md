# teng 📟 
A minimal, cross-platform game engine for the terminal with a focus on performance

## Getting Started
teng uses components as the building blocks. Every frame, each component (optionally):
- Handles received events (mouse, keyboard, resizes, etc.)
- Updates the game state
- Renders its core concept (if any) to the screen

Here's a simple example that renders static content to the screen:
```rust ,no_run
use std::io;
use teng::{install_panic_handler, terminal_cleanup, terminal_setup, Game, Pixel, Render, Renderer, SharedState};

struct MyComponent;

impl teng::Component for MyComponent {
    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let width = shared_state.display_info.width();
        let height = shared_state.display_info.height();
        let x = width / 2;
        let y = height / 2;
        let pixel = Pixel::new('█').with_color([0, 255, 0]);
        renderer.render_pixel(x, y, pixel, depth_base);

        "Hello World".with_bg_color([255, 0, 0]).render(&mut renderer, x, y+1, depth_base);
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
```

## Is teng an ECS?
Not really. teng's "Components" are quite similar to "Systems" in an ECS, but there is no built-in notion of entities or components in the ECS sense.
However, you can build an ECS inside teng quite easily, see [`examples/ecs`](examples/ecs/main.rs) for an example.