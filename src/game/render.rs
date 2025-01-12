use std::fmt::Display;
use std::io::Write;
use crate::game::{Pixel, renderer::Renderer};

pub trait Render {
    /// Render the object at the given position with the given depth
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32);
}

impl Render for &str {
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32) {
        for (i, c) in self.chars().enumerate() {
            let pixel = Pixel::new(c);
            renderer.render_pixel(x + i, y, pixel, depth);
        }
    }
}

impl Render for String {
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32) {
        self.as_str().render(renderer, x, y, depth);
    }
}

impl Render for char {
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32) {
        let pixel = Pixel::new(*self);
        renderer.render_pixel(x, y, pixel, depth);
    }
}

impl Render for Pixel {
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32) {
        renderer.render_pixel(x, y, *self, depth);
    }
}

pub struct Sprite<const WIDTH: usize, const HEIGHT: usize>(pub [[Pixel; WIDTH]; HEIGHT]);

impl<const WIDTH: usize, const HEIGHT: usize>  Sprite<WIDTH, HEIGHT> {
    pub fn height(&self) -> usize {
        HEIGHT
    }
    
    pub fn width(&self) -> usize {
        WIDTH
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> Render for Sprite<WIDTH, HEIGHT> {
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32) {
        for (i, row) in self.0.iter().enumerate() {
            for (j, pixel) in row.iter().enumerate() {
                renderer.render_pixel(x + j, y + i, *pixel, depth);
            }
        }
    }
}

pub struct WithColor<T>(pub [u8; 3], pub T);

impl<T: Render> Render for WithColor<T> {
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32) {
        let mut adapter = ColorRendererAdapter {
            renderer,
            color: self.0,
        };
        self.1.render(&mut adapter, x, y, depth);
    }
}

struct ColorRendererAdapter<'a, R> {
    renderer: &'a mut R,
    color: [u8; 3],
}

impl<'a, R: Renderer> Renderer for ColorRendererAdapter<'a, R> {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
        self.renderer.render_pixel(x, y, pixel.with_color(self.color), depth);
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.renderer.flush()
    }
}
