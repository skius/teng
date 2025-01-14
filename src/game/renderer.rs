use crate::game::display::Display;
use crate::game::{Pixel, Render};
use crossterm::queue;
use std::io;
use std::io::{Stdout, Write};
use std::ops::{Index, IndexMut};

pub trait Renderer {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32);
    fn flush(&mut self) -> io::Result<()>;
}

impl Renderer for &mut dyn Renderer {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
        Renderer::render_pixel(*self, x, y, pixel, depth);
    }

    fn flush(&mut self) -> io::Result<()> {
        Renderer::flush(*self)
    }
}

impl<W: Write> Renderer for DisplayRenderer<W> {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
        DisplayRenderer::render_pixel(self, x, y, pixel, depth);
    }

    fn flush(&mut self) -> io::Result<()> {
        DisplayRenderer::flush(self)
    }
}

pub struct DisplayRenderer<W: Write> {
    width: usize,
    height: usize,
    display: Display<Pixel>,
    prev_display: Display<Pixel>,
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
            prev_display: Display::new(width, height, Pixel::default()),
            depth_buffer: Display::new(width, height, i32::MIN),
            sink,
            default_fg_color: [255, 255, 255],
            last_fg_color: [255, 255, 255],
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }

    pub fn height(&self) -> usize {
        self.height
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
        if self.prev_display.height() > height {
            self.prev_display.resize_discard(width, height);
            // need to fill with something that is most likely different from the display, because
            // sometimes when resizing height the actual printed chars get mangled
            self.prev_display.fill(Pixel::default().with_color([1, 2, 3]));
        } else {
            // need to keep this because we're not rewriting it in this coming frame.
            self.prev_display.resize_keep(width, height);
        }
        self.depth_buffer.resize_discard(width, height);
    }

    /// Resizes the display and keeps the existing contents.
    pub fn resize_keep(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.display.resize_keep(width, height);
        self.prev_display.resize_keep(width, height);
        self.depth_buffer.resize_keep(width, height);
    }

    /// Higher depths have higher priority. At same depth, first write wins.
    pub fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32) {
        if x >= self.width || y >= self.height {
            return;
        }
        if depth > self.depth_buffer[(x, y)] {
            self.display[(x, y)] = pixel;
            self.depth_buffer[(x, y)] = depth;
        }
    }

    pub fn reset_screen(&mut self) {
        self.display.clear();
        self.depth_buffer.clear();
    }

    pub fn flush(&mut self) -> io::Result<()> {
        queue!(self.sink, crossterm::cursor::MoveTo(0, 0))?;
        let mut curr_pos = (0, 0);
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel = self.display[(x, y)];
                if pixel == self.prev_display[(x, y)] {
                    continue;
                }
                if curr_pos != (x, y) {
                    queue!(self.sink, crossterm::cursor::MoveTo(x as u16, y as u16))?;
                }
                let new_color = pixel.color.unwrap_or(self.default_fg_color);
                if new_color != self.last_fg_color {
                    queue!(
                        self.sink,
                        crossterm::style::SetForegroundColor(crossterm::style::Color::Rgb {
                            r: new_color[0],
                            g: new_color[1],
                            b: new_color[2],
                        })
                    )?;
                    self.last_fg_color = new_color;
                }
                queue!(self.sink, crossterm::style::Print(pixel.c))?;
                curr_pos = (x, y);
            }
            if y < self.height - 1 {
                queue!(self.sink, crossterm::cursor::MoveToNextLine(1))?;
                curr_pos = (0, y + 1);
            }
        }

        self.sink.flush()?;
        std::mem::swap(&mut self.display, &mut self.prev_display);
        // self.display.clear();
        self.last_fg_color = self.default_fg_color;
        queue!(
            self.sink,
            crossterm::style::SetForegroundColor(crossterm::style::Color::Rgb {
                r: self.default_fg_color[0],
                g: self.default_fg_color[1],
                b: self.default_fg_color[2],
            })
        )?;
        self.depth_buffer.clear();

        Ok(())
    }
}
