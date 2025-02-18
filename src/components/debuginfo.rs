use std::time::{Duration, Instant};
use crossterm::event::Event;
use crate::{BreakingAction, Component, SharedState, UpdateInfo};
use crate::rendering::render::Render;
use crate::rendering::renderer::Renderer;
use crate::seeds::get_seed_opt;

pub struct DebugMessage {
    message: String,
    expiry_time: Instant,
}

impl DebugMessage {
    pub fn new(message: impl Into<String>, expiry_time: Instant) -> Self {
        Self {
            message: message.into(),
            expiry_time,
        }
    }

    pub fn new_3s(message: impl Into<String>) -> Self {
        Self::new(message.into(), Instant::now() + Duration::from_secs(3))
    }
}

#[derive(Debug, Default, Clone)]
pub struct DebugInfo {
    player_y: f64,
    player_x: f64,
    left_wall: f64,
    bottom_wall: f64,
    y_vel: f64,
    target_queue: Vec<u16>,
}

impl DebugInfo {
    pub fn new() -> Self {
        Self::default()
    }
}

pub struct DebugInfoComponent {
    frametime_ns: u128,
    max_frametime_time: Instant,
    max_frametime_ns: u128,
    min_frametime_ns: u128,
    last_fps_time: Instant,
    fps: f64,
    target_fps: Option<f64>,
    frames_since_last_fps: u32,
    num_events: u64,
    num_update_calls: u64,
    sum_actual_dts: f64,
    last_actual_fps_computed: f64,
}

impl DebugInfoComponent {
    pub fn new() -> Self {
        Self {
            frametime_ns: 0,
            max_frametime_time: Instant::now(),
            max_frametime_ns: 0,
            min_frametime_ns: u128::MAX,
            last_fps_time: Instant::now(),
            fps: 0.0,
            target_fps: None,
            frames_since_last_fps: 0,
            num_events: 0,
            num_update_calls: 0,
            sum_actual_dts: 0.0,
            last_actual_fps_computed: 0.0,
        }
    }
}

impl DebugInfoComponent {
    const MAX_FRAMETIME_WINDOW: Duration = Duration::from_secs(5);
    const FPS_UPDATE_INTERVAL: Duration = Duration::from_millis(200);
}

impl<S> Component<S> for DebugInfoComponent {
    fn on_event(
        &mut self,
        _event: Event,
        _shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        self.num_events += 1;
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        self.num_update_calls += 1;
        let UpdateInfo {
            last_time,
            current_time,
            actual_dt,
            ..
        } = update_info;
        self.sum_actual_dts += actual_dt;

        let delta_time_ns = current_time.duration_since(last_time).as_nanos();
        self.frametime_ns = delta_time_ns;

        if delta_time_ns < self.min_frametime_ns {
            self.min_frametime_ns = delta_time_ns;
        }

        if current_time - self.max_frametime_time > Self::MAX_FRAMETIME_WINDOW {
            self.max_frametime_ns = 0;
        }

        if delta_time_ns > self.max_frametime_ns {
            self.max_frametime_ns = delta_time_ns;
            self.max_frametime_time = Instant::now();
        }

        self.frames_since_last_fps += 1;
        if current_time - self.last_fps_time > Self::FPS_UPDATE_INTERVAL {
            self.fps = (self.frames_since_last_fps as f64)
                / (current_time - self.last_fps_time).as_secs_f64();
            self.last_actual_fps_computed =
                1.0 / (self.sum_actual_dts / self.frames_since_last_fps as f64);
            self.frames_since_last_fps = 0;
            self.sum_actual_dts = 0.0;
            self.last_fps_time = current_time;
        }
        self.target_fps = shared_state.target_fps;

        // expire debug messages
        shared_state
            .debug_messages
            .retain(|msg| msg.expiry_time > current_time);
        // only keep the 10 most recent messages
        if shared_state.debug_messages.len() > 10 {
            shared_state
                .debug_messages
                .drain(0..shared_state.debug_messages.len() - 10);
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<S>, depth_base: i32) {
        let depth_base = i32::MAX - 100;
        let mut y = 0;
        format!("Help: q to quit, l to lock/unlock FPS, scroll to change FPS, b to cheat blocks, p to toggle parallax, m to toggle minimap, i to toggle debug info, r to start/stop recording").render(
            renderer,
            0,
            y,
            depth_base,
        );
        y += 1;

        format!("Frame time: {} ns", self.frametime_ns).render(renderer, 0, y, depth_base);
        y += 1;
        format!("Max frame time: {} ns", self.max_frametime_ns).render(renderer, 0, y, depth_base);
        y += 1;
        // format!("Min frame time: {} ns", self.min_frametime_ns).render(&mut renderer, 0, y, depth_base);
        // y += 1;
        let target_str = if let Some(target_fps) = self.target_fps {
            format!("{:.0}", target_fps)
        } else {
            "Unlocked".to_string()
        };
        format!("FPS: {:.2} ({})", self.fps, target_str).render(renderer, 0, y, depth_base);
        y += 1;
        // format!("Achievable FPS: {:.2}", self.last_actual_fps_computed).render(
        //     &mut renderer,
        //     0,
        //     y,
        //     depth_base,
        // );
        // y += 1;
        // let debug_string = format!("DebugInfo: {:#?}", shared_state.debug_info);
        // for line in debug_string.lines() {
        //     line.render(&mut renderer, 0, y, depth_base);
        //     y += 1;
        // }
        format!(
            "Display size: {}x{}",
            shared_state.display_info.width(),
            shared_state.display_info.height()
        )
            .render(renderer, 0, y, depth_base);
        y += 1;
        format!("Game seed: {:?}", get_seed_opt()).render(renderer, 0, y, depth_base);
        y += 1;
        format!("Debounced keys: {:?}", shared_state.debounced_down_keys)
            .render(renderer, 0, y, depth_base);
        y += 1;
        // format!("Events: {}", self.num_events).render(&mut renderer, 0, y, depth_base);
        // y += 1;
        // format!("Frames since last FPS: {}", self.frames_since_last_fps).render(&mut renderer, 0, y, depth_base);
        // y += 1;
        // format!("Update calls: {}", self.num_update_calls).render(&mut renderer, 0, y, depth_base);
        for dbg_msg in shared_state.debug_messages.iter() {
            for line in dbg_msg.message.as_str().lines() {
                line.render(renderer, 0, y, depth_base);
                y += 1;
            }
        }
    }
}