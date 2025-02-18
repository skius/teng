//! Pixel representation for terminal rendering.
//!
//! This module defines the `Pixel` struct, which represents a single character
//! and its associated color and background color for terminal-based rendering.
//!
//! The `Pixel` struct is the fundamental unit for building up the display in `teng`.
//! It allows you to control the character displayed at each position on the terminal,
//! as well as its foreground and background colors.

use crate::rendering::color::Color;

/// Represents a single pixel (character) for terminal rendering.
///
/// A `Pixel` consists of:
///
/// *   `c`: The character to be displayed.
/// *   `color`: The foreground color of the character (using [`Color`]).
/// *   `bg_color`: The background color of the character (using [`Color`]).
///
/// # Defaults
///
/// By default, a `Pixel` is created with:
///
/// *   Character: ' ' (space)
/// *   Foreground Color: `Color::Default` (renderer's default foreground)
/// *   Background Color: `Color::Transparent` (no background color, lets below color show through)
///
/// # Example
///
/// ```rust
/// use teng::rendering::pixel::Pixel;
/// use teng::rendering::color::Color;
///
/// let red_pixel = Pixel::new('█').with_color([255, 0, 0]); // Red block
/// let blue_on_yellow = Pixel::new('X')
///     .with_color([0, 0, 255])        // Blue foreground
///     .with_bg_color([255, 255, 0]); // Yellow background
///
/// println!("Red Pixel: {:?}", red_pixel);
/// println!("Blue on Yellow: {:?}", blue_on_yellow);
/// ```
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixel {
    /// The character to be displayed.
    pub c: char,
    /// The foreground color of the pixel.
    pub color: Color,
    /// The background color of the pixel.
    pub bg_color: Color,
}

impl Pixel {
    /// Creates a new `Pixel` with the given character and default colors.
    ///
    /// The foreground color will be `Color::Default`, and the background color
    /// will be `Color::Transparent`.
    pub fn new(c: char) -> Self {
        Self {
            c,
            color: Color::Default,
            bg_color: Color::Transparent,
        }
    }

    /// Creates a new `Pixel` with a transparent character and colors.
    /// 
    /// This is useful e.g. for creating a default "passthrough" pixel that doesn't overwrite existing content unless it's color is changed.
    pub fn transparent() -> Self {
        Self {
            c: ' ',
            color: Color::Transparent,
            bg_color: Color::Transparent,
        }
    }

    /// Creates a new `Pixel` with the same character and background color as `self`, but with a new foreground color.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use teng::rendering::pixel::Pixel;
    ///
    /// let pixel = Pixel::new('o');
    /// let red_pixel = pixel.with_color([255, 0, 0]); // red 'o' with default background
    /// ```
    pub fn with_color(self, color: [u8; 3]) -> Self {
        Self {
            color: Color::Rgb(color),
            c: self.c,
            bg_color: self.bg_color,
        }
    }

    /// Creates a new `Pixel` with the same character and foreground color as `self`, but with a new background color.
    /// 
    /// # Example
    /// 
    /// ```rust
    /// use teng::rendering::pixel::Pixel;
    /// 
    /// let pixel = Pixel::new('o');
    /// let green_bg_pixel = pixel.with_bg_color([0, 255, 0]); // 'o' with green background
    /// ```
    pub fn with_bg_color(self, bg_color: [u8; 3]) -> Self {
        Self {
            bg_color: Color::Rgb(bg_color),
            c: self.c,
            color: self.color,
        }
    }

    
    /// Overlays `self` over `other`, taking into account transparencies, and returns the result.
    /// 
    /// # Example
    ///
    /// ```rust
    /// use teng::rendering::pixel::Pixel;
    ///
    /// let base_pixel = Pixel::new('.'); // Top pixel, transparent background
    /// let overlay_pixel = Pixel::new('X').with_color([0, 255, 0]).with_bg_color([255, 0, 0]); // Bottom pixel, green 'X' with red background
    ///
    /// let combined_pixel = base_pixel.put_over(overlay_pixel);
    /// // combined_pixel will now be a default-colored '.' with a red background
    /// assert_eq!(combined_pixel, Pixel::new('.').with_bg_color([255, 0, 0]));
    /// ```
    pub fn put_over(self, other: Pixel) -> Self {
        // works with priorities: transparent < default < color
        // and other < self

        let mut new_pixel = self;
        if new_pixel.color == Color::Transparent {
            new_pixel.color = other.color;
            new_pixel.c = other.c;
        }
        if new_pixel.bg_color == Color::Transparent {
            new_pixel.bg_color = other.bg_color;
        }
        new_pixel
    }
}

impl Default for Pixel {
    fn default() -> Self {
        Self {
            c: ' ',
            color: Color::Default,
            bg_color: Color::Default,
        }
    }
}
