use crate::game::{renderer::Renderer, Pixel};
use std::fmt::Display;
use std::io::Write;

pub trait Render {
    /// Render the object at the given position with the given depth
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32);
}

impl Render for &str {
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32) {
        let mut y = y;
        let mut draw_x = x;
        for (i, c) in self.chars().enumerate() {
            if c == '\n' {
                y += 1;
                draw_x = x;
                continue;
            }
            let pixel = Pixel::new(c);
            renderer.render_pixel(draw_x, y, pixel, depth);
            draw_x += 1;
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

#[derive(Debug)]
pub struct Sprite<const WIDTH: usize, const HEIGHT: usize> {
    pub pixels: [[Pixel; WIDTH]; HEIGHT],
    center_pos: (usize, usize),
}

impl<const WIDTH: usize, const HEIGHT: usize> Sprite<WIDTH, HEIGHT> {
    pub fn height(&self) -> usize {
        HEIGHT
    }

    pub fn width(&self) -> usize {
        WIDTH
    }

    pub fn new(sprite: [[char; WIDTH]; HEIGHT], offset_x: usize, offset_y: usize) -> Self {
        let pixels = sprite.map(|row| row.map(|c| Pixel::new(c)));
        Self {
            pixels,
            center_pos: (offset_x, offset_y),
        }
    }
}

impl<const WIDTH: usize, const HEIGHT: usize> Render for Sprite<WIDTH, HEIGHT> {
    fn render<R: Renderer>(&self, renderer: &mut R, x: usize, y: usize, depth: i32) {
        let (center_x, center_y) = self.center_pos;
        let x = x as i32 - center_x as i32;
        let y = y as i32 - center_y as i32;

        for (i, row) in self.pixels.iter().enumerate() {
            for (j, pixel) in row.iter().enumerate() {
                let render_x = x + j as i32;
                let render_y = y + i as i32;
                if render_x < 0 || render_y < 0 {
                    continue;
                }
                renderer.render_pixel(render_x as usize, render_y as usize, *pixel, depth);
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
        self.renderer
            .render_pixel(x, y, pixel.with_color(self.color), depth);
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.renderer.flush()
    }
}
