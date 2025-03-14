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
    #[serde(rename = "sourceSize")]
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

struct Sprite {
    size: [u16; 2],
    atlas_offset: [u16; 2],
    // The proper center position of this sprite should be computed as: top_left + center_offset
    // in other words, to find the top left corner of where to render this given the desired center, is top_left = desired_center - center_offset
    center_offset: [i16; 2],
}

struct Animation {
    // The indices of the frames in the sprite list.
    frame_indices: Vec<usize>,
}

struct CombinedAnimation {
    // The indices of the animations in the animation list.
    animation_indices: Vec<usize>,
}

pub struct TextureAtlas {
    sprites: Vec<Sprite>,
    // indices in animations point to the sprites field.
    animations: Vec<Animation>,
    // indices in combined_animations point to the animations field.
    combined_animations: Vec<CombinedAnimation>,
}

pub fn load_texture_atlas(
    atlas_image_path: impl AsRef<Path>,
    atlas_teng_meta_path: impl AsRef<Path>,
    atlas_imgpack_meta_path: impl AsRef<Path>,
) {
    let img = image::open(atlas_image_path).unwrap();
    let atlas_teng_meta = std::fs::read_to_string(atlas_teng_meta_path).unwrap();
    let atlas_imgpack_meta = std::fs::read_to_string(atlas_imgpack_meta_path).unwrap();

    let teng_meta: TengMeta = serde_json::from_str(&atlas_teng_meta).unwrap();
    let imgpack_meta: ImgPackMeta = serde_json::from_str(&atlas_imgpack_meta).unwrap();

    let mut sprites = Vec::new();
    for (name, info) in teng_meta.image_infos {
        let imgpack_info = imgpack_meta.frames.get(&name).unwrap();
        let sprite = Sprite {
            size: info.original_size,
            atlas_offset: [imgpack_info.frame.x as u16, imgpack_info.frame.y as u16],
            center_offset: info.center_offset,
        };
        sprites.push(sprite);
    }

    let mut animations = Vec::new();
    for (name, frames) in teng_meta.animations {
        // TODO: overwrite frames to skip some based on the name.

        let mut frame_indices = Vec::new();

        for frame in frames {
            let imgpack_info = imgpack_meta.frames.get(&frame.filename).unwrap();
            let sprite = Sprite {
                size: frame.original_size,
                atlas_offset: [imgpack_info.frame.x as u16, imgpack_info.frame.y as u16],
                center_offset: frame.center_offset,
            };
            let sprite_index = sprites.len();
            sprites.push(sprite);
            frame_indices.push(sprite_index);
        }

        let animation = Animation {
            frame_indices,
        };
        animations.push(animation);
    }

}