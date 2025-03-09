//! Color representation for terminal rendering.

/// Represents colors for terminal rendering.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum Color {
    /// Use the renderer's default color.
    #[default]
    Default,
    /// A transparent color does not overwrite the existing content.
    /// If there is no other color, it will behave the same as default.
    Transparent,
    /// An RGB color.
    Rgb([u8; 3]),
}

impl Color {
    /// Unwraps the color, returning the RGB value if it is an RGB color, otherwise the passed color.
    pub fn unwrap_or(self, other: [u8; 3]) -> [u8; 3] {
        match self {
            Color::Default => other,
            Color::Transparent => other,
            Color::Rgb(c) => c,
        }
    }

    /// Returns whether the color is solid.
    /// Only transparent colors are not solid.
    pub fn is_solid(self) -> bool {
        match self {
            Color::Default => true,
            Color::Transparent => false,
            Color::Rgb(_) => true,
        }
    }
}
