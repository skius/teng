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

        let mut atlas = TextureAnimationAtlas {
            sprites,
            animations,
            combined_animations: HashMap::new(),
        };

        let different_parts = ["base", "bowlhair", "tools"];
        let human_prefix = "Characters_Human_";

        atlas.push_combined_animation(CombinedAnimationKey::PlayerIdle, human_prefix, "IDLE", "idle", &different_parts);
        atlas.push_combined_animation(CombinedAnimationKey::PlayerWalking, human_prefix, "WALKING", "walk", &different_parts);
        atlas.push_combined_animation(CombinedAnimationKey::PlayerRun, human_prefix, "RUN", "run", &different_parts);
        atlas.push_combined_animation(CombinedAnimationKey::PlayerAxe, human_prefix, "AXE", "axe", &different_parts);
        atlas.push_combined_animation(CombinedAnimationKey::PlayerAttack, human_prefix, "ATTACK", "attack", &different_parts);
        atlas.push_combined_animation(CombinedAnimationKey::PlayerCaught, human_prefix, "CAUGHT", "caught", &different_parts);
        atlas.push_combined_animation(CombinedAnimationKey::PlayerJump, human_prefix, "JUMP", "jump", &different_parts);
        atlas.push_combined_animation(CombinedAnimationKey::PlayerRoll, human_prefix, "ROLL", "roll", &different_parts);


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

    fn push_combined_animation(&mut self, name: CombinedAnimationKey, folder_prefix: &str, prefix: &str, suffix: &str, parts: &[&str]) {
        self.combined_animations.insert(name.as_str().to_string(), self.create_combined_animation(folder_prefix, prefix, suffix, parts));
    }

    // Updates all animations that are part of the combined animation to use the given frame indices.
    fn set_combined_animation_frame_indices(&mut self, name: &str, indices: &[usize]) {
        let combined_animation = self.combined_animations.get(name).unwrap();
        for anim_name in &combined_animation.animation_names {
            let animation = self.animations.get_mut(anim_name).unwrap();
            *animation = animation.with_frame_indices(indices);
        }
    }

    fn frame_count_ca(&self, name: &str) -> usize {
        let combined_animation = self.combined_animations.get(name).unwrap();
        let first_anim_name = &combined_animation.animation_names[0];
        self.animations.get(first_anim_name).unwrap().frame_names.len()
    }

    fn frame_count_a(&self, name: &str) -> usize {
        self.animations.get(name).unwrap().frame_names.len()
    }

    pub fn frame_count(&self, key: AnimationKey) -> usize {
        match key {
            AnimationKey::CombinedAnimation(combined_key) => self.frame_count_ca(combined_key.as_str()),
            AnimationKey::Animation(single_key) => self.frame_count_a(single_key.as_str()),
        }
    }

    pub fn get_sprites_for_ca_with_frame(&self, name: &str, frame: usize) -> impl Iterator<Item=Sprite> + '_ {
        let combined_animation = self.combined_animations.get(name).unwrap();
        combined_animation.animation_names.iter().map(move |anim_name| {
            let animation = self.animations.get(anim_name).unwrap();
            let frame_name = &animation.frame_names[frame];
            // panic!("Frame name: {}", frame_name);
            self.sprites.get(frame_name).unwrap().clone()
        })
    }

    pub fn get_sprites_for_a_with_frame(&self, name: &str, frame: usize) -> impl Iterator<Item = Sprite> + use<> {
        let animation = self.animations.get(name).unwrap();
        let frame_name = &animation.frame_names[frame];
        Some(self.sprites.get(frame_name).copied().unwrap()).into_iter()
    }

    pub fn get_sprites_frame(&self, key: AnimationKey, frame: usize) -> Box<dyn Iterator<Item = Sprite> + '_> {
        match key {
            AnimationKey::CombinedAnimation(combined_key) => Box::new(self.get_sprites_for_ca_with_frame(combined_key.as_str(), frame)),
            AnimationKey::Animation(single_key) => Box::new(self.get_sprites_for_a_with_frame(single_key.as_str(), frame)),
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum CombinedAnimationKey<'a> {
    PlayerIdle,
    PlayerWalking,
    PlayerRun,
    PlayerAxe,
    PlayerAttack,
    PlayerCaught,
    PlayerJump,
    PlayerRoll,
    Custom(&'a str),
}

impl<'a> CombinedAnimationKey<'a> {
    fn as_str(&self) -> &str {
        match self {
            CombinedAnimationKey::PlayerIdle => "PlayerIdle",
            CombinedAnimationKey::PlayerWalking => "PlayerWalking",
            CombinedAnimationKey::PlayerRun => "PlayerRun",
            CombinedAnimationKey::PlayerAxe => "PlayerAxe",
            CombinedAnimationKey::PlayerAttack => "PlayerAttack",
            CombinedAnimationKey::PlayerCaught => "PlayerCaught",
            CombinedAnimationKey::PlayerJump => "PlayerJump",
            CombinedAnimationKey::PlayerRoll => "PlayerRoll",
            CombinedAnimationKey::Custom(name) => name,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum SingleAnimationKey<'a> {
    Custom(&'a str),
}

impl<'a> SingleAnimationKey<'a> {
    fn as_str(&self) -> &str {
        match self {
            SingleAnimationKey::Custom(name) => name,
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub enum AnimationKey<'a> {
    CombinedAnimation(CombinedAnimationKey<'a>),
    Animation(SingleAnimationKey<'a>),
}

impl<'a> AnimationKey<'a> {
    pub const PLAYER_IDLE: AnimationKey<'static> = AnimationKey::CombinedAnimation(CombinedAnimationKey::PlayerIdle);
    pub const PLAYER_WALKING: AnimationKey<'static> = AnimationKey::CombinedAnimation(CombinedAnimationKey::PlayerWalking);
    pub const PLAYER_RUN: AnimationKey<'static> = AnimationKey::CombinedAnimation(CombinedAnimationKey::PlayerRun);
    pub const PLAYER_AXE: AnimationKey<'static> = AnimationKey::CombinedAnimation(CombinedAnimationKey::PlayerAxe);
    pub const PLAYER_ATTACK: AnimationKey<'static> = AnimationKey::CombinedAnimation(CombinedAnimationKey::PlayerAttack);
    pub const PLAYER_CAUGHT: AnimationKey<'static> = AnimationKey::CombinedAnimation(CombinedAnimationKey::PlayerCaught);
    pub const PLAYER_JUMP: AnimationKey<'static> = AnimationKey::CombinedAnimation(CombinedAnimationKey::PlayerJump);
    pub const PLAYER_ROLL: AnimationKey<'static> = AnimationKey::CombinedAnimation(CombinedAnimationKey::PlayerRoll);
}