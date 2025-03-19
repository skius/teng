use std::io;
use crossterm::event::{Event, MouseEventKind};
use teng::components::Component;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::Render;
use teng::rendering::renderer::Renderer;
use teng::{Game, SharedState, install_panic_handler, terminal_cleanup, terminal_setup, UpdateInfo, SetupInfo, BreakingAction};
use teng::rendering::color::Color;
use teng::util::planarvec::{Bounds, PlanarVec};


#[derive(Debug)]
struct State {
    // y goes up, x goes right.
    image: PlanarVec<Color>,
    default_color: Color,
    // in image coordinates. // TODO: really?
    camera_center: (i64, i64),
    // The scale of the editor in half pixels. To support intuitive mapping on mouse events, a minimum scale of 2 is required.
    editor_scale: i64,
    // The size of the screen in terminal pixels.
    screen_size: (i64, i64),
}

impl Default for State {
    fn default() -> Self {
        Self {
            image: Default::default(),
            default_color: Default::default(),
            camera_center: (0, 0),
            editor_scale: 2,
            screen_size: (1, 1),
        }
    }
}

fn div_floor(a: i64, b: i64) -> i64 {
    if a >= 0 {
        a / b
    } else {
        (a - b + 1) / b
    }
}

impl State {

    fn screen_to_image(&self, screen_x: usize, screen_y: usize) -> (i64, i64) {
        let (camera_x, camera_y) = self.camera_center;
        let (screen_width, screen_height) = self.screen_size;
        let scale = self.editor_scale;
        let screen_x_offset = screen_x as i64 - screen_width as i64 / 2;
        let screen_y_offset = screen_y as i64 - screen_height as i64 / 2;
        let image_x = camera_x + div_floor(screen_x_offset, scale);
        let image_y = camera_y - div_floor(screen_y_offset * 2, scale); // - because the y axis is flipped.
        (image_x, image_y)
    }

    fn camera_bounds(&self) -> Bounds {
        // From the bottom left corner to the top right corner.
        let (min_x, min_y) = self.screen_to_image(0, self.screen_size.1 as usize);
        let (max_x, max_y) = self.screen_to_image(self.screen_size.0 as usize, 0);
        Bounds {
            min_x,
            min_y,
            max_x,
            max_y,
        }
    }
    
    fn move_camera(&mut self, dx: i64, dy: i64) {
        self.camera_center.0 += dx;
        self.camera_center.1 += dy;
        self.adjust_screen_to_camera();
    }
    
    fn adjust_scale(&mut self, dscale: i64) {
        self.editor_scale += dscale;
        self.editor_scale = self.editor_scale.max(2);
        self.adjust_screen_to_camera();
    }
    
    fn adjust_screen_to_camera(&mut self) {
        let new_camera_bounds = self.camera_bounds();
        self.image.expand(new_camera_bounds, self.default_color);
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.screen_size = (width as i64, height as i64);
        self.adjust_screen_to_camera();
    }
}

struct DrawComponent;

impl Component<State> for DrawComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<State>) {
        self.on_resize(setup_info.display_info.width(), setup_info.display_info.height(), shared_state);
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<State>) {
        shared_state.custom.resize(width, height);
    }

    fn on_event(&mut self, event: Event, shared_state: &mut SharedState<State>) -> Option<BreakingAction> {
        if let Event::Mouse(me) = event {
            if let MouseEventKind::ScrollUp = me.kind {
                shared_state.custom.adjust_scale(2);
            }
            if let MouseEventKind::ScrollDown = me.kind {
                shared_state.custom.adjust_scale(-2);
            }
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<State>) {
        if shared_state.pressed_keys.did_press_char_ignore_case('c') {
            shared_state.custom.image.clear(shared_state.custom.default_color);
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('w') {
            shared_state.custom.move_camera(0, 1);
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('s') {
            shared_state.custom.move_camera(0, -1);
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('a') {
            shared_state.custom.move_camera(-1, 0);
        }
        if shared_state.pressed_keys.did_press_char_ignore_case('d') {
            shared_state.custom.move_camera(1, 0);
        }
        
        if shared_state.mouse_info.left_mouse_down {
            let (x, y) = shared_state.mouse_info.last_mouse_pos;
            let (image_x, image_y) = shared_state.custom.screen_to_image(x, y);
            shared_state.custom.image[(image_x, image_y)] = Color::Rgb([255, 255, 255]);
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<State>, depth_base: i32) {
        // Each element in the image is a half block pixel, so in terminal resolution it's 1x0.5 pixels.
        // we render it at 'scale' * 1x0.5 pixels.

        let screen_width = shared_state.display_info.width();
        let screen_height = shared_state.display_info.height();

        for x in 0..screen_width {
            for y in 0..screen_height {
                let (image_x, image_y) = shared_state.custom.screen_to_image(x, y);
                let color = shared_state.custom.image[(image_x, image_y)];
                let pixel = Pixel::new(' ').with_bg_color(color.unwrap_or([0,0,0]));
                renderer.render_pixel(x, y, pixel, depth_base);
            }
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
    game.add_component(Box::new(DrawComponent));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
