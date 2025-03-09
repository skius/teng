use std::path::Path;
use std::time::Instant;
use image::GenericImageView;
use teng::rendering::color::Color;
use teng::rendering::render::HalfBlockDisplayRender;

/// A pixel sprite.
///
/// Indexing starts at the top-left corner of the sprite.
pub struct Sprite {
    pub height: u16,
    pub width: u16,
    pub pixels: Vec<Color>,
    // offset from top-left corner of the sprite. When calling render(x, y), the top-left corner will be rendered at
    // (x - attach_offset.0, y - attach_offset.1).
    attach_offset: (i16, i16),
    pub flipped_x: bool,
}

impl Sprite {
    fn get_index(&self, x: u16, y: u16) -> usize {
        let x = if self.flipped_x {
            self.width - x - 1
        } else {
            x
        };
        y as usize * self.width as usize + x as usize
    }

    pub fn render_to_hbd(&self, x: i64, y: i64, hbd: &mut HalfBlockDisplayRender) {
        for i in 0..self.height {
            for j in 0..self.width {
                let color = self.pixels[self.get_index(j, i)];
                let x = x + j as i64 - self.attach_offset.0 as i64;
                let y = y + i as i64 - self.attach_offset.1 as i64;
                // TODO: do we want the HBD to maybe ignore transparent colors? i.e., if it already has a color and someone calls set_color it gets ignored?
                // hmm. maybe there should be a separate `add_color` function that has that behavior, because set_color sounds quite authoritative.
                if !color.is_solid() {
                    continue;
                }
                if x >= 0 && y >= 0 {
                    hbd.set_color(x as usize, y as usize, color);
                }
            }
        }
    }

    pub fn set_flipped_x(&mut self, flipped_x: bool) {
        self.flipped_x = flipped_x;
    }
}

pub struct Animation {
    frames: Vec<Sprite>,
    frame_duration_secs: f32,
    start_time: Instant,
}

impl Animation {
    pub fn from_strip(filename: impl Into<String>, frame_duration_secs: f32) -> Self {
        let filename = filename.into();
        // strip suffix, then read "stripN" where N is the frame number
        // TODO: make this prettier.
        let strip_num = {
            assert!(filename.ends_with(".png"));
            let strip_num = filename.split("strip").nth(1).unwrap();
            let strip_num = strip_num.split(".").nth(0).unwrap();
            strip_num.parse::<u32>().unwrap()
        };

        let image = image::open(filename).unwrap();
        let (width, height) = image.dimensions();

        // single sprite dimension
        let sprite_width = width / strip_num;
        let sprite_height = height;

        let mut frames = Vec::new();

        for i in 0..strip_num {
            let mut pixels = Vec::new();
            for y in 0..sprite_height {
                for x in 0..sprite_width {
                    let pixel = image.get_pixel(x + i * sprite_width, y);
                    let is_transparent = pixel[3] < 255;
                    let color = if is_transparent {
                        Color::Transparent
                    } else {
                        Color::Rgb([pixel[0], pixel[1], pixel[2]])
                    };
                    pixels.push(color);
                }
            }
            frames.push(Sprite {
                height: sprite_height as u16,
                width: sprite_width as u16,
                pixels,
                attach_offset: (sprite_width as i16 / 2, sprite_height as i16 / 2),
                flipped_x: false,
            });
        }


        Animation {
            frames,
            frame_duration_secs,
            start_time: Instant::now(),
        }
    }

    pub fn render_to_hbd(&self, x: i64, y: i64, hbd: &mut HalfBlockDisplayRender, current_time: Instant) {
        let time_passed = current_time.duration_since(self.start_time).as_secs_f32();
        let frame_index = (time_passed / self.frame_duration_secs) as usize % self.frames.len();
        self.frames[frame_index].render_to_hbd(x, y, hbd);
    }

    pub fn set_flipped_x(&mut self, flipped_x: bool) {
        for frame in &mut self.frames {
            frame.set_flipped_x(flipped_x);
        }
    }
}

pub struct CombinedAnimations {
    animations: Vec<Animation>,
}

impl CombinedAnimations {
    pub fn new(animations: Vec<Animation>) -> Self {
        CombinedAnimations {
            animations,
        }
    }

    pub fn render_to_hbd(&self, x: i64, y: i64, hbd: &mut HalfBlockDisplayRender, current_time: Instant) {
        for animation in &self.animations {
            animation.render_to_hbd(x, y, hbd, current_time);
        }
    }

    pub fn set_flipped_x(&mut self, flipped_x: bool) {
        for animation in &mut self.animations {
            animation.set_flipped_x(flipped_x);
        }
    }
}