use std::io;
use std::io::{Stdout, Write};
use std::ops::{Index, IndexMut};
use crossterm::queue;
use crate::game::{Pixel, Render};

pub trait Renderer {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32);
    fn flush(&mut self) -> io::Result<()>;
}

impl<W: Write> Renderer for DisplayRenderer<W> {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
        DisplayRenderer::render_pixel(self, x, y, pixel, depth);
    }

    fn flush(&mut self) -> io::Result<()> {
        DisplayRenderer::flush(self)
    }
}

struct Display<T> {
    width: usize,
    height: usize,
    default: T,
    pixels: Vec<T>,
}

impl<T: Clone> Display<T> {
    fn new(width: usize, height: usize, default: T) -> Self {
        Self {
            width,
            height,
            default: default.clone(),
            pixels: vec![default; width * height],
        }
    }

    fn clear(&mut self) {
        for pixel in self.pixels.iter_mut() {
            *pixel = self.default.clone();
        }
    }

    fn resize_discard(&mut self, width: usize, height: usize) {
        self.pixels.resize(width * height, self.default.clone());
        self.width = width;
        self.height = height;
    }
}

impl<T> Display<T> {
    fn get_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    fn get(&self, x: usize, y: usize) -> Option<&T> {
        self.pixels.get(self.get_index(x, y))
    }

    fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        let idx = self.get_index(x, y);
        self.pixels.get_mut(idx)
    }

    fn set(&mut self, x: usize, y: usize, value: T) {
        if let Some(pixel) = self.get_mut(x, y) {
            *pixel = value;
        }
    }
}

impl<T> Index<(usize, usize)> for Display<T> {
    type Output = T;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.pixels[self.get_index(x, y)]
    }
}

impl<T> IndexMut<(usize, usize)> for Display<T> {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        let idx = self.get_index(x, y);
        &mut self.pixels[idx]
    }
}

pub struct DisplayRenderer<W: Write> {
    width: usize,
    height: usize,
    display: Display<Pixel>,
    // larger values are closer and get preference
    depth_buffer: Display<i32>,
    default_fg_color: [u8; 3],
    last_fg_color: [u8; 3],
    sink: W,
}

impl DisplayRenderer<Stdout> {
    pub fn new(width: usize, height: usize) -> Self {
        Self::new_with_sink(width, height, std::io::stdout())
    }
}

impl<W: Write> DisplayRenderer<W> {
    pub fn new_with_sink(width: usize, height: usize, sink: W) -> Self {
        Self {
            width,
            height,
            display: Display::new(width, height, Pixel::default()),
            depth_buffer: Display::new(width, height, i32::MIN),
            sink,
            default_fg_color: [255, 255, 255],
            last_fg_color: [255, 255, 255],
        }
    }

    /// Set the default fg color. Works on next flush.
    pub fn set_default_fg_color(&mut self, color: [u8; 3]) {
        self.default_fg_color = color;
    }

    /// Resizes the display and mangles the existing contents.
    pub fn resize_discard(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.display.resize_discard(width, height);
        self.depth_buffer.resize_discard(width, height);
    }

    /// Resizes the display and keeps the existing contents.
    pub fn resize_keep(&mut self, width: usize, height: usize) {
        let mut new_display = Display::new(width, height, Pixel::default());
        let mut new_depth_buffer = Display::new(width, height, i32::MIN);

        for y in 0..self.height.min(height) {
            for x in 0..self.width.min(width) {
                new_display[(x, y)] = self.display[(x, y)];
                new_depth_buffer[(x, y)] = self.depth_buffer[(x, y)];
            }
        }

        self.width = width;
        self.height = height;
        self.display = new_display;
        self.depth_buffer = new_depth_buffer;
    }

    /// Higher depths have higher priority. At same depth, first write wins.
    pub fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
        if depth > self.depth_buffer[(x, y)] {
            self.display[(x, y)] = pixel;
            self.depth_buffer[(x, y)] = depth;
        }
    }

    pub fn flush(&mut self) -> io::Result<()> {
        queue!(self.sink, crossterm::cursor::MoveTo(0, 0))?;
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel = self.display[(x, y)];
                if let Some(new_color) = pixel.color {
                    if new_color != self.last_fg_color {
                        queue!(self.sink, crossterm::style::SetForegroundColor(crossterm::style::Color::Rgb {
                            r: new_color[0],
                            g: new_color[1],
                            b: new_color[2],
                        }))?;
                        self.last_fg_color = new_color;
                    }
                }
                queue!(self.sink, crossterm::style::Print(pixel.c))?;
            }
            if y < self.height - 1 {
                queue!(self.sink, crossterm::cursor::MoveToNextLine(1))?;
            }
        }

        self.sink.flush()?;
        self.last_fg_color = self.default_fg_color;
        queue!(self.sink, crossterm::style::SetForegroundColor(crossterm::style::Color::Rgb {
            r: self.default_fg_color[0],
            g: self.default_fg_color[1],
            b: self.default_fg_color[2],
        }))?;
        self.depth_buffer.pixels.fill(i32::MIN);

        Ok(())
    }
}