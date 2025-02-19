//! 2D display buffer for terminal rendering.
//!
//! This module defines the `Display` struct, which acts as a 2D buffer to hold
//! `Pixel` data before it is rendered to the terminal.
//!
//! The `Display` is essentially a grid of [`Pixel`]s with a fixed width and height.
//! It provides methods for:
//!
//! *   Creating and resizing the display buffer.
//! *   Clearing and filling the buffer with a default pixel.
//! *   Accessing and modifying individual pixels at specific coordinates.
//! *   Iterating over pixels in the display.

use std::fmt;
use std::fmt::{Debug, Formatter};
use std::ops::{Index, IndexMut};

/// A 2D display buffer for storing pixels.
///
/// The `Display` struct represents a grid of pixels with a fixed `width` and `height`.
/// It is used to build up a frame before rendering it to the terminal.
/// While intended for [`Pixel`]s, it is generic over the type of 'pixel' it contains.
///
/// # Example
///
/// ```rust
/// use teng::rendering::display::Display;
/// use teng::rendering::pixel::Pixel;
///
/// // Create a 10x5 display, initialized with default pixels
/// let mut display: Display<Pixel> = Display::new(10, 5, Pixel::default());
///
/// // Set a pixel at coordinates (2, 3)
/// display[(2, 3)] = Pixel::new('X');
///
/// // Get the pixel at (5, 1)
/// let pixel_at_5_1 = display[(5, 1)];
///
/// println!("Display dimensions: {}x{}", display.width(), display.height());
/// println!("Pixel at (2, 3): {:?}", display[(2, 3)]); // prints "Pixel('X')"
/// println!("Pixel at (5, 1): {:?}", pixel_at_5_1); // prints "Pixel(' ')" (the default)
/// ```
pub struct Display<T> {
    width: usize,
    height: usize,
    default: T,
    pixels: Vec<T>,
}

impl<T: Debug> Debug for Display<T> {
    fn fmt(&self, f: &mut Formatter) -> fmt::Result {
        write!(
            f,
            "Display {{ width: {}, height: {}, pixels: {:?} }}",
            self.width, self.height, self.pixels
        )
    }
}

impl<T: Clone> Clone for Display<T> {
    fn clone(&self) -> Self {
        Self {
            width: self.width,
            height: self.height,
            default: self.default.clone(),
            pixels: self.pixels.clone(),
        }
    }
}

impl<T: Clone> Display<T> {
    /// Creates a new `Display` with the given width and height.
    ///
    /// All pixels in the display are initially initialized with the `default` pixel value.
    ///
    /// ```rust
    /// use teng::rendering::display::Display;
    /// use teng::rendering::pixel::Pixel;
    ///
    /// let display: Display<Pixel> = Display::new(20, 10, Pixel::transparent());
    /// assert_eq!(display.width(), 20);
    /// assert_eq!(display.height(), 10);
    /// assert_eq!(display[(0, 0)], Pixel::transparent());
    /// ```
    pub fn new(width: usize, height: usize, default: T) -> Self {
        Self {
            width,
            height,
            default: default.clone(),
            pixels: vec![default; width * height],
        }
    }

    /// Clears the display, filling it with the default pixel value.
    ///
    /// This resets the entire display to its initial state, effectively erasing
    /// any previously rendered content.
    ///
    /// ```rust
    /// use teng::rendering::display::Display;
    /// use teng::rendering::pixel::Pixel;
    ///
    /// let mut display: Display<Pixel> = Display::new(8, 8, Pixel::default());
    /// // ... render some content to 'display' ...
    /// display.clear(); // Erase everything
    /// // 'display' is now filled with default pixels again.
    /// ```
    pub fn clear(&mut self) {
        for pixel in self.pixels.iter_mut() {
            *pixel = self.default.clone();
        }
    }

    /// Fills the entire display with a given pixel value.
    ///
    /// This is similar to `clear()` but allows you to fill with a specific `value`
    /// instead of the default pixel.
    ///
    /// ```rust
    /// use teng::rendering::display::Display;
    /// use teng::rendering::pixel::Pixel;
    ///
    /// let mut display: Display<Pixel> = Display::new(15, 5, Pixel::default());
    /// display.fill(Pixel::new('#').with_color([200, 200, 200])); // Fill with gray '#'
    /// ```
    pub fn fill(&mut self, value: T) {
        for pixel in self.pixels.iter_mut() {
            *pixel = value.clone();
        }
    }

    /// Resizes the display, mangling any existing pixel data.
    ///
    /// The display buffer is resized to the new `width` and `height`, and may contain arbitrary (safe) data.
    /// Use this if you know you will overwrite the entire display anyway.
    pub fn resize_discard(&mut self, width: usize, height: usize) {
        self.pixels.resize(width * height, self.default.clone());
        self.width = width;
        self.height = height;
    }

    /// Resizes the display, keeping existing pixel data where possible.
    ///
    /// If the new dimensions are larger than the current dimensions, the existing pixels are kept and the new pixels are filled with the default value.
    pub fn resize_keep(&mut self, width: usize, height: usize) {
        let mut new_pixels = vec![self.default.clone(); width * height];
        let min_width = self.width.min(width);
        for y in 0..self.height.min(height) {
            let src_start = y * self.width;
            let dst_start = y * width;
            let src_end = src_start + min_width;
            let dst_end = dst_start + min_width;
            new_pixels[dst_start..dst_end].clone_from_slice(&self.pixels[src_start..src_end]);
        }
        self.pixels = new_pixels;
        self.width = width;
        self.height = height;
    }
}

impl<T> Display<T> {
    /// Calculates the linear index for a given (x, y) coordinate.
    #[inline]
    fn get_index(&self, x: usize, y: usize) -> usize {
        y * self.width + x
    }

    /// Gets the height of the display (number of rows).
    pub fn height(&self) -> usize {
        self.height
    }

    /// Gets the width of the display (number of columns).
    pub fn width(&self) -> usize {
        self.width
    }

    /// Gets a reference to the pixel at the given (x, y) coordinates.
    ///
    /// Returns `None` if the coordinates are out of bounds.
    pub fn get(&self, x: usize, y: usize) -> Option<&T> {
        self.pixels.get(self.get_index(x, y))
    }

    /// Gets a mutable reference to the pixel at the given (x, y) coordinates.
    ///
    /// Returns `None` if the coordinates are out of bounds.
    pub fn get_mut(&mut self, x: usize, y: usize) -> Option<&mut T> {
        let idx = self.get_index(x, y);
        self.pixels.get_mut(idx)
    }

    /// Sets the pixel at the given (x, y) coordinates to the specified value if the coordinates are in bounds.
    pub fn set(&mut self, x: usize, y: usize, value: T) {
        if let Some(pixel) = self.get_mut(x, y) {
            *pixel = value;
        }
    }

    /// Returns an iterator over the pixels in the display.
    pub fn iter(&self) -> impl Iterator<Item = (usize, usize, &T)> {
        self.pixels.iter().enumerate().map(|(idx, pixel)| {
            let x = idx % self.width;
            let y = idx / self.width;
            (x, y, pixel)
        })
    }

    /// Returns a mutable iterator over the pixels in the display.
    pub fn iter_mut(&mut self) -> impl Iterator<Item = (usize, usize, &mut T)> {
        self.pixels.iter_mut().enumerate().map(|(idx, pixel)| {
            let x = idx % self.width;
            let y = idx / self.width;
            (x, y, pixel)
        })
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
