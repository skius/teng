use crate::rendering::render::Render;
use crate::Renderer;
use std::fmt::Debug;
use std::time::{Duration, Instant};

pub trait Animation {
    /// Returns true if the animation is done.
    fn render(
        &self,
        screen_base: (usize, usize),
        current_time: Instant,
        renderer: &mut dyn Renderer,
    ) -> bool;
}

pub struct CharAnimationSequence {
    pub sequence: Vec<char>,
    pub start_time: Instant,
    pub time_per_frame: Duration,
}

impl Animation for CharAnimationSequence {
    fn render(
        &self,
        (x, y): (usize, usize),
        current_time: Instant,
        renderer: &mut dyn Renderer,
    ) -> bool {
        let elapsed = current_time - self.start_time;
        let frame = (elapsed.as_secs_f64() / self.time_per_frame.as_secs_f64()) as usize;
        if frame >= self.sequence.len() {
            return true;
        }
        self.sequence[frame].render(renderer, x, y, i32::MAX);
        false
    }
}

pub struct RenderAnimationSequence {
    pub sequence: Vec<Box<dyn Render>>,
    pub start_time: Instant,
    pub time_per_frame: Duration,
}

impl Animation for RenderAnimationSequence {
    fn render(
        &self,
        (x, y): (usize, usize),
        current_time: Instant,
        renderer: &mut dyn Renderer,
    ) -> bool {
        let elapsed = current_time - self.start_time;
        let frame = (elapsed.as_secs_f64() / self.time_per_frame.as_secs_f64()) as usize;
        if frame >= self.sequence.len() {
            return true;
        }
        self.sequence[frame].render(renderer, x, y, i32::MAX);
        false
    }
}
