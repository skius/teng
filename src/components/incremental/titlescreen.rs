use crate::components::{KeyPressRecorderComponent, MouseTrackerComponent};
use crate::rendering::pixel::Pixel;
use crate::rendering::render::{Render, Sprite};
use crate::{BreakingAction, Component, Renderer, SetupInfo, SharedState, UpdateInfo};
use crossterm::event::Event;
use std::any::{Any, TypeId};
use std::collections::HashSet;
use std::time::Instant;

pub struct TitleScreenComponent {
    width: usize,
    height: usize,
    final_text: String,
    current_prefix_length: usize,
    next_char_time: Instant,
    finished: bool,
    sprite_positions: Vec<(f64, f64)>,
}

impl TitleScreenComponent {
    pub fn new() -> Self {
        Self {
            width: 0,
            height: 0,
            final_text: "Terminally Ill".to_string(),
            current_prefix_length: 0,
            next_char_time: Instant::now(),
            finished: false,
            sprite_positions: vec![],
        }
    }

    fn exit(&mut self, shared_state: &mut SharedState) {
        self.finished = true;
        shared_state.remove_components.insert(TypeId::of::<Self>());
        shared_state.whitelisted_components = None;
    }
}

impl Component for TitleScreenComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        self.width = setup_info.width;
        self.height = setup_info.height;
        shared_state.whitelisted_components = Some(HashSet::from([
            TypeId::of::<Self>(),
            TypeId::of::<KeyPressRecorderComponent>(),
            TypeId::of::<MouseTrackerComponent>(),
        ]));
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState) {
        self.width = width;
        self.height = height;
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        // simulate typing out the text
        if self.current_prefix_length < self.final_text.len() {
            if update_info.current_time >= self.next_char_time {
                if self.final_text.as_bytes()[self.current_prefix_length] != b' ' {
                    print!("\x1b[1;1;1,~");
                }
                self.current_prefix_length += 1;
                let mean = 100.0;
                let std_dev = 80.0;
                let offset = rand::random::<f64>() * std_dev + mean;
                self.next_char_time =
                    update_info.current_time + std::time::Duration::from_millis(offset as u64);
                if self.current_prefix_length == self.final_text.len() {
                    self.next_char_time =
                        update_info.current_time + std::time::Duration::from_millis(300);
                }
            }
        }

        if self.current_prefix_length == self.final_text.len()
            && update_info.current_time > self.next_char_time
        {
            // spawn some sprites
            if self.sprite_positions.len() < 50 {
                let x = rand::random::<f64>() * self.width as f64;
                let y = rand::random::<f64>() * self.height as f64;
                self.sprite_positions.push((x, -y));
            }

            // move the sprites
            for (x, y) in &mut self.sprite_positions {
                *y += 30.0 * update_info.dt;
            }

            // remove sprites that are off screen
            self.sprite_positions
                .retain(|(_, y)| *y < self.height as f64);
        }

        if shared_state.pressed_keys.inner.len() > 0 || shared_state.mouse_pressed.any() {
            self.exit(shared_state);
        }
    }

    fn is_active(&self, shared_state: &SharedState) -> bool {
        !self.finished
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX - 3;
        let depth_text = i32::MAX - 2;
        let depth_sprites = i32::MAX - 1;
        // render a black screen over everything
        for x in 0..self.width {
            for y in 0..self.height {
                renderer.render_pixel(x, y, Pixel::new(' ').with_bg_color([0, 0, 0]), depth_base);
            }
        }

        let center_y = self.height / 2;
        let center_x = self.width / 2;
        let text_x_start = center_x - self.final_text.len() / 2;
        let mut curr_y = center_y;
        (&self.final_text[..self.current_prefix_length]).render(
            renderer,
            text_x_start,
            curr_y,
            depth_text,
        );
        curr_y += 2;

        if self.current_prefix_length == self.final_text.len()
            && Instant::now() > self.next_char_time
        {
            let credit_text_grey = "A game by ";
            let credit_text_white = "skius";
            let all_credit_len = credit_text_grey.len() + credit_text_white.len();
            let credit_text_grey_x_start = center_x - all_credit_len / 2;
            let credit_text_white_x_start = credit_text_grey_x_start + credit_text_grey.len();
            credit_text_grey.with_color([160; 3]).render(
                renderer,
                credit_text_grey_x_start,
                curr_y,
                depth_text,
            );
            credit_text_white.render(renderer, credit_text_white_x_start, curr_y, depth_text);
            curr_y += 3;

            // invert colors every 500ms
            let continue_text = "Press any key to continue";
            let text_x_start = center_x - continue_text.len() / 2;
            let elapsed = self.next_char_time.elapsed().as_millis();
            if elapsed % 1300 < 750 {
                continue_text.render(renderer, text_x_start, curr_y, depth_text);
            } else {
                continue_text
                    .with_color([0, 0, 0])
                    .with_bg_color([255, 255, 255])
                    .render(renderer, text_x_start, curr_y, depth_text);
            }
        }

        for (x, y) in &self.sprite_positions {
            let x = *x as usize;
            if *y < 0.0 {
                continue;
            }
            let y = *y as usize;
            let sprite = Sprite::new([['▁', '▄', '▁'], ['▗', '▀', '▖']], 1, 1);
            sprite.render(renderer, x, y, depth_sprites);
        }
    }
}
