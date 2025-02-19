//! A visual tool to check the functionality of `Bounds::union` and `Bounds::subtract`

use std::io;
use std::io::stdout;
use std::time::Instant;
use teng::components::Component;
use teng::rendering::pixel::Pixel;
use teng::rendering::renderer::Renderer;
use teng::util::planarvec::Bounds;
use teng::{
    install_panic_handler, terminal_cleanup, terminal_setup, Game, SharedState, UpdateInfo,
};

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new(stdout());
    game.install_recommended_components();
    game.add_component(Box::new(BoundsCheckerComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}

pub struct BoundsCheckerComponent {
    first_loc: Option<(usize, usize)>,
    second_loc: Option<(usize, usize)>,
    sub_first_loc: Option<(usize, usize)>,
    sub_second_loc: Option<(usize, usize)>,
    union_instead_of_subtract: bool,
}

impl BoundsCheckerComponent {
    pub fn new() -> Self {
        Self {
            first_loc: None,
            second_loc: None,
            sub_first_loc: None,
            sub_second_loc: None,
            union_instead_of_subtract: false,
        }
    }
}

impl Component for BoundsCheckerComponent {
    fn update(&mut self, _update_info: UpdateInfo, shared_state: &mut SharedState) {
        // was there a mouse down event?
        let pressed = shared_state.mouse_pressed.left;
        if pressed {
            self.first_loc = None;
            self.second_loc = None;

            self.first_loc = Some(shared_state.mouse_info.last_mouse_pos);
        }

        // was there a mouse up event?
        if !shared_state.mouse_info.left_mouse_down
            && self.first_loc.is_some()
            && self.second_loc.is_none()
        {
            // set the second location
            self.second_loc = Some(shared_state.mouse_info.last_mouse_pos);
        }

        // was there a mouse down event for right?
        let pressed = shared_state.mouse_pressed.right;
        if pressed {
            self.sub_first_loc = None;
            self.sub_second_loc = None;

            self.sub_first_loc = Some(shared_state.mouse_info.last_mouse_pos);
        }

        // was there a mouse up event for right?
        if !shared_state.mouse_info.right_mouse_down
            && self.sub_first_loc.is_some()
            && self.sub_second_loc.is_none()
        {
            // set the second location
            self.sub_second_loc = Some(shared_state.mouse_info.last_mouse_pos);
        }

        if shared_state.pressed_keys.did_press_char_ignore_case(' ') {
            self.union_instead_of_subtract = !self.union_instead_of_subtract;
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX - 1;

        let mut min_x = 0;
        let mut max_x = 0;
        let mut min_y = 0;
        let mut max_y = 0;

        let mut first_bounds = None;

        if let Some((x, y)) = self.first_loc {
            let first_pos = (x, y);

            let second_pos = if let Some((x, y)) = self.second_loc {
                (x, y)
            } else {
                // must be still pressing, so just take this for rendering
                shared_state.mouse_info.last_mouse_pos
            };

            min_x = first_pos.0.min(second_pos.0);
            max_x = first_pos.0.max(second_pos.0);
            min_y = first_pos.1.min(second_pos.1);
            max_y = first_pos.1.max(second_pos.1);

            let bounds = Bounds {
                min_x: min_x as i64,
                max_x: max_x as i64,
                min_y: min_y as i64,
                max_y: max_y as i64,
            };

            first_bounds = Some(bounds);
        }

        let mut sub_bounds = None;

        if let Some((x, y)) = self.sub_first_loc {
            let first_pos = (x, y);

            let second_pos = if let Some((x, y)) = self.sub_second_loc {
                (x, y)
            } else {
                // must be still pressing, so just take this for rendering
                shared_state.mouse_info.last_mouse_pos
            };

            min_x = first_pos.0.min(second_pos.0);
            max_x = first_pos.0.max(second_pos.0);
            min_y = first_pos.1.min(second_pos.1);
            max_y = first_pos.1.max(second_pos.1);

            let bounds = Bounds {
                min_x: min_x as i64,
                max_x: max_x as i64,
                min_y: min_y as i64,
                max_y: max_y as i64,
            };

            sub_bounds = Some(bounds);
        }

        if self.union_instead_of_subtract {
            // render the union
            match (first_bounds, sub_bounds) {
                (Some(b1), Some(b2)) => {
                    let bound = b1.union(b2);
                    for x in bound.min_x..=bound.max_x {
                        for y in bound.min_y..=bound.max_y {
                            renderer.render_pixel(
                                x as usize,
                                y as usize,
                                Pixel::new('█'),
                                depth_base,
                            );
                        }
                    }
                }
                _ => {}
            }
        } else {
            // render the difference
            if let Some(bounds) = first_bounds {
                let the_bounds = bounds.subtract(sub_bounds.unwrap_or(Bounds::empty()));

                for bound in the_bounds.iter() {
                    for x in bound.min_x..=bound.max_x {
                        for y in bound.min_y..=bound.max_y {
                            renderer.render_pixel(
                                x as usize,
                                y as usize,
                                Pixel::new('█'),
                                depth_base,
                            );
                        }
                    }
                }
            }
        }
    }
}
