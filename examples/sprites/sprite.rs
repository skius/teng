use std::path::Path;
use std::time::Instant;
use image::GenericImageView;
use teng::rendering::color::Color;
use teng::rendering::render::HalfBlockDisplayRender;

/// A pixel sprite.
///
/// Indexing starts at the top-left corner of the sprite.
#[derive(Debug, Clone)]
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

    pub fn get_rotated(&self, angle_deg: f64) -> Self {
        let (new_width, new_height, res_buffer) = rotsprite::rotsprite(&self.pixels, &Color::Transparent, self.width as usize, angle_deg).unwrap();

        // keep attach offset the same by computing the new offset
        let new_attach_offset = {
            let (old_attach_x, old_attach_y) = self.attach_offset;
            let (old_width, old_height) = (self.width as i16, self.height as i16);
            let (new_width, new_height) = (new_width as i16, new_height as i16);
            let new_attach_x = (old_attach_x as f64 / old_width as f64 * new_width as f64).round() as i16;
            let new_attach_y = (old_attach_y as f64 / old_height as f64 * new_height as f64).round() as i16;
            (new_attach_x, new_attach_y)
        };

        Self {
            attach_offset: new_attach_offset,
            width: new_width as u16,
            height: new_height as u16,
            pixels: res_buffer,
            flipped_x: self.flipped_x,
        }
    }
}

pub enum AnimationKind {
    Repeat,
    /// This animation will not repeat. Instead it will indicate that it is done after the last frame.
    /// It can also issue a trigger at a specific frame.
    OneShot {
        // if the animation passes this frame, it will issue a trigger.
        trigger_frame: Option<usize>,
    },
}

pub enum AnimationResult {
    Done,
    Trigger,
}

pub struct Animation {
    pub frames: Vec<Sprite>,
}

impl Animation {
    pub fn from_strip(filename: impl Into<String>) -> Self {
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
        }
    }

    pub fn render_to_hbd(&self, x: i64, y: i64, hbd: &mut HalfBlockDisplayRender, frame_index: usize) {
        self.frames[frame_index].render_to_hbd(x, y, hbd);
    }

    pub fn set_flipped_x(&mut self, flipped_x: bool) {
        for frame in &mut self.frames {
            frame.set_flipped_x(flipped_x);
        }
    }
}

pub struct CombinedAnimations {
    // animation render order goes from low priority to high priority
    // invariant: all animations have the same number of frames
    pub animations: Vec<Animation>,
    pub num_frames: usize,
    frame_duration_secs: f32,
    last_rendered_frame: usize,
    has_issued_trigger: bool,
    kind: AnimationKind,
}

impl CombinedAnimations {
    pub fn new(animations: Vec<Animation>, frame_duration_secs: f32) -> Self {
        let num_frames = animations[0].frames.len();
        CombinedAnimations {
            animations,
            num_frames,
            frame_duration_secs,
            last_rendered_frame: 0,
            kind: AnimationKind::Repeat,
            has_issued_trigger: false,
        }
    }

    pub fn reset(&mut self) {
        self.last_rendered_frame = 0;
        self.has_issued_trigger = false;
    }

    pub fn set_kind(&mut self, kind: AnimationKind) {
        self.kind = kind;
    }

    pub fn is_oneshot(&self) -> bool {
        match self.kind {
            AnimationKind::OneShot { .. } => true,
            _ => false,
        }
    }

    pub fn is_finished(&self) -> bool {
        match self.kind {
            AnimationKind::OneShot { trigger_frame } => {
                if self.last_rendered_frame < self.num_frames - 1 {
                    return false;
                }
                // don't say it's finished if we haven't issued a trigger yet despite being past the last frame
                if let Some(trigger_frame) = trigger_frame {
                    if !self.has_issued_trigger && self.last_rendered_frame >= trigger_frame {
                        return false;
                    }
                }
                true
            } ,
            AnimationKind::Repeat => false,
        }
    }

    fn render_frame_index_to_hbd(&self, x: i64, y: i64, hbd: &mut HalfBlockDisplayRender, frame_index: usize) {
        for animation in &self.animations {
            animation.render_to_hbd(x, y, hbd, frame_index);
        }
    }

    pub fn render_to_hbd(&mut self, x: i64, y: i64, hbd: &mut HalfBlockDisplayRender, time_passed: f32) -> Option<AnimationResult> {
        let frame_index_unbounded = (time_passed / self.frame_duration_secs) as usize;
        match self.kind {
            AnimationKind::Repeat => {
                let frame_index = frame_index_unbounded % self.num_frames;
                self.last_rendered_frame = frame_index;
                self.render_frame_index_to_hbd(x, y, hbd, frame_index);
                None
            }
            AnimationKind::OneShot { trigger_frame } => {
                // if this is the first time we've moved past the trigger frame, issue a trigger, even if we're past the last frame
                let mut result = None;
                if let Some(trigger_frame) = trigger_frame {
                    let first_time_past_trigger = self.last_rendered_frame < trigger_frame && frame_index_unbounded >= trigger_frame;
                    if first_time_past_trigger {
                        self.has_issued_trigger = true;
                        result = Some(AnimationResult::Trigger);
                    }
                }

                if frame_index_unbounded >= self.num_frames {
                    // just render the last frame
                    let frame_index = self.num_frames - 1;
                    self.render_frame_index_to_hbd(x, y, hbd, frame_index);
                    self.last_rendered_frame = frame_index;
                    return result.or(Some(AnimationResult::Done));
                }

                let frame_index = frame_index_unbounded;

                self.last_rendered_frame = frame_index;
                self.render_frame_index_to_hbd(x, y, hbd, frame_index);
                result
            }
        }
    }

    pub fn set_flipped_x(&mut self, flipped_x: bool) {
        for animation in &mut self.animations {
            animation.set_flipped_x(flipped_x);
        }
    }
}