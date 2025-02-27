//! FABRIK (Forward And Backward Reaching Inverse Kinematics) example.

use std::io;
use std::io::stdout;
use std::time::Instant;
use crossterm::event::KeyCode;
use teng::components::Component;
use teng::rendering::pixel::Pixel;
use teng::rendering::renderer::Renderer;
use teng::util::planarvec::Bounds;
use teng::util::planarvec2_experimental::ExponentialGrowingBounds;
use teng::{
    install_panic_handler, terminal_cleanup, terminal_setup, Game, SetupInfo, SharedState,
    UpdateInfo,
};
use teng::util::for_coord_in_line;

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new(stdout());
    game.install_recommended_components();
    game.add_component(Box::new(FabrikComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}

#[derive(Copy, Clone)]
struct Point<T> {
    x: T,
    y: T,
}

impl Point<f64> {
    fn distance(&self, other: &Self) -> f64 {
        ((self.x - other.x).powi(2) + (self.y - other.y).powi(2)).sqrt()
    }
}

struct Segment {
    start: Point<f64>,
    length: f64,
}

pub struct FabrikComponent {
    base_anchor: Option<Point<f64>>,
    target: Option<Point<f64>>,
    segments: Vec<Segment>,
    last_point: Point<f64>,
    currently_creating_segment: Option<Point<f64>>,
}

impl FabrikComponent {
    pub fn new() -> Self {
        Self {
            base_anchor: None,
            target: None,
            segments: vec![],
            last_point: Point { x: 0.0, y: 0.0 },
            currently_creating_segment: None,
        }
    }

    fn forward_reach(&mut self) {
        let target = self.target.unwrap();
        let mut last_point = target;
        self.last_point = target;
        for segment in self.segments.iter_mut().rev() {
            let start = last_point;
            let end = segment.start;

            let length = segment.length;
            let mut distance = start.distance(&end);
            if distance < 0.0001 {
                distance = 0.0001;
            }
            let ratio = length / distance;

            let new_end = Point {
                x: (1.0 - ratio) * start.x + ratio * end.x,
                y: (1.0 - ratio) * start.y + ratio * end.y,
            };

            segment.start = new_end;

            last_point = new_end;
        }
    }

    fn backward_reach(&mut self) {
        let base_anchor = self.base_anchor.unwrap();
        let mut last_point = base_anchor;
        for i in 0..self.segments.len() {
            let start = last_point;
            let end = if i < self.segments.len() - 1 {
                self.segments[i + 1].start
            } else {
                self.target.unwrap()
            };
            let segment = &mut self.segments[i];
            let length = segment.length;
            let mut distance = start.distance(&end);
            if distance < 0.0001 {
                distance = 0.0001;
            }
            let ratio = length / distance;

            let new_end = Point {
                x: (1.0 - ratio) * start.x + ratio * end.x,
                y: (1.0 - ratio) * start.y + ratio * end.y,
            };

            segment.start = start;

            last_point = new_end;
        }
        
        self.last_point = last_point;
    }
}

impl Component for FabrikComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<()>) {
        self.last_point = Point {
            x: setup_info.display_info.width() as f64 / 2.0,
            y: setup_info.display_info.height() as f64 / 2.0,
        };
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<()>) {
        let (x, y) = shared_state.mouse_info.last_mouse_pos;
        let mouse_point = Point {
            x: x as f64,
            y: y as f64,
        };

        if shared_state.mouse_pressed.right {
            if let Some(start_point) = self.currently_creating_segment.take() {
                let length = start_point.distance(&mouse_point);
                self.last_point = mouse_point;
                let new_segment = Segment {
                    start: start_point,
                    length,
                };
                self.segments.push(new_segment);
                if self.base_anchor.is_none() {
                    self.base_anchor = Some(start_point);
                }
            }

            self.currently_creating_segment = Some(mouse_point);
        }

        if shared_state.pressed_keys.inner().contains_key(&KeyCode::Esc) {
            self.currently_creating_segment = None;
        }

        if shared_state.mouse_info.left_mouse_down {
            // set target
            self.target = Some(mouse_point);
            
            self.forward_reach();
            self.backward_reach();
        }

        // if shared_state.pressed_keys.did_press_char_ignore_case('f') {
        //     self.forward_reach();
        // }
        // if shared_state.pressed_keys.did_press_char_ignore_case('b') {
        //     self.backward_reach();
        // }

        if shared_state.pressed_keys.did_press_char_ignore_case('c') {
            *self = Self::new();
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<()>, depth_base: i32) {
        let depth_segment_start = depth_base + 1;
        let depth_target = depth_base + 2;

        let mouse_point = Point {
            x: shared_state.mouse_info.last_mouse_pos.0 as f64,
            y: shared_state.mouse_info.last_mouse_pos.1 as f64,
        };

        let mut last_point = self.last_point;
        for segment in self.segments.iter().rev() {
            let start = last_point;
            let end = segment.start;

            let x_u_start = start.x.floor() as usize;
            let y_u_start = start.y.floor() as usize;
            let x_u_end = end.x.floor() as usize;
            let y_u_end = end.y.floor() as usize;

            for_coord_in_line(false, (x_u_start as i64, y_u_start as i64), (x_u_end as i64, y_u_end as i64), |x, y| {
                renderer.render_pixel(x as usize, y as usize, Pixel::new('X'), depth_base);
            });

            renderer.render_pixel(x_u_end, y_u_end, Pixel::new('S'), depth_segment_start);


            last_point = segment.start;
        }

        if let Some(start_point) = self.currently_creating_segment {
            let x_u_start = start_point.x.floor() as usize;
            let y_u_start = start_point.y.floor() as usize;
            let x_u_end = mouse_point.x.floor() as usize;
            let y_u_end = mouse_point.y.floor() as usize;

            for_coord_in_line(false, (x_u_start as i64, y_u_start as i64), (x_u_end as i64, y_u_end as i64), |x, y| {
                renderer.render_pixel(x as usize, y as usize, Pixel::new('X').with_color([255, 0, 0]), depth_base);
            });
        }

        if let Some(target) = self.target {
            let x_u = target.x.floor() as usize;
            let y_u = target.y.floor() as usize;
            renderer.render_pixel(x_u, y_u, Pixel::new('O').with_color([0, 255, 0]), depth_target);
        }
    }
}
