pub mod eventrecorder;
pub mod incremental;

use crate::seeds::{get_seed, get_seed_opt};
use crate::util::for_coord_in_line;
use crate::{
    BreakingAction, Component, MouseInfo, Render, Renderer, SetupInfo, SharedState, UpdateInfo,
};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind, MouseEvent, MouseEventKind};
use std::collections::HashMap;
use std::time::{Duration, Instant};

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

impl Component for DebugInfoComponent {
    fn on_event(
        &mut self,
        _event: Event,
        _shared_state: &mut SharedState,
    ) -> Option<BreakingAction> {
        self.num_events += 1;
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
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

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX - 100;
        let mut y = 0;
        format!("Help: q to quit, l to lock/unlock FPS, scroll to change FPS, b to cheat blocks, p to toggle parallax, m to toggle minimap, i to toggle debug info, r to start/stop recording").render(
            &mut renderer,
            0,
            y,
            depth_base,
        );
        y += 1;

        format!("Frame time: {} ns", self.frametime_ns).render(&mut renderer, 0, y, depth_base);
        y += 1;
        format!("Max frame time: {} ns", self.max_frametime_ns).render(
            &mut renderer,
            0,
            y,
            depth_base,
        );
        y += 1;
        // format!("Min frame time: {} ns", self.min_frametime_ns).render(&mut renderer, 0, y, depth_base);
        // y += 1;
        let target_str = if let Some(target_fps) = self.target_fps {
            format!("{:.0}", target_fps)
        } else {
            "Unlocked".to_string()
        };
        format!("FPS: {:.2} ({})", self.fps, target_str).render(&mut renderer, 0, y, depth_base);
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
        .render(&mut renderer, 0, y, depth_base);
        y += 1;
        format!("Game seed: {:?}", get_seed_opt()).render(&mut renderer, 0, y, depth_base);
        y += 1;
        format!("Debounced keys: {:?}", shared_state.debounced_down_keys).render(
            &mut renderer,
            0,
            y,
            depth_base,
        );
        y += 1;
        // format!("Events: {}", self.num_events).render(&mut renderer, 0, y, depth_base);
        // y += 1;
        // format!("Frames since last FPS: {}", self.frames_since_last_fps).render(&mut renderer, 0, y, depth_base);
        // y += 1;
        // format!("Update calls: {}", self.num_update_calls).render(&mut renderer, 0, y, depth_base);
        for dbg_msg in shared_state.debug_messages.iter() {
            for line in dbg_msg.message.as_str().lines() {
                line.render(&mut renderer, 0, y, depth_base);
                y += 1;
            }
        }
    }
}

pub struct FpsLockerComponent {
    locked: bool,
    default_fps: f64,
}

impl FpsLockerComponent {
    pub fn new(default_fps: f64) -> Self {
        Self {
            locked: true,
            default_fps,
        }
    }
}

impl Component for FpsLockerComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        shared_state.target_fps = Some(self.default_fps);
    }

    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        match event {
            Event::Mouse(me) => match me.kind {
                MouseEventKind::ScrollDown => {
                    self.default_fps -= 1.0;
                    if self.default_fps < 1.0 {
                        self.default_fps = 1.0;
                    }
                }
                MouseEventKind::ScrollUp => {
                    self.default_fps += 1.0;
                }
                _ => {}
            },
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        if shared_state.pressed_keys.did_press_char_ignore_case('l') {
            self.locked = !self.locked;
        }
        shared_state.target_fps = self.locked.then_some(self.default_fps);
    }
}

pub struct MouseEvents {
    events: Vec<MouseInfo>,
    has_new_this_frame: bool,
}

impl MouseEvents {
    pub fn new() -> Self {
        Self {
            events: vec![],
            has_new_this_frame: false,
        }
    }

    pub fn push(&mut self, event: MouseInfo) {
        self.events.push(event);
    }

    pub fn has_new_this_frame(&self) -> bool {
        self.has_new_this_frame
    }

    /// Calls the passed closure with a new mouse info for every interpolated mouse info since last frame.
    /// Only calls the closure if there has been a new event this frame.
    pub fn for_each_linerp_only_fresh(&self, f: impl FnMut(MouseInfo)) {
        if !self.has_new_this_frame {
            return;
        }

        self.for_each_linerp_sticky(f);
    }

    /// Calls the passed closure with a new mouse info for every interpolated mouse info
    /// since last frame. The closure is also called with the last mouse info if no event
    /// has been received this frame.
    /// To only get fresh events, use `for_each_linerp_only_fresh`.
    pub fn for_each_linerp_sticky(&self, mut f: impl FnMut(MouseInfo)) {
        if self.events.len() < 2 {
            self.events.first().map(|mi| f(*mi));
            return;
        }
        // do the start
        f(*self.events.first().unwrap());
        for i in 0..self.events.len() - 1 {
            // and then every pair excluding the starts.
            MouseTrackerComponent::smooth_two_updates(
                true,
                self.events[i],
                self.events[i + 1],
                &mut f,
            );
        }
    }
}

pub struct MouseTrackerComponent {
    last_mouse_info: MouseInfo,
    did_press_left: bool,
    did_press_right: bool,
    did_press_middle: bool,
    mouse_events: MouseEvents,
}

impl MouseTrackerComponent {
    pub fn new() -> Self {
        Self {
            last_mouse_info: MouseInfo::default(),
            did_press_left: false,
            did_press_right: false,
            did_press_middle: false,
            mouse_events: MouseEvents::new(),
        }
    }

    pub fn fill_mouse_info(event: MouseEvent, mouse_info: &mut MouseInfo) {
        mouse_info.last_mouse_pos = (event.column as usize, event.row as usize);
        let (button, down) = match event {
            MouseEvent {
                kind: MouseEventKind::Down(button),
                ..
            } => (button, true),
            MouseEvent {
                kind: MouseEventKind::Up(button),
                ..
            } => (button, false),
            _ => return,
        };

        match button {
            crossterm::event::MouseButton::Left => {
                mouse_info.left_mouse_down = down;
            }
            crossterm::event::MouseButton::Right => {
                mouse_info.right_mouse_down = down;
            }
            crossterm::event::MouseButton::Middle => {
                mouse_info.middle_mouse_down = down;
            }
        }
    }

    /// Calls the passed closure with a new mouse info for every interpolated mouse info between
    /// the two passed mouse infos. Also includes the endpoints.
    pub fn smooth_two_updates(
        exclude_start: bool,
        first: MouseInfo,
        second: MouseInfo,
        mut f: impl FnMut(MouseInfo),
    ) {
        // linearly interpolate from first to second pixel.
        // so, rasterize a line connecting the two points

        let start_x = first.last_mouse_pos.0 as i64;
        let start_y = first.last_mouse_pos.1 as i64;
        let end_x = second.last_mouse_pos.0 as i64;
        let end_y = second.last_mouse_pos.1 as i64;

        for_coord_in_line(exclude_start, (start_x, start_y), (end_x, end_y), |x, y| {
            // important, use first mouse info to determine mouse button state, avoids edge cases
            // when entering and leaving terminal/process
            // however, use second mouse info for the end point
            let mi = if (x, y) == (end_x, end_y) {
                second
            } else {
                first
            };

            f(MouseInfo {
                last_mouse_pos: (x as usize, y as usize),
                left_mouse_down: mi.left_mouse_down,
                right_mouse_down: mi.right_mouse_down,
                middle_mouse_down: mi.middle_mouse_down,
            });
        });
    }
}

impl Component for MouseTrackerComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        if let Event::Mouse(event) = event {
            Self::fill_mouse_info(event, &mut self.last_mouse_info);
            self.mouse_events.push(self.last_mouse_info);
            self.mouse_events.has_new_this_frame = true;
            match event {
                MouseEvent {
                    kind: MouseEventKind::Down(button),
                    ..
                } => match button {
                    // Note: we are sticky-setting did_press_*: even if a Up event appear in the same
                    // frame, we're keeping the 'true'. Only next frame will we reset.
                    // The mouse_events queue should be used to handle inter-frame events.
                    crossterm::event::MouseButton::Left => {
                        self.did_press_left = true;
                    }
                    crossterm::event::MouseButton::Right => {
                        self.did_press_right = true;
                    }
                    crossterm::event::MouseButton::Middle => {
                        self.did_press_middle = true;
                    }
                },
                _ => {}
            }
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        shared_state.mouse_info = self.last_mouse_info;
        shared_state.mouse_pressed.right = self.did_press_right;
        shared_state.mouse_pressed.left = self.did_press_left;
        shared_state.mouse_pressed.middle = self.did_press_middle;
        std::mem::swap(&mut self.mouse_events, &mut shared_state.mouse_events);
        self.mouse_events.events.clear();
        // always have the last mouse info in the queue
        self.mouse_events.push(self.last_mouse_info);
        self.mouse_events.has_new_this_frame = false;

        self.did_press_left = false;
        self.did_press_right = false;
        self.did_press_middle = false;
    }
}

pub struct QuitterComponent;

impl Component for QuitterComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        if matches!(
            event,
            Event::Key(KeyEvent {
                // TODO: Add breakingaction to update() and move this there and used shared_state?
                code: KeyCode::Char('q' | 'Q'),
                ..
            })
        ) {
            Some(BreakingAction::Quit)
        } else {
            None
        }
    }
}

pub struct PressedKeys {
    inner: micromap::Map<KeyCode, u8, 16>,
}

impl PressedKeys {
    pub fn new() -> Self {
        Self {
            inner: micromap::Map::new(),
        }
    }

    pub fn inner(&self) -> &micromap::Map<KeyCode, u8, 16> {
        &self.inner
    }

    /// Not recommended to use. However, it is useful to hack key actions in other components
    /// if the update order is known.
    pub fn insert(&mut self, key: KeyCode) {
        self.inner.insert(key, 1);
    }

    pub fn did_press_char(&self, c: char) -> bool {
        self.inner.contains_key(&KeyCode::Char(c))
    }

    pub fn did_press_char_ignore_case(&self, c: char) -> bool {
        self.did_press_char(c) || self.did_press_char(c.to_ascii_uppercase())
    }
}

/// Must be before any other components that use key presses.
pub struct KeyPressRecorderComponent {
    pressed_keys: micromap::Map<KeyCode, u8, 16>,
}

impl KeyPressRecorderComponent {
    pub fn new() -> Self {
        Self {
            pressed_keys: micromap::Map::new(),
        }
    }
}

impl Component for KeyPressRecorderComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        match event {
            // only capture presses to work on windows as well (where we get Release too)
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code,
                ..
            }) => {
                if let Some(count) = self.pressed_keys.get_mut(&code) {
                    *count += 1;
                } else {
                    // UB if we insert more than 16 keys
                    if self.pressed_keys.len() < 16 {
                        self.pressed_keys.insert(code, 1);
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        std::mem::swap(&mut shared_state.pressed_keys.inner, &mut self.pressed_keys);
        self.pressed_keys.clear();
    }
}

pub struct KeypressDebouncerComponent {
    max_delay_ms: u128,
    last_keypress: HashMap<KeyCode, Instant>,
}

impl KeypressDebouncerComponent {
    pub fn new(max_delay_ms: u128) -> Self {
        Self {
            max_delay_ms,
            last_keypress: HashMap::new(),
        }
    }
}

impl Component for KeypressDebouncerComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        match event {
            // only capture presses to work on windows as well (where we get Release too)
            Event::Key(KeyEvent {
                kind: KeyEventKind::Press,
                code,
                ..
            }) => {
                self.last_keypress.insert(code, Instant::now());
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        // go through all keys, consume the ones that have expired
        shared_state.debounced_down_keys.clear();
        self.last_keypress = self
            .last_keypress
            .drain()
            .filter(|(key, time)| {
                if time.elapsed().as_millis() < self.max_delay_ms {
                    shared_state.debounced_down_keys.insert(*key);
                    true
                } else {
                    false
                }
            })
            .collect();
    }
}
