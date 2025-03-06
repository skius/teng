use std::io;
use crossterm::event::{Event, KeyCode};
use teng::components::Component;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::Render;
use teng::rendering::renderer::Renderer;
use teng::{install_panic_handler, terminal_cleanup, terminal_setup, BreakingAction, Game, SetupInfo, SharedState, UpdateInfo};
use teng::components::ui::{UiComponent, UiElement};

struct MyWindow {
    title_bar_height: usize,
    width: usize,
    height: usize,
    text_box: String,
    background_color: [u8; 3],
}

impl MyWindow {
    fn new() -> Self {
        let background_color = [rand::random::<u8>(), rand::random::<u8>(), rand::random::<u8>()];
        Self {
            title_bar_height: 1,
            width: 20,
            height: 10,
            text_box: String::new(),
            background_color,
        }
    }
}

impl UiElement for MyWindow {
    fn is_hover_drag(&self, x: usize, y: usize) -> bool {
        y < self.title_bar_height
    }

    fn is_resizing_drag(&self, x: usize, y: usize) -> bool {
        x >= self.width - 1 && y >= self.height - 1
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<()>) {
        self.width = width;
        self.height = height;
    }

    fn get_size(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    fn on_event(&mut self, event: Event, shared_state: &mut SharedState<()>) -> Option<BreakingAction> {
        if let Event::Key(key_event) = event {
            if let KeyCode::Char(c) = key_event.code {
                self.text_box.push(c);
            }
            if key_event.code == KeyCode::Backspace {
                self.text_box.pop();
            }
            if key_event.code == KeyCode::Enter {
                self.text_box.push('\n');
            }
        }
        None
    }

    fn render(&self, renderer: &mut dyn Renderer, depth_base: i32) {
        let depth_text_box = depth_base + 1;
        let depth_title_bar = depth_base + 2;

        let pixel = Pixel::new(' ').with_bg_color(self.background_color);

        for x in 0..self.width {
            for y in 1..self.height {
                renderer.render_pixel(x, y, pixel, depth_base);
            }
        }

        // text box
        self.text_box.render(renderer, 0, 1, depth_text_box);

        // title bar
        for x in 0..self.width {
            renderer.render_pixel(x, 0, Pixel::new(' ').with_bg_color([150, 150, 150]), depth_base);
        }
        "My Window".render(renderer, 0, 0, depth_title_bar);
        
        // the resize corner symbol
        let corner_expand_pixel = Pixel::new('⤡').with_bg_color([150, 150, 150]);
        renderer.render_pixel(self.width - 1, self.height - 1, corner_expand_pixel, depth_text_box);
    }
}

struct Setup;

impl Component for Setup {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<()>) {
        // add some UI components
        let width = shared_state.display_info.width();
        let height = shared_state.display_info.height();

        shared_state.ui.add_window(30, 15, Box::new(MyWindow::new()));

    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<()>) {
        if shared_state.pressed_keys.did_press_char_ignore_case(' ') {
            let width = shared_state.display_info.width();
            let height = shared_state.display_info.height();
            
            let anchor_x = rand::random::<usize>() % width;
            let anchor_y = rand::random::<usize>() % height;
            
            shared_state.ui.add_window(anchor_x, anchor_y, Box::new(MyWindow::new()));
        }
    }
}

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new_with_custom_buf_writer();
    // If you don't install the recommended components, you will need to have your own
    // component that exits the process, since Ctrl-C does not work in raw mode.
    game.install_recommended_components();
    game.add_component(Box::new(UiComponent::new()));
    game.add_component(Box::new(Setup));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
