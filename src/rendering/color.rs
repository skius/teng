#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Color {
    #[default]
    Default,
    /// A transparent color does not overwrite the existing color
    /// If there is no other color, it will behave the same as default.
    Transparent,
    Rgb([u8; 3]),
}

impl Color {
    pub fn unwrap_or(self, other: [u8; 3]) -> [u8; 3] {
        match self {
            Color::Default => other,
            Color::Transparent => other,
            Color::Rgb(c) => c,
        }
    }

    pub fn is_solid(self) -> bool {
        match self {
            Color::Default => true,
            Color::Transparent => false,
            Color::Rgb(_) => true,
        }
    }
}
