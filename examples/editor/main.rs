use std::io;
use crokey::key;
use crossterm::event::{Event, MouseEventKind};
use teng::components::Component;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::{HalfBlockDisplayRender, Render};
use teng::rendering::renderer::Renderer;
use teng::{Game, SharedState, install_panic_handler, terminal_cleanup, terminal_setup, UpdateInfo, SetupInfo, BreakingAction};
use teng::components::ui::{UiComponent, UiElement};
use teng::rendering::color::Color;
use teng::util::planarvec::{Bounds, PlanarVec};

// Renders in a half block display.
struct PreviewWindow {
    hbd: HalfBlockDisplayRender,
    size: (usize, usize),
}

impl PreviewWindow {
    fn new() -> Self {
        Self {
            hbd: HalfBlockDisplayRender::new(1,1),
            size: (1, 1),
        }
    }
}

impl UiElement<State> for PreviewWindow {
    fn update(&mut self, shared_state: &mut SharedState<State>) {
        let (width, height) = shared_state.custom.screen_size;
        self.size = (width as usize, height as usize); // terminal pixels
        self.hbd.resize_discard(self.size.0, self.size.1 * 2); // times two due to half pixels
        self.hbd.clear();

        let mouse_pos = shared_state.custom.last_mouse_pos;

        // render the image to the half block display
        for y in 0..self.hbd.height() {
            for x in 0..self.hbd.width() {
                let checker_color = shared_state.custom.screen_to_checkerboard_raw(x, y);
                self.hbd.set_color(x, y, checker_color);

                let image_pos@(image_x, image_y) = shared_state.custom.screen_to_image_raw(x, y);
                if image_pos == mouse_pos {
                    let mut pixel = Pixel::new('█');
                    pixel.color = Color::Rgb([200; 3]);
                    self.hbd.set_color(x, y, pixel.color);
                }
                let color = shared_state.custom.image[(image_x, image_y)];
                if !color.is_solid() {
                    continue;
                }
                self.hbd.set_color(x, y, color);
            }
        }
    }

    fn get_size(&self) -> (usize, usize) {
        self.size
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<State>, depth_base: i32) {
        self.hbd.render(renderer, 0, 0, depth_base);
    }
}



// Renders at a scale.
struct DrawWindow {
    size: (usize, usize),
}

impl DrawWindow {
    fn new() -> Self {
        Self {
            size: (1, 1),
        }
    }
}

impl UiElement<State> for DrawWindow {
    fn update(&mut self, shared_state: &mut SharedState<State>) {
        let (width, height) = shared_state.custom.screen_size;
        self.size = (width as usize, height as usize);
    }

    fn get_size(&self) -> (usize, usize) {
        self.size
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<State>, depth_base: i32) {
        let depth_checkerboard = depth_base;
        let depth_mouse = depth_base + 1;
        let depth_drawing = depth_base + 2;
        let (width, height) = shared_state.custom.screen_size;

        let mouse_image_pos = shared_state.custom.last_mouse_pos;

        for x in 0..width {
            for y in 0..height {
                let x = x as usize;
                let y = y as usize;
                let color = shared_state.custom.screen_to_checkerboard(x, y);
                let mut pixel = Pixel::new(' ');
                pixel.bg_color = color;
                renderer.render_pixel(x, y, pixel, depth_checkerboard);

                let image_pos@(image_x, image_y) = shared_state.custom.screen_to_image(x, y);
                if image_pos == mouse_image_pos {
                    let mut pixel = Pixel::new('█');
                    pixel.color = Color::Rgb([200; 3]);
                    renderer.render_pixel(x, y, pixel, depth_mouse);
                }

                let color = shared_state.custom.image[(image_x, image_y)];
                let mut pixel = Pixel::new('█');
                pixel.color = color;
                renderer.render_pixel(x, y, pixel, depth_drawing);
            }
        }
    }
}

#[derive(Debug, Default)]
struct EditHistory {
    edits: Vec<(i64, i64, Color, Color)>,
    // The most recent change is setting (x, y) to color. If we want to change the same coords to the same color, we don't need to record that.
    last_edit: Option<(i64, i64, Color)>,
}

impl EditHistory {
    fn add_edit(&mut self, x: i64, y: i64, old_color: Color, new_color: Color) {
        if self.last_edit == Some((x, y, new_color)) {
            return;
        }
        self.edits.push((x, y, old_color, new_color));
        self.last_edit = Some((x, y, new_color));
    }

    fn undo_one(&mut self) -> Option<(i64, i64, Color, Color)> {
        if let Some(edit@(x, y, old_color, _)) = self.edits.pop() {
            self.last_edit = Some((x, y, old_color));
            Some(edit)
        } else {
            None
        }
    }
}


#[derive(Debug)]
struct State {
    // y goes up, x goes right.
    image: PlanarVec<Color>,
    default_color: Color,
    // in image coordinates. // TODO: really?
    camera_center: (i64, i64),
    // The scale of the editor in half pixels. To support intuitive mapping on mouse events, a minimum scale of 2 is required.
    editor_scale: i64,
    // The size of the screen in terminal pixels. Really this is half the width of the actual window. should probably split it up and give it to the individual UiElements
    screen_size: (i64, i64),
    // TODO: have some history of edits, Edit(coord, prev_color, new_color), that a user can undo. should be more than just pixel edits, maybe on the granularity of entire lines (holding LMB down)?
    // actually no, single-pixel changes are enough.
    history: EditHistory,
    // used to draw a grey hover
    last_mouse_pos: (i64, i64),
}

impl Default for State {
    fn default() -> Self {
        Self {
            image: PlanarVec::default(),
            default_color: Color::Transparent,
            // default_color: Color::Default, // TODO: BUG with this
            camera_center: (0, 0),
            editor_scale: 2,
            screen_size: (1, 1),
            history: EditHistory::default(),
            last_mouse_pos: (0, 0),
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

    const CHECKERBOARD_SCALE: i64 = 3;

    fn screen_to_image(&self, screen_x: usize, screen_y: usize) -> (i64, i64) {
        let (camera_x, camera_y) = self.camera_center;
        let (screen_width, screen_height) = self.screen_size;
        let scale = self.editor_scale;
        let screen_x_offset = screen_x as i64 - screen_width as i64 / 2;
        let screen_y_offset = screen_y as i64 - screen_height as i64 / 2;
        let image_x = camera_x + div_floor(screen_x_offset, scale);
        let image_y = camera_y - div_floor(screen_y_offset * 2, scale); // - because the y axis is flipped, *2 because we want square displays
        (image_x, image_y)
    }

    /// Expects square pixel coordinates and ignores scale.
    fn screen_to_image_raw(&self, screen_x: usize, screen_y: usize) -> (i64, i64) {
        let (camera_x, camera_y) = self.camera_center;
        let (screen_width, screen_height) = self.screen_size;
        let screen_x_offset = screen_x as i64 - screen_width as i64 / 2;
        let screen_y_offset = screen_y as i64 - screen_height as i64 / 2 * 2; // * 2 because we're in pixel coordinates
        let image_x = camera_x + screen_x_offset;
        let image_y = camera_y - screen_y_offset;
        (image_x, image_y)
    }

    fn screen_to_checkerboard(&self, screen_x: usize, screen_y: usize) -> Color {
        let (image_x, image_y) = self.screen_to_image(screen_x, screen_y);
        let image_x = div_floor(image_x, Self::CHECKERBOARD_SCALE);
        let image_y = div_floor(image_y, Self::CHECKERBOARD_SCALE);
        let color_a = Color::Rgb([50; 3]);
        let color_b = Color::Rgb([100; 3]);
        if (image_x + image_y) % 2 == 0 {
            color_a
        } else {
            color_b
        }
    }

    fn screen_to_checkerboard_raw(&self, screen_x: usize, screen_y: usize) -> Color {
        let (image_x, image_y) = self.screen_to_image_raw(screen_x, screen_y);
        let image_x = div_floor(image_x, Self::CHECKERBOARD_SCALE);
        let image_y = div_floor(image_y, Self::CHECKERBOARD_SCALE);
        let color_a = Color::Rgb([50; 3]);
        let color_b = Color::Rgb([100; 3]);
        if (image_x + image_y) % 2 == 0 {
            color_a
        } else {
            color_b
        }
    }

    fn camera_bounds(&self) -> Bounds {
        // From the bottom left corner to the top right corner.
        let (min_x, min_y) = self.screen_to_image_raw(0, 2 * self.screen_size.1 as usize);
        let (max_x, max_y) = self.screen_to_image_raw(self.screen_size.0 as usize, 0);
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

    fn set_mouse_pos(&mut self, pos: (usize, usize)) {
        self.last_mouse_pos = self.screen_to_image(pos.0, pos.1);
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

    fn draw_pixel(&mut self, x: i64, y: i64, color: Color) {
        let old_color = self.image[(x, y)];
        self.history.add_edit(x, y, old_color, color);
        self.image[(x, y)] = color;
    }

    fn undo_one(&mut self) {
        if let Some((x, y, old_color, _)) = self.history.undo_one() {
            self.image[(x, y)] = old_color;
        }
    }
}

struct DrawComponent {
    combiner: crokey::Combiner,
}

impl DrawComponent {
    fn new() -> Self {
        Self {
            combiner: crokey::Combiner::default(),
        }
    }
}

impl Component<State> for DrawComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<State>) {
        shared_state.ui.add_window("draw_window", 0, 0, Box::new(DrawWindow::new()));
        shared_state.ui.add_window("preview_window", 0, 0, Box::new(PreviewWindow::new()));

        self.on_resize(setup_info.display_info.width(), setup_info.display_info.height(), shared_state);
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<State>) {
        shared_state.custom.resize(width / 2, height);
        shared_state.ui.set_anchor("preview_window", width / 2, 0);
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

        if let Event::Key(ke) = event {
            if let Some(key_combination) = self.combiner.transform(ke) {
                match key_combination {
                    key!(ctrl-z) => {
                        shared_state.custom.undo_one();
                    }
                    _ => {}
                }
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

        shared_state.custom.set_mouse_pos(shared_state.mouse_info.last_mouse_pos);

        if shared_state.mouse_info.left_mouse_down {
            let (x, y) = shared_state.mouse_info.last_mouse_pos;
            let (image_x, image_y) = shared_state.custom.screen_to_image(x, y);
            shared_state.custom.draw_pixel(image_x, image_y, Color::Rgb([255, 255, 255]));
        }
        if shared_state.mouse_info.right_mouse_down {
            let (x, y) = shared_state.mouse_info.last_mouse_pos;
            let (image_x, image_y) = shared_state.custom.screen_to_image(x, y);
            shared_state.custom.draw_pixel(image_x, image_y, shared_state.custom.default_color);
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<State>, depth_base: i32) {
        // Each element in the image is a half block pixel, so in terminal resolution it's 1x0.5 pixels.
        // we render it at 'scale' * 1x0.5 pixels.

        // let screen_width = shared_state.display_info.width();
        // let screen_height = shared_state.display_info.height();
        //
        // for x in 0..screen_width {
        //     for y in 0..screen_height {
        //         let (image_x, image_y) = shared_state.custom.screen_to_image(x, y);
        //         let color = shared_state.custom.image[(image_x, image_y)];
        //         let pixel = Pixel::new(' ').with_bg_color(color.unwrap_or([0,0,0]));
        //         renderer.render_pixel(x, y, pixel, depth_base);
        //     }
        // }
    }
}

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new_with_custom_buf_writer();
    // If you don't install the recommended components, you will need to have your own
    // component that exits the process, since Ctrl-C does not work in raw mode.
    game.install_recommended_components();
    game.add_component(Box::new(DrawComponent::new()));
    game.add_component(Box::new(UiComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
