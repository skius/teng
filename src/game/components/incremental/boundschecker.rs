use crate::game::{Component, Pixel, Renderer, SharedState, UpdateInfo};
use crate::game::components::incremental::planarvec::Bounds;

pub struct BoundsCheckerComponent {
    first_loc: Option<(usize, usize)>,
    second_loc: Option<(usize, usize)>,
    sub_first_loc: Option<(usize, usize)>,
    sub_second_loc: Option<(usize, usize)>,
}

impl BoundsCheckerComponent {
    pub fn new() -> Self {
        Self {
            first_loc: None,
            second_loc: None,
            sub_first_loc: None,
            sub_second_loc: None,
        }
    }
}

impl Component for BoundsCheckerComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        // was there a mouse down event?
        let pressed = shared_state.mouse_pressed.left;
        if pressed {
            self.first_loc = None;
            self.second_loc = None;

            self.first_loc = Some(shared_state.mouse_info.last_mouse_pos);
        }

        // was there a mouse up event?
        if !shared_state.mouse_info.left_mouse_down && self.first_loc.is_some() && self.second_loc.is_none() {
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
        if !shared_state.mouse_info.right_mouse_down && self.sub_first_loc.is_some() && self.sub_second_loc.is_none() {
            // set the second location
            self.sub_second_loc = Some(shared_state.mouse_info.last_mouse_pos);
        }

    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX -1;
        
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

        
        if let Some(bounds) = first_bounds {
            let the_bounds = bounds.subtract(sub_bounds.unwrap_or(Bounds::empty()));
            
            for bound in the_bounds.iter() {
                for x in bound.min_x..=bound.max_x {
                    for y in bound.min_y..=bound.max_y {
                        renderer.render_pixel(x as usize, y as usize, Pixel::new('█'), depth_base);
                    }
                }
            }
        }
    }
}