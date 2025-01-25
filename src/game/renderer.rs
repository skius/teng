use crate::game::display::Display;
use crate::game::{Pixel, Render};
use crossterm::queue;
use std::io;
use std::io::{Stdout, Write};
use std::ops::{Index, IndexMut};

pub trait Renderer {
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32);
    fn flush(&mut self) -> io::Result<()>;

    fn set_default_bg_color(&mut self, color: [u8; 3]) {
        // default implementation does nothing
    }
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

    fn set_default_bg_color(&mut self, color: [u8; 3]) {
        DisplayRenderer::set_default_bg_color(self, color);
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
    default_bg_color: [u8; 3],
    last_bg_color: [u8; 3],
    sink: W,
}

impl DisplayRenderer<Stdout> {
    pub fn new(width: usize, height: usize) -> Self {
        Self::new_with_sink(width, height, std::io::stdout())
    }
}

impl<W: Write> DisplayRenderer<W> {
    pub fn new_with_sink(width: usize, height: usize, sink: W) -> Self {
        // Need to initialize the prev display with the same default as the new display,
        // because that's the pixel that gets applied on every frame's reset screen.
        let mut prev_display = Display::new(width, height, Pixel::default());
        // but for the first frame, we want this to be different from the display so that
        // we force a draw (reason being the background color could be different from the terminal's actual default)
        prev_display.fill(Pixel::default().with_color([1, 2, 3]));

        Self {
            width,
            height,
            display: Display::new(width, height, Pixel::default()),
            prev_display,
            depth_buffer: Display::new(width, height, i32::MIN),
            sink,
            default_fg_color: [255, 255, 255],
            last_fg_color: [255, 255, 255],
            default_bg_color: [0, 0, 0],
            last_bg_color: [0, 0, 0],
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

    pub fn set_default_bg_color(&mut self, color: [u8; 3]) {
        self.default_bg_color = color;
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
            self.prev_display
                .fill(Pixel::default().with_color([1, 2, 3]));
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
    pub fn render_pixel(&mut self, x: usize, y: usize, new_pixel: Pixel, new_depth: i32) {
        if x >= self.width || y >= self.height {
            return;
        }

        let old_depth = self.depth_buffer[(x, y)];
        if old_depth == i32::MIN {
            // This is the first pixel arriving for this position, so we do not want to do any blending.
            self.display[(x, y)] = new_pixel;
            self.depth_buffer[(x, y)] = new_depth;
            return;
        }
        let old_pixel = self.display[(x, y)];

        let (lower_pixel, upper_pixel) = if old_depth < new_depth {
            (old_pixel, new_pixel)
        } else {
            (new_pixel, old_pixel)
        };
        self.display[(x, y)] = upper_pixel.put_over(lower_pixel);
        self.depth_buffer[(x, y)] = old_depth.max(new_depth);
    }

    pub fn reset_screen(&mut self) {
        // needed because otherwise we get the 'solitaire bouncing cards' effect
        self.display.clear();
        self.depth_buffer.clear();
    }

    pub fn flush(&mut self) -> io::Result<()> {
        queue!(self.sink, crossterm::cursor::MoveTo(0, 0))?;

        let render_everything = self.last_bg_color != self.default_bg_color
            || self.last_fg_color != self.default_fg_color;

        self.last_fg_color = self.default_fg_color;
        self.last_bg_color = self.default_bg_color;
        queue!(
            self.sink,
            crossterm::style::SetColors(crossterm::style::Colors {
                foreground: Some(crossterm::style::Color::Rgb {
                    r: self.default_fg_color[0],
                    g: self.default_fg_color[1],
                    b: self.default_fg_color[2],
                }),
                background: Some(crossterm::style::Color::Rgb {
                    r: self.default_bg_color[0],
                    g: self.default_bg_color[1],
                    b: self.default_bg_color[2],
                }),
            }),
        )?;

        let mut curr_pos = (0, 0);
        for y in 0..self.height {
            for x in 0..self.width {
                let pixel = self.display[(x, y)];
                if !render_everything {
                    if pixel == self.prev_display[(x, y)] {
                        continue;
                    }
                    if curr_pos != (x, y) {
                        queue!(self.sink, crossterm::cursor::MoveTo(x as u16, y as u16))?;
                    }
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
                let new_bg_color = pixel.bg_color.unwrap_or(self.default_bg_color);
                if new_bg_color != self.last_bg_color {
                    queue!(
                        self.sink,
                        crossterm::style::SetBackgroundColor(crossterm::style::Color::Rgb {
                            r: new_bg_color[0],
                            g: new_bg_color[1],
                            b: new_bg_color[2],
                        })
                    )?;
                    self.last_bg_color = new_bg_color;
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

        // We're "abusing" these fields to compute on next flush whether the defaults have changed.
        // If the defaults did indeed change across calls, our 'prev_display' is essentially invalidated,
        // since pixel-equivalence does not equal display-equivalence, because two Color::Default values
        // are not display-equivalent anymore.
        // It is fine to abuse these fields, since they get set on the next flush anyway in the beginning.
        self.last_fg_color = self.default_fg_color;
        self.last_bg_color = self.default_bg_color;

        Ok(())
    }
}
