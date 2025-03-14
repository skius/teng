use std::collections::HashMap;
use std::path::Path;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
struct TengMeta {
    image_infos: HashMap<String, ImageInfo>,
    animations: HashMap<String, Vec<ImageInfo>>,
}

#[derive(Serialize, Deserialize)]
struct ImageInfo {
    original_size: [u16; 2],
    solid_offset: [u16; 2],
    center_offset: [i16; 2],
    filename: String,
}

#[derive(Serialize, Deserialize)]
struct ImgPackMeta {
    frames: HashMap<String, ImgPackInfo>,
    meta: ImgPackMetaInfo,
}

#[derive(Serialize, Deserialize)]
struct ImgPackInfo {
    frame: ImgPackXywh,
    rotated: bool,
    trimmed: bool,
    #[serde(rename = "spriteSourceSize")]
    sprite_source_size: ImgPackXywh,
    #[serde(rename = "sourceRects")]
    source_rects: ImgPackWh,
    pivot: ImgPackXy,
}

#[derive(Serialize, Deserialize)]
struct ImgPackMetaInfo {
    app: String,
    version: String,
    format: String,
    size: ImgPackWh,
    scale: String,
}

#[derive(Serialize, Deserialize)]
struct ImgPackWh {
    w: f64,
    h: f64,
}

#[derive(Serialize, Deserialize)]
struct ImgPackXy {
    x: f64,
    y: f64,
}

#[derive(Serialize, Deserialize)]
struct ImgPackXywh {
    x: f64,
    y: f64,
    w: f64,
    h: f64,
}

#[derive(Debug, Clone, Copy)]
pub struct Sprite {
    pub size: [u16; 2],
    pub atlas_offset: [u16; 2],
    // The proper center position of this sprite should be computed as: top_left + center_offset
    // in other words, to find the top left corner of where to render this given the desired center, is top_left = desired_center - center_offset
    pub center_offset: [i16; 2],
}

impl Sprite {
    fn from_teng_and_imgpack_info(teng_info: &ImageInfo, imgpack_info: &ImgPackInfo) -> Sprite {
        Sprite {
            size: [imgpack_info.source_rects.w as u16, imgpack_info.source_rects.h as u16],
            atlas_offset: [imgpack_info.frame.x as u16, imgpack_info.frame.y as u16],
            center_offset: teng_info.center_offset,
        }
    }
}

#[derive(Debug, Clone)]
struct Animation {
    // The names of the frames in the sprite map. Keys into the sprites field.
    // Order is relevant and determines the order of the frames in the animation.
    frame_names: Vec<String>,
}

impl Animation {
    pub fn with_frame_indices(&self, indices: &[usize]) -> Animation {
        let frame_names = indices.iter().map(|&i| self.frame_names[i].clone()).collect();
        Animation {
            frame_names,
        }
    }
}

#[derive(Debug, Clone)]
struct CombinedAnimation {
    // The names of the animations in the animation map. Keys into the animations field.
    // Each animation must have the same number of frames.
    animation_names: Vec<String>,
}

#[derive(Debug, Clone)]
pub struct TextureAnimationAtlas {
    sprites: HashMap<String, Sprite>,
    animations: HashMap<String, Animation>,
    combined_animations: HashMap<String, CombinedAnimation>,
}



impl TextureAnimationAtlas {
    pub fn load(
        atlas_image_path: impl AsRef<Path>,
        atlas_teng_meta_path: impl AsRef<Path>,
        atlas_imgpack_meta_path: impl AsRef<Path>,
    ) -> (Self, image::DynamicImage) {
        let img = image::open(atlas_image_path).unwrap();
        let atlas_teng_meta = std::fs::read_to_string(atlas_teng_meta_path).unwrap();
        let atlas_imgpack_meta = std::fs::read_to_string(atlas_imgpack_meta_path).unwrap();

        let teng_meta: TengMeta = serde_json::from_str(&atlas_teng_meta).unwrap();
        let imgpack_meta: ImgPackMeta = serde_json::from_str(&atlas_imgpack_meta).unwrap();

        let mut sprites = HashMap::new();
        for (name, info) in teng_meta.image_infos {
            let imgpack_info = imgpack_meta.frames.get(&name).unwrap();
            let sprite = Sprite::from_teng_and_imgpack_info(&info, imgpack_info);
            sprites.insert(name, sprite);
        }

        let mut animations = HashMap::new();
        for (name, frames) in teng_meta.animations {
            // TODO: overwrite frames to skip some based on the name.

            let mut frame_names = Vec::new();

            for frame in frames {
                let imgpack_info = imgpack_meta.frames.get(&frame.filename).unwrap();
                let sprite = Sprite::from_teng_and_imgpack_info(&frame, imgpack_info);
                sprites.insert(frame.filename.clone(), sprite);
                frame_names.push(frame.filename);
            }

            let animation = Animation {
                frame_names
            };
            let anim_name = name.split("_strip").next().unwrap().to_string();
            animations.insert(anim_name, animation);
        }

        let different_parts = ["base", "bowlhair", "tools"];


        let human_prefix = "Characters_Human_";
        let prefix_suffix_pairs = vec![
            ("IDLE", "idle", 9),
            ("WALKING", "walk", 8),
            ("RUN", "run", 8),
            ("AXE", "axe", 10),
            ("ATTACK", "attack", 10),
            ("CAUGHT", "caught", 10),
            ("JUMP", "jump", 9),
            ("ROLL", "roll", 10),
        ];

        let mut combined_animations = HashMap::new();
        for (prefix, suffix, _stripnum) in prefix_suffix_pairs {
            let mut animation_names = Vec::new();
            for part in different_parts.iter() {
                let anim_name = format!("{human_prefix}{prefix}_{part}_{suffix}");
                animation_names.push(anim_name);
            }
            let combined_animation = CombinedAnimation {
                animation_names,
            };
            combined_animations.insert(suffix.to_string(), combined_animation);
        }

        let mut atlas = TextureAnimationAtlas {
            sprites,
            animations,
            combined_animations: HashMap::new(),
        };

        atlas.push_combined_animation("PlayerIdle", human_prefix, "IDLE", "idle", &different_parts);
        atlas.push_combined_animation("PlayerWalking", human_prefix, "WALKING", "walk", &different_parts);
        atlas.push_combined_animation("PlayerRun", human_prefix, "RUN", "run", &different_parts);
        atlas.push_combined_animation("PlayerAxe", human_prefix, "AXE", "axe", &different_parts);
        atlas.push_combined_animation("PlayerAttack", human_prefix, "ATTACK", "attack", &different_parts);
        atlas.push_combined_animation("PlayerCaught", human_prefix, "CAUGHT", "caught", &different_parts);
        atlas.push_combined_animation("PlayerJump", human_prefix, "JUMP", "jump", &different_parts);
        atlas.push_combined_animation("PlayerRoll", human_prefix, "ROLL", "roll", &different_parts);


        (atlas, img)
    }

    fn create_combined_animation(&self, folder_prefix: &str, prefix: &str, suffix: &str, parts: &[&str]) -> CombinedAnimation {
        let mut animation_names = Vec::new();
        for part in parts.iter() {
            let anim_name = format!("{folder_prefix}{prefix}_{part}_{suffix}");
            animation_names.push(anim_name);
        }
        CombinedAnimation {
            animation_names,
        }
    }

    fn push_combined_animation(&mut self, name: &str, folder_prefix: &str, prefix: &str, suffix: &str, parts: &[&str]) {
        self.combined_animations.insert(name.to_string(), self.create_combined_animation(folder_prefix, prefix, suffix, parts));
    }

    // Updates all animations that are part of the combined animation to use the given frame indices.
    fn set_combined_animation_frame_indices(&mut self, name: &str, indices: &[usize]) {
        let combined_animation = self.combined_animations.get(name).unwrap();
        for anim_name in &combined_animation.animation_names {
            let animation = self.animations.get_mut(anim_name).unwrap();
            *animation = animation.with_frame_indices(indices);
        }
    }

    pub fn get_sprites_for_ca_with_frame(&self, name: &str, frame: usize) -> impl Iterator<Item = Sprite> {
        let combined_animation = self.combined_animations.get(name).unwrap();
        combined_animation.animation_names.iter().map(move |anim_name| {
            let animation = self.animations.get(anim_name).unwrap();
            let frame_name = &animation.frame_names[frame];
            // panic!("Frame name: {}", frame_name);
            self.sprites.get(frame_name).unwrap().clone()
        })
    }
}