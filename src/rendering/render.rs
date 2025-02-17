use crate::rendering::{display::Display, renderer::Renderer, color::Color, pixel::Pixel};
use std::fmt::Debug;
use std::io::Write;

pub trait Render {
    /// Render the object at the given position with the given depth
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32);

    /// Render the object with a given color
    fn with_color(&self, color: [u8; 3]) -> impl Render
    where
        Self: Sized,
    {
        WithColor(color, self)
    }

    /// Render the object with transparency
    fn transparent(&self) -> impl Render
    where
        Self: Sized,
    {
        WithTransparency(self)
    }

    /// Render the object with a given background color
    fn with_bg_color(&self, bg_color: [u8; 3]) -> impl Render
    where
        Self: Sized,
    {
        WithBgColor(bg_color, self)
    }
}

impl Render for &str {
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
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
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
        self.as_str().render(renderer, x, y, depth);
    }
}

impl Render for char {
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
        let pixel = Pixel::new(*self);
        renderer.render_pixel(x, y, pixel, depth);
    }
}

impl Render for Pixel {
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
        renderer.render_pixel(x, y, *self, depth);
    }
}

// TODO: refactor Sprite into separate module, remove generics? smallvec could help if we flatten the array
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
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
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

impl<T: Render> Render for &T {
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
        (*self).render(renderer, x, y, depth);
    }
}

struct WithColor<T>(pub [u8; 3], pub T);

impl<T: Render> Render for WithColor<T> {
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
        let mut adapter = ColorRendererAdapter {
            renderer,
            color: self.0,
        };
        self.1.render(&mut adapter, x, y, depth);
    }
}

struct WithBgColor<T>(pub [u8; 3], pub T);

impl<T: Render> Render for WithBgColor<T> {
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
        let mut adapter = BgColorRendererAdapter {
            renderer,
            bg_color: self.0,
        };
        self.1.render(&mut adapter, x, y, depth);
    }
}

struct WithTransparency<T>(pub T);

impl<T: Render> Render for WithTransparency<T> {
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
        let mut adapter = TransparentRendererAdapter { renderer };
        self.0.render(&mut adapter, x, y, depth);
    }
}

struct ColorRendererAdapter<'a> {
    renderer: &'a mut dyn Renderer,
    color: [u8; 3],
}

impl<'a> Renderer for ColorRendererAdapter<'a> {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
        self.renderer
            .render_pixel(x, y, pixel.with_color(self.color), depth);
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.renderer.flush()
    }
}

struct BgColorRendererAdapter<'a> {
    renderer: &'a mut dyn Renderer,
    bg_color: [u8; 3],
}

impl<'a> Renderer for BgColorRendererAdapter<'a> {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
        self.renderer
            .render_pixel(x, y, pixel.with_bg_color(self.bg_color), depth);
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.renderer.flush()
    }
}

struct TransparentRendererAdapter<'a> {
    renderer: &'a mut dyn Renderer,
}

impl<'a> Renderer for TransparentRendererAdapter<'a> {
    fn render_pixel(&mut self, x: usize, y: usize, mut pixel: Pixel, depth: i32) {
        pixel.color = Color::Transparent;
        self.renderer.render_pixel(x, y, pixel, depth);
    }

    fn flush(&mut self) -> std::io::Result<()> {
        self.renderer.flush()
    }
}

pub struct HalfBlockDisplayRender {
    width: usize,
    height: usize,
    display: Display<Color>,
}

impl HalfBlockDisplayRender {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            display: Display::new(width, height, Color::Transparent),
        }
    }

    pub fn height(&self) -> usize {
        self.height
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn set_color(&mut self, x: usize, y: usize, color: Color) {
        self.display.set(x, y, color);
    }

    pub fn resize_discard(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.display.resize_discard(width, height);
    }

    pub fn clear(&mut self) {
        self.display.clear();
    }
}

impl Render for HalfBlockDisplayRender {
    fn render(&self, renderer: &mut dyn Renderer, base_x: usize, base_y: usize, depth: i32) {
        for y_offset in 0..(self.height / 2) {
            for x_offset in 0..self.width {
                let x = base_x + x_offset;
                let y = base_y + y_offset;
                let color_top = *self.display.get(x_offset, 2 * y_offset).unwrap();
                let color_bottom = *self.display.get(x_offset, 2 * y_offset + 1).unwrap();

                match (color_top, color_bottom) {
                    // no need to draw anything
                    (Color::Transparent, Color::Transparent) => continue,
                    (Color::Transparent, color) => {
                        let mut pixel = Pixel::new('▄');
                        pixel.color = color;
                        pixel.bg_color = Color::Transparent;
                        renderer.render_pixel(x, y, pixel, depth);
                    }
                    (color, Color::Transparent) => {
                        let mut pixel = Pixel::new('▀');
                        pixel.color = color;
                        pixel.bg_color = Color::Transparent;
                        renderer.render_pixel(x, y, pixel, depth);
                    }
                    (color_top, color_bottom) => {
                        let mut pixel = Pixel::new('▀');
                        pixel.color = color_top;
                        pixel.bg_color = color_bottom;
                        renderer.render_pixel(x, y, pixel, depth);
                    }
                }
            }
        }
    }
}
use crate::rendering::pixel;
