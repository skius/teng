use crate::{Render, Renderer};
use std::fmt::Debug;
use std::time::{Duration, Instant};

pub trait Animation: Debug {
    /// Returns true if the animation is done.
    fn render(
        &self,
        screen_base: (usize, usize),
        current_time: Instant,
        renderer: &mut dyn Renderer,
    ) -> bool;
}

#[derive(Debug)]
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
        mut renderer: &mut dyn Renderer,
    ) -> bool {
        let elapsed = current_time - self.start_time;
        let frame = (elapsed.as_secs_f64() / self.time_per_frame.as_secs_f64()) as usize;
        if frame >= self.sequence.len() {
            return true;
        }
        self.sequence[frame].render(&mut renderer, x, y, i32::MAX);
        false
    }
}
