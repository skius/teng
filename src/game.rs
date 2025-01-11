use std::io;
use std::io::{Stdout, Write};
use std::ops::{Index, IndexMut};
use crossterm::queue;
mod renderer;
mod render;

pub use renderer::*;
pub use render::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixel {
    c: char,
    color: [u8; 3],
}

impl Pixel {
    pub fn new(c: char) -> Self {
        Self { c, color: [255, 255, 255] }
    }

    pub fn with_color(self, color: [u8; 3]) -> Self {
        Self { color, ..self }
    }
}

impl Default for Pixel {
    fn default() -> Self {
        Self { c: ' ', color: [255, 255, 255]}
    }
}



