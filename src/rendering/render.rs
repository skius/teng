//! Rendering traits and implementations for renderable objects.
//!
//! This module defines the [`Render`] trait, which enables objects to be rendered
//! to a [`Renderer`]. It also provides implementations of the `Render` trait for
//! common types like `&str`, `String`, `char`, [`Pixel`], and [`Sprite`].
//!
//! **Key Concepts:**
//!
//! *   **`Render` Trait:**  Defines the `render()` method, which takes a `Renderer`,
//!     coordinates (x, y), and a depth value as arguments.  Any type that implements
//!     `Render` can be drawn to the screen.
//! *   **Renderer Abstraction:**  The `Render` trait works with the [`Renderer`] trait,
//!     making rendering code independent of the specific rendering backend (e.g., terminal,
//!     in-memory buffer).
//! *   **Depth-based Rendering:**  The `depth` parameter in `render()` controls the rendering
//!     order, allowing you to layer objects on top of each other. Higher depth values are
//!     rendered on top.
//! *   **Trait Extensions for Styling:**  The `Render` trait provides extension methods like
//!     `with_color()`, `transparent()`, and `with_bg_color()` to easily create styled
//!     renderable objects without modifying the original object.
//!
//! **Implementations of `Render`:**
//!
//! The module provides `Render` implementations for:
//!
//! *   `&str` and `String`:  Renders text strings.
//! *   `char`: Renders a single character.
//! *   [`Pixel`]: Renders a single pixel.
//! *   [`Sprite`]: Renders a sprite (predefined grid of pixels).
//! *   `&T` where `T: Render`: Allows rendering of references to renderable objects.
//!
//! **Styling and Adapters:**
//!
//! The `with_color()`, `transparent()`, and `with_bg_color()` methods don't directly
//! modify the original object. Instead, they return *adapter* structs (`WithColor`,
//! `WithTransparency`, `WithBgColor`) that wrap the original object and apply the
//! styling during rendering.  This allows for flexible and composable styling without
//! changing the underlying data.

use crate::rendering::{color::Color, display::Display, pixel::Pixel, renderer::Renderer};
use std::fmt::Debug;

/// Trait for objects that can be rendered to a [`Renderer`].
///
/// Implement this trait for any type you want to be able to draw on the screen
/// using a `Renderer`.
pub trait Render {
    /// Renders the object to the given `Renderer` at the specified position and depth.
    ///
    /// *   `renderer`: The [`Renderer`] to use for drawing.
    /// *   `x`: The x-coordinate (column) to render at (0-indexed, from left).
    /// *   `y`: The y-coordinate (row) to render at (0-indexed, from top).
    /// *   `depth`: The rendering depth. Higher depth values are rendered on top. Should be forwarded
    ///      to the `Renderer`.
    fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32);

    /// Creates a new `Render` object with the specified foreground color applied.
    ///
    /// This is a trait extension method that returns a `WithColor` adapter.  It does
    /// not modify the original object but instead creates a new renderable object
    /// that applies the color during rendering.
    ///
    /// # Example
    ///
    /// ```rust ,no_run
    /// use teng::rendering::render::Render;
    /// use teng::rendering::renderer::Renderer;
    /// use teng::rendering::pixel::Pixel;
    ///
    /// struct MyRenderable;
    /// impl Render for MyRenderable {
    ///     fn render(&self, renderer: &mut dyn Renderer, x: usize, y: usize, depth: i32) {
    ///         renderer.render_pixel(x, y, Pixel::new('M'), depth);
    ///     }
    /// }
    ///
    /// let my_object = MyRenderable;
    /// let red_object = my_object.with_color([255, 0, 0]); // Create a red version
    ///
    /// // ... later in render function ...
    /// # let mut renderer = panic!("any renderer");
    /// red_object.render(renderer, 10, 5, 0); // Render 'M' in red
    /// ```
    fn with_color(&self, color: [u8; 3]) -> impl Render
    where
        Self: Sized,
    {
        WithColor(color, self)
    }

    // TODO: unused. Is this necessary? transparent_bg might be more useful to have?
    #[doc(hidden)]
    fn transparent(&self) -> impl Render
    where
        Self: Sized,
    {
        WithTransparency(self)
    }

    /// Creates a new `Render` object with the specified background color applied.
    ///
    /// This returns a `WithBgColor` adapter. It does not modify the original object
    /// but creates a new `Render` that applies the background color during rendering.
    ///
    /// For an example, see [`Render::with_color`].
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
/// Represents a sprite (a fixed-size grid of pixels).
///
/// Sprites are useful for pre-defining graphical elements that can be easily
/// rendered at different positions in the game.
///
/// Type Parameters:
///
/// *   `WIDTH`:  The width of the sprite (number of columns).
/// *   `HEIGHT`: The height of the sprite (number of rows).
///
/// # Example
///
/// ```rust
/// use teng::rendering::render::Sprite;
/// use teng::rendering::pixel::Pixel;
///
/// let tree_sprite: Sprite<3, 4> = Sprite::new([
///     [' ', 'A', ' '],
///     ['/', 'W', '\\'],
///     ['I', 'I', 'I'],
///     ['I', 'I', 'I'],
/// ], 1, 1); // Center at 'W' when rendering
/// ```
#[derive(Debug)]
pub struct Sprite<const WIDTH: usize, const HEIGHT: usize> {
    /// 2D array of pixels representing the sprite.
    pub pixels: [[Pixel; WIDTH]; HEIGHT],
    /// Position of the "center" of the sprite (used for rendering positioning).
    center_pos: (usize, usize),
}

impl<const WIDTH: usize, const HEIGHT: usize> Sprite<WIDTH, HEIGHT> {
    /// Gets the height of the sprite in pixels.
    pub fn height(&self) -> usize {
        HEIGHT
    }
    /// Gets the width of the sprite in pixels.
    pub fn width(&self) -> usize {
        WIDTH
    }

    /// Creates a new `Sprite` from a 2D array of characters.
    ///
    /// *   `sprite`: A 2D array (`[[char; WIDTH]; HEIGHT]`) defining the sprite's character representation.
    /// *   `offset_x`: The x-offset of the sprite's center (relative to the top-left corner).
    /// *   `offset_y`: The y-offset of the sprite's center (relative to the top-left corner).
    ///
    /// The `center_pos` is used to control the rendering position of the sprite. When you
    /// call `sprite.render(renderer, x, y, depth)`, the coordinate `(x, y)` will correspond
    /// to the sprite's `center_pos`.
    ///
    /// # Example
    ///
    /// ```rust
    /// use teng::rendering::render::Sprite;
    ///
    /// let smiley_sprite: Sprite<3, 3> = Sprite::new([
    ///     ['-', '-', '-'],
    ///     ['O', ' ', 'O'],
    ///     [' ', 'U', ' '],
    /// ], 1, 1); // Center at ' ' in the middle
    /// ```
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

/// A struct representing a display with "double the resolution" of the terminal.
///
/// This is done by only providing the capability to draw differently colored pixels to the screen.
/// Each pixel is one half of a terminal-sized pixel, and drawn via the Unicode half block characters, '▀' and '▄', and setting their respective foreground and background colors.
#[derive(Debug)]
pub struct HalfBlockDisplayRender {
    width: usize,
    height: usize,
    // min x, min y, max x, max y
    dirty_rect: Option<(usize, usize, usize, usize)>,
    display: Display<Color>,
}

impl HalfBlockDisplayRender {
    /// Creates a new `HalfBlockDisplayRender` with the specified width and height.
    ///
    /// # Arguments
    ///
    /// * `width` - The width of the display.
    /// * `height` - The height of the display. This is double the desired height in terminal pixels.
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            width,
            height,
            dirty_rect: None,
            display: Display::new(width, height, Color::Transparent),
        }
    }

    /// Returns the height of the display.
    pub fn height(&self) -> usize {
        self.height
    }

    /// Returns the width of the display.
    pub fn width(&self) -> usize {
        self.width
    }

    /// Sets the color of a specific pixel in the display. Uses the half-block coordinate space.
    pub fn set_color(&mut self, x: usize, y: usize, color: Color) {
        if x >= self.width || y >= self.height {
            return;
        }
        self.display.set(x, y, color);
        let dirty_rect = self.dirty_rect.get_or_insert((x, y, x, y));
        dirty_rect.0 = dirty_rect.0.min(x);
        dirty_rect.1 = dirty_rect.1.min(y);
        dirty_rect.2 = dirty_rect.2.max(x);
        dirty_rect.3 = dirty_rect.3.max(y);
    }

    /// Returns the color of a specific pixel in the display. Uses the half-block coordinate space.
    /// Returns `None` if the coordinates are out of bounds.
    pub fn get_color(&self, x: usize, y: usize) -> Option<Color> {
        self.display.get(x, y).copied()
    }

    /// Resizes the display to the specified width and height, discarding the current content.
    pub fn resize_discard(&mut self, width: usize, height: usize) {
        self.width = width;
        self.height = height;
        self.display.resize_discard(width, height);
    }

    /// Clears the display, setting all pixels to the transparent color.
    pub fn clear(&mut self) {
        self.display.clear();
        self.dirty_rect = None;
    }
}

impl Render for HalfBlockDisplayRender {
    fn render(&self, renderer: &mut dyn Renderer, base_x: usize, base_y: usize, depth: i32) {
        let Some((min_x, min_y, max_x, max_y)) = self.dirty_rect else {
            return;
        };

        // adjust y's to terminal space
        let min_y = min_y / 2;
        let max_y = max_y / 2;

        // for y_offset in 0..(self.height / 2) {
        // for x_offset in 0..self.width {
        for y_offset in min_y..=max_y {
            for x_offset in min_x..=max_x {
                let x = base_x + x_offset;
                let y = base_y + y_offset;
                let color_top = *self.display.get(x_offset, 2 * y_offset).unwrap();
                let color_bottom = *self.display.get(x_offset, 2 * y_offset + 1).unwrap();

                match (color_top, color_bottom) {
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
