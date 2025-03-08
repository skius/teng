//! Rendering logic and the `Renderer` trait.
//!
//! This module defines the core rendering abstractions for `teng`:
//!
//! *   [`Renderer`] trait:  Defines the interface for rendering operations,
//!     abstracting away the underlying rendering backend.
//! *   [`DisplayRenderer`] struct:  A concrete implementation of the `Renderer` trait
//!     that renders to a terminal using the `crossterm` library.
//!
//! **Key Functionality of `Renderer` and `DisplayRenderer`:**
//!
//! *   **Pixel Rendering:**  `render_pixel()` function allows you to draw individual [`Pixel`]s
//!     at specific (x, y) coordinates on the display.
//! *   **Display Buffer Management:** `DisplayRenderer` internally manages two [`Display`] buffers:
//!     *   `display`:  The current frame being built.
//!     *   `prev_display`:  The previously rendered frame, used for optimization to only
//!         update changed pixels in the terminal.
//! *   **Depth Buffering:** `DisplayRenderer` uses depth buffers (`depth_buffer` and `bg_depth_buffer`)
//!     to handle overlapping pixels and ensure correct rendering order. Pixels with higher depth
//!     values are rendered on top of pixels with lower depth values.
//! *   **Color Management:**  `DisplayRenderer` manages default foreground and background colors
//!     and efficiently sets terminal colors only when they change.
//! *   **Flushing to Terminal:** `flush()` function writes the contents of the `display` buffer
//!     to the terminal, optimizing updates by only sending changes since the last frame.
//! *   **Resizing:**  `resize_discard()` and `resize_keep()` functions allow you to resize the
//!     rendering area, either discarding or preserving existing content.

use crate::rendering::{display::Display, pixel::Pixel};
use crossterm::queue;
use std::io;
use std::io::Write;

/// Trait for rendering operations.
///
/// Implementors of this trait provide methods for rendering pixels and flushing
/// the rendered output to a target (e.g., a terminal).
pub trait Renderer {
    /// Renders a single pixel at the specified coordinates with the given depth.
    ///
    /// Higher depth values are rendered on top of lower depth values.  If two pixels
    /// are rendered at the same depth, the first one rendered will take precedence.
    ///
    /// Coordinates are 0-indexed, starting from the top-left corner of the display.
    // TODO: Switch API from usize to i64 to allow easier partial out of bounds handling?
    fn render_pixel(&mut self, x: usize, y: usize, pixel: Pixel, depth: i32);

    /// Flushes the rendered output to the target.
    ///
    /// This function should be called after rendering all pixels for a frame to
    /// actually display the changes on the terminal (or other rendering target).
    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }

    /// Sets the default background color for subsequent rendering operations.
    ///
    /// This default color will be used when rendering pixels without an explicit
    /// background color set. It applies to the next `flush()` operation.
    fn set_default_bg_color(&mut self, color: [u8; 3]) {
        // default implementation does nothing
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

/// Concrete `Renderer` implementation that renders to a terminal using `crossterm`.
///
/// `DisplayRenderer` manages two display buffers (`display` and `prev_display`) and
/// uses depth buffers to handle pixel overlapping and efficient terminal updates.
pub struct DisplayRenderer<W: Write> {
    width: usize,
    height: usize,
    /// The current frame being built.
    display: Display<Pixel>,
    /// The previously rendered frame, used for optimization to only update changed pixels.
    prev_display: Display<Pixel>,
    /// Depth buffer for pixel rendering, larger values are rendered on top.
    depth_buffer: Display<i32>,
    /// Depth buffer for background color rendering. Only solid background colors are tracked.
    bg_depth_buffer: Display<i32>,
    default_fg_color: [u8; 3],
    last_fg_color: [u8; 3],
    default_bg_color: [u8; 3],
    last_bg_color: [u8; 3],
    sink: W,
}

impl<W: Write> DisplayRenderer<W> {
    /// Creates a new `DisplayRenderer` with a custom output sink.
    ///
    /// Allows rendering to targets such as `stdout`, files, or in-memory buffers.
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
            bg_depth_buffer: Display::new(width, height, i32::MIN),
            sink,
            default_fg_color: [255, 255, 255],
            last_fg_color: [255, 255, 255],
            default_bg_color: [0, 0, 0],
            last_bg_color: [0, 0, 0],
        }
    }

    /// Gets the width of the display in characters.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Gets the height of the display in characters.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Set the default fg color. Works on next flush.
    pub fn set_default_fg_color(&mut self, color: [u8; 3]) {
        self.default_fg_color = color;
    }

    /// Set the default bg color. Works on next flush.
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
        self.bg_depth_buffer.resize_discard(width, height);
    }

    /// Resizes the display and keeps the existing contents.
    pub fn resize_keep(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.display.resize_keep(width, height);
        self.prev_display.resize_keep(width, height);
        self.depth_buffer.resize_keep(width, height);
        self.bg_depth_buffer.resize_keep(width, height);
    }

    /// Renders a single pixel to the display buffer at the specified coordinates and depth.
    ///
    /// This function updates the internal `display` and `depth_buffer` based on the
    /// pixel's depth and color information. It does not directly write to the terminal;
    /// call `flush()` to perform the actual terminal output.
    ///
    /// Higher depths have higher priority. At same depth, the first call wins.
    pub fn render_pixel(&mut self, x: usize, y: usize, new_pixel: Pixel, new_depth: i32) {
        if x >= self.width || y >= self.height {
            return;
        }

        // match &mut new_pixel.color {
        //     Color::Rgb(arr) => {
        //         // add some random noise to the new pixel color and bg color
        //         let d = rand::thread_rng().gen_range(-4..=4);
        //         arr[0] = (arr[0] as i16 + d).max(0).min(255) as u8;
        //         arr[1] = (arr[1] as i16 + d).max(0).min(255) as u8;
        //         arr[2] = (arr[2] as i16 + d).max(0).min(255) as u8;
        //     }
        //     _ => {}
        // }

        let old_depth = self.depth_buffer[(x, y)];
        if old_depth == i32::MIN {
            // This is the first pixel arriving for this position, so we do not want to do any blending.
            self.display[(x, y)] = new_pixel;
            self.depth_buffer[(x, y)] = new_depth;
            if new_pixel.bg_color.is_solid() {
                self.bg_depth_buffer[(x, y)] = new_depth;
            }
            return;
        }
        let old_pixel = self.display[(x, y)];

        // for the background, keep track of the 'depth' that was associated with the first non-transparent pixel
        let old_bg_depth = self.bg_depth_buffer[(x, y)];
        let mut new_bg_color = old_pixel.bg_color;
        if old_bg_depth < new_depth && new_pixel.bg_color.is_solid() {
            self.bg_depth_buffer[(x, y)] = new_depth;
            new_bg_color = new_pixel.bg_color;
        }

        let (lower_pixel, upper_pixel) = if old_depth < new_depth {
            (old_pixel, new_pixel)
        } else {
            (new_pixel, old_pixel)
        };
        // TODO: "put_over" can be reworked; we're replacing bg_color anyway, so it only really
        // needs to handle fg color and chars.
        let mut created_pixel = upper_pixel.put_over(lower_pixel);
        created_pixel.bg_color = new_bg_color;
        self.display[(x, y)] = created_pixel;
        self.depth_buffer[(x, y)] = old_depth.max(new_depth);
    }

    /// Resets the screen to a blank state.
    ///
    /// Clears both the `display` buffer and the depth buffers, effectively preparing
    /// for a fresh frame of rendering.
    pub fn reset_screen(&mut self) {
        // needed because otherwise we get the 'solitaire bouncing cards' effect
        self.display.clear();
        self.depth_buffer.clear();
        self.bg_depth_buffer.clear();
    }

    /// Flushes the contents of the display buffer to the terminal.
    ///
    /// This function iterates through the `display` buffer and writes the changes
    /// to the terminal output using `crossterm`. It optimizes updates by only
    /// redrawing pixels that have changed since the last `flush()`.
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
                let mut new_color_change = None;
                let mut new_bg_color_change = None;
                let new_color = pixel.color.unwrap_or(self.default_fg_color);
                if new_color != self.last_fg_color {
                    new_color_change = Some(crossterm::style::Color::Rgb {
                        r: new_color[0],
                        g: new_color[1],
                        b: new_color[2],
                    });
                    self.last_fg_color = new_color;
                }
                let new_bg_color = pixel.bg_color.unwrap_or(self.default_bg_color);
                if new_bg_color != self.last_bg_color {
                    new_bg_color_change = Some(crossterm::style::Color::Rgb {
                        r: new_bg_color[0],
                        g: new_bg_color[1],
                        b: new_bg_color[2],
                    });
                    self.last_bg_color = new_bg_color;
                }
                // optimize color changes by combining into a single SetColors. If both are None, this is a noop.
                queue!(
                    self.sink,
                    crossterm::style::SetColors(crossterm::style::Colors {
                        foreground: new_color_change,
                        background: new_bg_color_change,
                    })
                )?;
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
