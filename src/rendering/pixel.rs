use crate::rendering::color::Color;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixel {
    pub c: char,
    pub color: Color,
    pub bg_color: Color,
}

impl Pixel {
    pub fn new(c: char) -> Self {
        Self {
            c,
            color: Color::Default,
            bg_color: Color::Transparent,
        }
    }

    pub fn transparent() -> Self {
        Self {
            c: ' ',
            color: Color::Transparent,
            bg_color: Color::Transparent,
        }
    }

    pub fn with_color(self, color: [u8; 3]) -> Self {
        Self {
            color: Color::Rgb(color),
            c: self.c,
            bg_color: self.bg_color,
        }
    }

    pub fn with_bg_color(self, bg_color: [u8; 3]) -> Self {
        Self {
            bg_color: Color::Rgb(bg_color),
            c: self.c,
            color: self.color,
        }
    }

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
