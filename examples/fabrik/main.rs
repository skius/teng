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
use teng::rendering::color::Color;
use teng::rendering::render::{HalfBlockDisplayRender, Render};
use teng::util::for_coord_in_line;

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new_with_custom_buf_writer();
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
    last_point: Option<Point<f64>>,
    currently_creating_segment: Option<Point<f64>>,
    half_block_display_render: HalfBlockDisplayRender,
}

impl FabrikComponent {
    pub fn new() -> Self {
        Self {
            base_anchor: None,
            target: None,
            segments: vec![],
            last_point: None,
            currently_creating_segment: None,
            half_block_display_render: HalfBlockDisplayRender::new(0, 0),
        }
    }

    fn forward_reach(&mut self) {
        let Some(target) = self.target else {
            return;
        };
        let mut last_point = target;
        self.last_point = Some(target);
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
        let Some(base_anchor) = self.base_anchor else {
            return;
        };
        let Some(target) = self.target else {
            return;
        };
        let mut last_point = base_anchor;
        for i in 0..self.segments.len() {
            let start = last_point;
            let end = if i < self.segments.len() - 1 {
                self.segments[i + 1].start
            } else {
                target
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

        self.last_point = Some(last_point);
    }

    fn render_to_half_block_display(&mut self, mouse_point: Point<f64>) {
        self.half_block_display_render.clear();

        let segment_line_color = Color::Rgb([255, 255, 255]);
        let target_color = Color::Rgb([0, 255, 0]);
        let creating_segment_color = Color::Rgb([255, 0, 0]);
        let segment_start_color = Color::Rgb([255, 255, 0]);

        let base_anchor_color = Color::Rgb([0, 0, 255]);

        if self.segments.len() > 0 {
            let mut last_point = self.last_point.unwrap(); // must have last point
            for segment in self.segments.iter().rev() {
                let start = last_point;
                let end = segment.start;

                let x_start = start.x.floor() as i64;
                let y_start = start.y.floor() as i64;
                let x_end = end.x.floor() as i64;
                let y_end = end.y.floor() as i64;

                for_coord_in_line(true, (x_start, y_start), (x_end, y_end), |x, y| {
                    if x < 0 || y < 0 {
                        return;
                    }
                    self.half_block_display_render.set_color(x as usize, y as usize, segment_line_color);
                });

                if x_start >= 0 && y_start >= 0 {
                    self.half_block_display_render.set_color(x_start as usize, y_start as usize, segment_start_color);
                }

                last_point = segment.start;
            }
        }


        if let Some(start_point) = self.currently_creating_segment {
            let x_u_start = start_point.x.floor() as usize;
            let y_u_start = start_point.y.floor() as usize;
            let x_u_end = mouse_point.x.floor() as usize;
            let y_u_end = mouse_point.y.floor() as usize;

            for_coord_in_line(false, (x_u_start as i64, y_u_start as i64), (x_u_end as i64, y_u_end as i64), |x, y| {
                self.half_block_display_render.set_color(x as usize, y as usize, creating_segment_color);
            });
        }

        if let Some(target) = self.target {
            let x_u = target.x.floor() as usize;
            let y_u = target.y.floor() as usize;
            self.half_block_display_render.set_color(x_u, y_u, target_color);
        }

        if let Some(base_anchor) = self.base_anchor {
            let x_u = base_anchor.x.floor() as usize;
            let y_u = base_anchor.y.floor() as usize;
            self.half_block_display_render.set_color(x_u, y_u, base_anchor_color);
        }
    }
}

impl Component for FabrikComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<()>) {
        // self.last_point = Point {
        //     x: setup_info.display_info.width() as f64 / 2.0,
        //     y: setup_info.display_info.height() as f64 / 2.0,
        // };
        self.on_resize(setup_info.display_info.width(), setup_info.display_info.height(), shared_state);
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<()>) {
        self.half_block_display_render = HalfBlockDisplayRender::new(width, 2 * height);
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<()>) {
        let (x, y) = shared_state.mouse_info.last_mouse_pos;
        let mouse_point = Point {
            x: x as f64,
            y: y as f64 * 2.0,
        };

        if shared_state.mouse_pressed.right {
            // if we are not creating a segment but we do already have a last point, connect it immediately
            if let Some(last_point) = self.last_point {
                if self.currently_creating_segment.is_none() {
                    let new_segment = Segment {
                        start: last_point,
                        length: last_point.distance(&mouse_point),
                    };
                    self.segments.push(new_segment);
                    if self.base_anchor.is_none() {
                        self.base_anchor = Some(last_point);
                    }
                    self.last_point = Some(mouse_point);
                }
            }

            if let Some(start_point) = self.currently_creating_segment.take() {
                let length = start_point.distance(&mouse_point);
                self.last_point = Some(mouse_point);
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
            self.currently_creating_segment = None;

            self.forward_reach();
            self.backward_reach();
        }

        shared_state.mouse_events.for_each_linerp_only_fresh(|mi| {
            if mi.middle_mouse_down {
                self.currently_creating_segment = None;
                let mouse_point = Point {
                    x: mi.last_mouse_pos.0 as f64,
                    y: mi.last_mouse_pos.1 as f64 * 2.0,
                };

                if self.base_anchor.is_none() {
                    self.base_anchor = Some(mouse_point);
                }
                let Some(last_point) = self.last_point else {
                    self.last_point = Some(mouse_point);
                    return;
                };

                let new_segment = Segment {
                    start: last_point,
                    length: last_point.distance(&mouse_point),
                };
                self.segments.push(new_segment);
                self.last_point = Some(mouse_point);
            }
        });

        // if shared_state.pressed_keys.did_press_char_ignore_case('f') {
        //     self.forward_reach();
        // }
        // if shared_state.pressed_keys.did_press_char_ignore_case('b') {
        //     self.backward_reach();
        // }

        if shared_state.pressed_keys.did_press_char_ignore_case('c') {
            self.segments.clear();
            self.base_anchor = None;
            self.target = None;
            self.currently_creating_segment = None;
            self.last_point = None;
        }


        // render into half block display
        self.render_to_half_block_display(mouse_point);
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<()>, depth_base: i32) {
        self.half_block_display_render.render(renderer, 0, 0, depth_base);
    }
}
