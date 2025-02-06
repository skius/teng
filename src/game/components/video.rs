use rust_embed::Embed;
use tempfile::env::temp_dir;
use video_rs::Decoder;
use crate::game::{Component, Pixel, Renderer, SharedState, UpdateInfo};

#[derive(Embed)]
#[folder = "assets/"]
struct Asset;

struct Frame(Vec<[u8; 3]>);

pub struct VideoComponent {
    width: usize,
    height: usize,
    fps: f64,
    frames: Vec<Frame>,
    start_time: std::time::Instant,
    render_frame_idx: usize,
}

impl VideoComponent {
    pub fn new() -> Self {
        video_rs::init().unwrap();

        let asset = Asset::get("output4-small.mp4").unwrap();
        let bytes = asset.data.as_ref();

        let temp_dir = temp_dir();
        let temp_file = temp_dir.join("output4-small.mp4");
        std::fs::write(&temp_file, bytes).unwrap();


        let mut decoder = Decoder::new(temp_file).unwrap();

        let width = 172;
        let height = 134;
        let fps = 25.0;

        let mut frames = vec![];

        for frame in decoder.decode_iter() {
            if let Ok((_, frame)) = frame {
                assert!(frame.len() == width * height * 3);
                let mut frame_rust = vec![];
                for i in 0..height {
                    for j in 0..width {
                        let rgb = frame.slice(ndarray::s![i, j, ..]);
                        let r = rgb[0];
                        let g = rgb[1];
                        let b = rgb[2];
                        frame_rust.push([r, g, b]);
                    }
                }
                frames.push(Frame(frame_rust));
            } else {
                println!("Error decoding frame: {:?}", frame);
                break;
            }
        }

        VideoComponent {
            width,
            height,
            fps,
            frames,
            start_time: std::time::Instant::now(),
            render_frame_idx: 0,
        }
    }
}

impl Component for VideoComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let frame_duration = 1.0 / self.fps;
        let frame_idx = (update_info.current_time - self.start_time).as_secs_f64() / frame_duration;
        self.render_frame_idx = frame_idx as usize % self.frames.len();
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let offset_x = (shared_state.display_info.width() - self.width) / 2;
        let frame = &self.frames[self.render_frame_idx];

        for y in 0..(self.height/2) {
            for x in 0..self.width {
                let color_top = frame.0[2 * y * self.width + x];
                let color_bottom = frame.0[(2 * y+1) * self.width + x];

                let mut pixel = Pixel::new('▄');
                let pixel = pixel.with_color(color_bottom).with_bg_color(color_top);

                renderer.render_pixel(x + offset_x, y, pixel, i32::MAX);
            }
        }
    }
}