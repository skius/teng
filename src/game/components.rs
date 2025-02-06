pub mod elevator;
pub mod incremental;
pub mod video;

use crate::game::display::Display;
use crate::game::{
    BreakingAction, Component, MouseInfo, Pixel, Render, Renderer, SharedState, Sprite, UpdateInfo,
};
use crate::physics::PhysicsBoard;
use crossterm::event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use smallvec::SmallVec;
use std::time::{Duration, Instant};

#[derive(Debug, Default, Clone)]
pub struct ElevatorInfo {
    total: usize,
    total_finished: usize,
    total_in_elevator_now: usize,
    total_waiting_now: usize,
    avg_wait_time_finished: f64,
    avg_wait_time_overall: f64,
    max_wait_time: f64,
    spawn_rate: f64,
    avg_wait_time_overall_per_spawn_rate: f64,
}

#[derive(Debug, Default, Clone)]
pub struct DebugInfo {
    player_y: f64,
    player_x: f64,
    left_wall: f64,
    bottom_wall: f64,
    y_vel: f64,
    elevator_info: ElevatorInfo,
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
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
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
            self.last_actual_fps_computed = 1.0 / (self.sum_actual_dts / self.frames_since_last_fps as f64);
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
        format!("Help: q to quit, l to lock/unlock FPS, scroll to change FPS, LMB for drawing, RMB for flood fill, c to clear, WASD to walk, arrow keys to shoot, space to jump, f to apply force").render(
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
        format!("Achievable FPS: {:.2}", self.last_actual_fps_computed).render(&mut renderer, 0, y, depth_base);
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
        // format!("Pressed keys: {:?}", shared_state.pressed_keys).render(&mut renderer, 0, y, depth_base);
        // y += 1;
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

pub struct FPSLockerComponent {
    locked: bool,
    default_fps: f64,
}

impl FPSLockerComponent {
    pub fn new(default_fps: f64) -> Self {
        Self {
            locked: true,
            default_fps,
        }
    }
}

impl Component for FPSLockerComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                ..
            }) => {
                self.locked = !self.locked;
            }
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
        shared_state.target_fps = self.locked.then_some(self.default_fps);
    }
}

pub struct MouseTrackerComponent {
    last_mouse_info: MouseInfo,
    did_press_left: bool,
    did_press_right: bool,
    did_press_middle: bool,
}

impl MouseTrackerComponent {
    pub fn new() -> Self {
        Self {
            last_mouse_info: MouseInfo::default(),
            did_press_left: false,
            did_press_right: false,
            did_press_middle: false,
        }
    }

    pub fn fill_mouse_info(event: MouseEvent, mouse_info: &mut MouseInfo) {
        mouse_info.last_mouse_pos = (event.column as usize, event.row as usize);
        match event {
            MouseEvent {
                kind: MouseEventKind::Down(button),
                ..
            } => match button {
                crossterm::event::MouseButton::Left => {
                    mouse_info.left_mouse_down = true;
                }
                crossterm::event::MouseButton::Right => {
                    mouse_info.right_mouse_down = true;
                }
                crossterm::event::MouseButton::Middle => {
                    mouse_info.middle_mouse_down = true;
                }
            },
            MouseEvent {
                kind: MouseEventKind::Up(button),
                ..
            } => match button {
                crossterm::event::MouseButton::Left => {
                    mouse_info.left_mouse_down = false;
                }
                crossterm::event::MouseButton::Right => {
                    mouse_info.right_mouse_down = false;
                }
                crossterm::event::MouseButton::Middle => {
                    mouse_info.middle_mouse_down = false;
                }
            },
            _ => {}
        }
    }

    /// Calls the passed closure with a new mouse info for every interpolated mouse info between
    /// the two passed mouse infos. Also includes the endpoints.
    pub fn smooth_two_updates(first: MouseInfo, second: MouseInfo, mut f: impl FnMut(MouseInfo)) {
        // linearly interpolate from first to second pixel.
        // so, rasterize a line connecting the two points

        let start_x = first.last_mouse_pos.0 as i32;
        let start_y = first.last_mouse_pos.1 as i32;
        let end_x = second.last_mouse_pos.0 as i32;
        let end_y = second.last_mouse_pos.1 as i32;

        // TODO: Understand this code
        let dx = (end_x - start_x).abs();
        let dy = (end_y - start_y).abs();
        let sx = if start_x < end_x { 1 } else { -1 };
        let sy = if start_y < end_y { 1 } else { -1 };
        let mut err = dx - dy;
        let mut x = start_x;
        let mut y = start_y;

        while x != end_x || y != end_y {
            f(MouseInfo {
                last_mouse_pos: (x as usize, y as usize),
                // important, use first mouse info to determine mouse button state, avoids edge cases
                // when entering and leaving terminal/process
                left_mouse_down: first.left_mouse_down,
                right_mouse_down: first.right_mouse_down,
                middle_mouse_down: first.middle_mouse_down,
            });

            let e2 = 2 * err;
            if e2 > -dy {
                err -= dy;
                x += sx;
            }
            if e2 < dx {
                err += dx;
                y += sy;
            }
        }

        f(second);
    }
}

impl Component for MouseTrackerComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        if let Event::Mouse(event) = event {
            Self::fill_mouse_info(event, &mut self.last_mouse_info);
            match event {
                MouseEvent {
                    kind: MouseEventKind::Down(button),
                    ..
                } => match button {
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
        // if shared_state.mouse_info != self.last_mouse_info {
        //     shared_state.debug_messages.push(DebugMessage::new(
        //         format!("Mouse: {:?}", self.last_mouse_info),
        //         update_info.current_time + Duration::from_secs(5),
        //     ));
        // }
        shared_state.mouse_info = self.last_mouse_info;
        shared_state.mouse_pressed.right = self.did_press_right;
        shared_state.mouse_pressed.left = self.did_press_left;
        shared_state.mouse_pressed.middle = self.did_press_middle;
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
                code: KeyCode::Char('q'),
                ..
            })
        ) {
            Some(BreakingAction::Quit)
        } else {
            None
        }
    }
}

pub struct FloodFillComponent {
    has_content: bool,
    board: Display<bool>,
    visited: Display<bool>,
    stack: Vec<(i32, i32)>,
    last_mouse_info: MouseInfo,
    received_down_event_this_frame: bool,
}

impl FloodFillComponent {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            has_content: false,
            board: Display::new(width, height, false),
            visited: Display::new(width, height, false),
            stack: vec![],
            last_mouse_info: MouseInfo::default(),
            received_down_event_this_frame: false,
        }
    }

    fn flood_fill(&mut self) -> bool {
        // determine inaccessible regions starting from the border. 'true' determines a line and must
        // not be crossed.

        // Reset from previous flood fill
        self.visited.fill(false);
        self.stack.clear();

        let width = self.board.width();
        let height = self.board.height();

        for x in 0..width {
            if !self.board[(x, 0)] {
                self.stack.push((x as i32, 0));
            }
            if !self.board[(x, height - 1)] {
                self.stack.push((x as i32, height as i32 - 1));
            }
        }
        for y in 0..height {
            if !self.board[(0, y)] {
                self.stack.push((0, y as i32));
            }
            if !self.board[(width - 1, y)] {
                self.stack.push((width as i32 - 1, y as i32));
            }
        }

        while let Some((x, y)) = self.stack.pop() {
            if y < 0 || y >= height as i32 || x < 0 || x >= width as i32 {
                // oob
                continue;
            }
            let x = x as usize;
            let y = y as usize;
            if self.board[(x, y)] {
                // wall, skip
                continue;
            }
            if self.visited[(x, y)] {
                // visited, skip
                continue;
            }
            self.visited[(x, y)] = true;
            self.stack.push((x as i32 - 1, y as i32));
            self.stack.push((x as i32 + 1, y as i32));
            self.stack.push((x as i32, y as i32 - 1));
            self.stack.push((x as i32, y as i32 + 1));
        }

        // fill inaccessible regions
        let mut flood_fill_happened = false;
        for y in 0..height {
            for x in 0..width {
                if !self.visited[(x, y)] {
                    self.board[(x, y)] = true;
                    flood_fill_happened = true;
                }
            }
        }

        flood_fill_happened
    }

    fn on_release(&mut self, shared_state: &mut SharedState, current_time: Instant) {
        // do what needs to happen after release
        // move over to decay board
        for y in 0..self.board.height() {
            for x in 0..self.board.width() {
                if self.board[(x, y)] {
                    shared_state.decay_board[(x, y)] =
                        DecayElement::new_with_time('█', current_time);
                    self.board[(x, y)] = false;
                }
            }
        }
    }
}

impl Component for FloodFillComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        match event {
            Event::Resize(width, height) => {
                self.board.resize_discard(width as usize, height as usize);
                self.visited.resize_discard(width as usize, height as usize);
            }
            Event::Mouse(event) => {
                let mut new_mouse_info = self.last_mouse_info;
                MouseTrackerComponent::fill_mouse_info(event, &mut new_mouse_info);
                MouseTrackerComponent::smooth_two_updates(
                    self.last_mouse_info,
                    new_mouse_info,
                    |mouse_info| {
                        if mouse_info.right_mouse_down {
                            let (x, y) = mouse_info.last_mouse_pos;
                            self.board.set(x, y, true);
                        }
                    },
                );
                self.last_mouse_info = new_mouse_info;
                // if self.last_mouse_info.right_mouse_down {
                //     let (x, y) = self.last_mouse_info.last_mouse_pos;
                //     self.board.set(x, y, true);
                // }
                if self.last_mouse_info.right_mouse_down {
                    self.received_down_event_this_frame = true;
                }
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        if self.received_down_event_this_frame {
            // We must have received some mouse events since last release.
            self.has_content = true;
            // Tracking and updating of board state happens on event handling as to not skip any
            // pixels at low frame rates (i.e., being able to update more than one pixel per frame).
            // For performance reasons, flood fill still only happens once per frame.
            if self.flood_fill() {
                // TODO: Print debug message
            }
        }
        // if we are in released state and have unprocessed content, process
        if !shared_state.mouse_info.right_mouse_down && self.has_content {
            self.has_content = false;
            self.on_release(shared_state, update_info.current_time);
        }
        self.received_down_event_this_frame = false;
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        for y in 0..self.board.height() {
            for x in 0..self.board.width() {
                if self.board[(x, y)] {
                    "█".render(&mut renderer, x, y, depth_base);
                }
            }
        }
    }
}

pub struct SimpleDrawComponent {
    last_mouse_info: MouseInfo,
    // small queue for multiple events in one frame
    draw_queue: SmallVec<[(u16, u16); 20]>,
}

impl SimpleDrawComponent {
    pub fn new() -> Self {
        Self {
            last_mouse_info: MouseInfo::default(),
            draw_queue: SmallVec::new(),
        }
    }
}

impl Component for SimpleDrawComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        if let Event::Mouse(event) = event {
            let mut new_mouse_info = self.last_mouse_info;
            MouseTrackerComponent::fill_mouse_info(event, &mut new_mouse_info);
            MouseTrackerComponent::smooth_two_updates(
                self.last_mouse_info,
                new_mouse_info,
                |mouse_info| {
                    if mouse_info.left_mouse_down {
                        let x = mouse_info.last_mouse_pos.0 as u16;
                        let y = mouse_info.last_mouse_pos.1 as u16;
                        self.draw_queue.push((x, y));
                    }
                },
            );
            self.last_mouse_info = new_mouse_info;
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        for (x, y) in self.draw_queue.drain(..) {
            shared_state.decay_board[(x as usize, y as usize)] =
                DecayElement::new_with_time('█', update_info.current_time);
        }
        // also current pixel, in case we're holding the button and not moving
        if self.last_mouse_info.left_mouse_down {
            let (x, y) = self.last_mouse_info.last_mouse_pos;
            shared_state.decay_board[(x, y)] =
                DecayElement::new_with_time('█', update_info.current_time);
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct DecayElement {
    pub c: char,
    pub inception_time: Option<Instant>,
}

impl DecayElement {
    pub fn new(c: char) -> Self {
        Self {
            c,
            inception_time: None,
        }
    }

    pub fn new_with_time(c: char, inception_time: Instant) -> Self {
        Self {
            c,
            inception_time: Some(inception_time),
        }
    }
}

pub struct DecayComponent {}

impl DecayComponent {
    const DECAY_TIME: Duration = Duration::from_millis(500);
    const DECAY_STAGES: [char; 4] = ['█', '▓', '▒', '░'];

    pub fn new() -> Self {
        Self {}
    }

    fn release(&mut self, physics_board: &mut PhysicsBoard, x: usize, y: usize) {
        physics_board.add_entity(x, y, '░');
    }
}

impl Component for DecayComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let current = update_info.current_time;
        let nanos_per_stage = Self::DECAY_TIME.as_nanos() / Self::DECAY_STAGES.len() as u128;
        for (x, y, element) in shared_state.decay_board.iter_mut() {
            if let Some(inception_time) = element.inception_time {
                let elapsed = current.saturating_duration_since(inception_time).as_nanos();
                let stage = (elapsed / nanos_per_stage) as usize;
                if stage < Self::DECAY_STAGES.len() {
                    element.c = Self::DECAY_STAGES[stage];
                } else {
                    element.c = ' ';
                    element.inception_time = None;
                    self.release(&mut shared_state.physics_board, x, y);
                }
            }
        }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        for (x, y, element) in shared_state.decay_board.iter() {
            // we generally skip ' '
            if element.c != ' ' {
                element.c.render(&mut renderer, x, y, depth_base);
            }
        }
    }
}

// This just runs the physics simulation contained in the shared state.
pub struct PhysicsComponent {}

impl PhysicsComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for PhysicsComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        shared_state.collision_board.clear();
        let dt = update_info.dt;
        shared_state
            .physics_board
            .update(dt, shared_state.decay_board.height(), |s| {
                // TODO: debug print
            });

        for col in shared_state.physics_board.board.iter() {
            for entity in col {
                let x = entity.x.floor() as usize;
                let y = entity.y.floor() as usize;
                if x >= shared_state.display_info.width() || y >= shared_state.display_info.height()
                {
                    // TODO: debug error
                    continue;
                }
                shared_state.collision_board[(x, y)] = true;
            }
        }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let width = shared_state.display_info.width();
        let height = shared_state.display_info.height();
        for col in shared_state.physics_board.board.iter() {
            for entity in col {
                let x = entity.x.floor() as usize;
                let y = entity.y.floor() as usize;
                if x < width && y < height {
                    let vel = entity.vel_y.abs();
                    // map vel from 0 to height to 0 to 255
                    let vel = (vel / height as f64 * 255.0) as u8;
                    let color = [255, 255 - vel, 255];
                    renderer.render_pixel(
                        x,
                        y,
                        Pixel::new(entity.c), /*.with_color(color)*/
                        depth_base,
                    );
                }
            }
        }

        // for (x, y, element) in shared_state.collision_board.iter() {
        //     if *element {
        //         ".".render(&mut renderer, x, y, 100000);
        //     }
        // }
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
            Event::Key(ke) => {
                // assert_eq!(ke.kind, crossterm::event::KeyEventKind::Press);
                if let Some(count) = self.pressed_keys.get_mut(&ke.code) {
                    *count += 1;
                } else {
                    // UB if we insert more than 16 keys
                    if self.pressed_keys.len() < 16 {
                        self.pressed_keys.insert(ke.code, 1);
                    }
                }
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        std::mem::swap(&mut shared_state.pressed_keys, &mut self.pressed_keys);
        self.pressed_keys.clear();
    }
}

struct Bullet {
    x: f64,
    y: f64,
    vel_x: f64,
    vel_y: f64,
}

impl Bullet {
    // returns whether the bullet should be deleted or not
    fn update(
        &mut self,
        dt: f64,
        height: usize,
        width: usize,
        shared_state: &mut SharedState,
    ) -> bool {
        self.x += self.vel_x * dt;
        self.y += self.vel_y * dt;
        if self.x < 0.0 || self.x >= width as f64 || self.y < 0.0 || self.y >= height as f64 {
            return true;
        }
        if ForceApplyComponent::apply_force(
            shared_state,
            self.x.floor() as usize,
            self.y.floor() as usize,
            -20.0,
            1,
        ) {
            return true;
        }
        false
    }
}

pub struct PlayerComponent {
    x: f64,
    y: f64,
    x_vel: f64,
    y_vel: f64,
    sprite: Sprite<3, 2>,
    dead_sprite: Sprite<5, 1>,
    bullets: Vec<Bullet>,
    bullet_char: char,
    dead: bool,
    max_height_since_last_ground_touch: f64,
}

impl PlayerComponent {
    const DEATH_HEIGHT: f64 = 25.0;

    pub fn new(x: usize, y: usize) -> Self {
        Self {
            x: x as f64,
            y: y as f64,
            x_vel: 0.0,
            y_vel: 0.0,
            sprite: Sprite::new([['▁', '▄', '▁'], ['▗', '▀', '▖']], 1, 1),
            dead_sprite: Sprite::new([['▂', '▆', '▆', ' ', '▖']], 2, 0),
            bullets: vec![],
            bullet_char: '●',
            dead: false,
            max_height_since_last_ground_touch: y as f64,
        }
    }

    fn spawn_bullet(&mut self, shared_state: &mut SharedState, vel_x: f64, vel_y: f64) {
        // Because the aspect ratio of pixels is 2:1, we divide vel_y by 2 to get the same
        // visual velocity as x.
        let vel_y = vel_y / 2.0;

        let mut bullet = Bullet {
            x: self.x as f64,
            y: self.y as f64 - 1.0, // center is between legs, want to shoot from chest
            vel_x,
            vel_y,
        };
        if vel_x < 0.0 {
            bullet.x -= 1.0; // only 1, as we're updating right after spawning, which with floor leads to going down.
        } else if vel_x > 0.0 {
            bullet.x += 2.0;
        }
        if vel_y < 0.0 {
            bullet.y -= 0.0;
        } else if vel_y > 0.0 {
            bullet.y += 2.0;
        }
        self.bullets.push(bullet);
    }
}

impl Component for PlayerComponent {
    // fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
    //     match event {
    //         Event::Key(ke) => {
    //             assert_eq!(ke.kind, crossterm::event::KeyEventKind::Press);
    //             match ke.code {
    //                 KeyCode::Char('w' | 'W') => {
    //                     self.y = self.y.saturating_sub(1);
    //                 }
    //                 KeyCode::Char('s' | 'S') => {
    //                     self.y = self.y.saturating_add(1);
    //                 }
    //                 KeyCode::Char(c@('a' | 'A')) => {
    //                     self.x = self.x.saturating_sub(1 + c.is_ascii_uppercase() as usize);
    //                 }
    //                 KeyCode::Char(c@('d' | 'D')) => {
    //                     self.x = self.x.saturating_add(1 + c.is_ascii_uppercase() as usize);
    //                 }
    //                 _ => {}
    //             }
    //         }
    //         _ => {}
    //     }
    //     None
    // }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        // Bullet spawning
        let bullet_speed = 12.0;
        if shared_state.pressed_keys.contains_key(&KeyCode::Left) {
            let mut speed_mod = 0.0;
            if self.x_vel < 0.0 {
                speed_mod = self.x_vel;
            }
            self.spawn_bullet(shared_state, -bullet_speed + speed_mod, 0.0);
        }
        if shared_state.pressed_keys.contains_key(&KeyCode::Right) {
            let mut speed_mod = 0.0;
            if self.x_vel > 0.0 {
                speed_mod = self.x_vel;
            }
            self.spawn_bullet(shared_state, bullet_speed + speed_mod, 0.0);
        }
        if shared_state.pressed_keys.contains_key(&KeyCode::Up) {
            let mut speed_mod = 0.0;
            if self.y_vel < 0.0 {
                speed_mod = self.y_vel;
            }
            self.spawn_bullet(shared_state, 0.0, -bullet_speed + speed_mod);
        }
        if shared_state.pressed_keys.contains_key(&KeyCode::Down) {
            let mut speed_mod = 0.0;
            if self.y_vel > 0.0 {
                speed_mod = self.y_vel;
            }
            self.spawn_bullet(shared_state, 0.0, bullet_speed + speed_mod);
        }
        // if shared_state.pressed_keys.contains_key(&KeyCode::Char('k')) {
        //     self.dead = true;
        // }

        let dt = update_info.dt;
        self.bullets.retain_mut(|bullet| {
            let delete = bullet.update(
                dt,
                shared_state.display_info.height(),
                shared_state.display_info.width(),
                shared_state,
            );
            !delete
        });

        // Player inputs
        if shared_state.pressed_keys.contains_key(&KeyCode::Char('a')) {
            if self.x_vel > 0.0 {
                self.x_vel = 0.0;
            } else if self.x_vel == 0.0 {
                self.x_vel = -10.0;
            } else {
                self.x_vel = -10.0;
            }
        } else if shared_state.pressed_keys.contains_key(&KeyCode::Char('d')) {
            if self.x_vel < 0.0 {
                self.x_vel = 0.0;
            } else if self.x_vel == 0.0 {
                self.x_vel = 10.0;
            } else {
                self.x_vel = 10.0;
            }
        }
        if shared_state.pressed_keys.contains_key(&KeyCode::Char('A')) {
            if self.x_vel > 0.0 {
                self.x_vel = 0.0;
            } else if self.x_vel == 0.0 {
                self.x_vel = -20.0;
            } else {
                self.x_vel = -20.0
            }
        } else if shared_state.pressed_keys.contains_key(&KeyCode::Char('D')) {
            if self.x_vel < 0.0 {
                self.x_vel = 0.0;
            } else if self.x_vel == 0.0 {
                self.x_vel = 20.0;
            } else {
                self.x_vel = 20.0;
            }
        }

        // Player physics
        let height = shared_state.display_info.height() as f64;
        let width = shared_state.display_info.width() as f64;

        let gravity = 40.0;

        self.y_vel += gravity * dt;
        self.x += self.x_vel * dt;

        let mut bottom_wall = height;
        let mut left_wall = 0.0f64;
        let mut right_wall = width;
        let mut left_idx = None;
        let mut right_idx = None;
        let step_size = 1;

        // find a physics entity below us
        let mut x_u = self.x.floor() as usize;
        let mut y_u = self.y.floor() as usize;
        if y_u >= height as usize {
            y_u = height as usize - 1;
        }

        {
            // Check left
            let x = x_u as i32 - 1;
            for y in (y_u - 1)..=y_u {
                for x in ((x - 4)..=x).rev() {
                    if x < 0 || x >= width as i32 {
                        break;
                    }
                    if shared_state.collision_board[(x as usize, y)] {
                        if left_wall < x as f64 + 1.0 {
                            left_idx = Some(x as usize);
                            left_wall = x as f64 + 1.0; // plus 1.0 because we define collision on <x differently?
                        }
                        break;
                    }
                }
            }
        }
        {
            // Check right
            let x = x_u as i32 + 1;
            for y in (y_u - 1)..=y_u {
                for x in x..=(x + 4) {
                    if x < 0 || x >= width as i32 {
                        break;
                    }
                    if shared_state.collision_board[(x as usize, y)] {
                        if right_wall > x as f64 {
                            right_idx = Some(x as usize);
                            right_wall = x as f64;
                        }
                        break;
                    }
                }
            }
        }

        // -1.0 etc to account for size of sprite
        if self.x - 1.0 < left_wall {
            // Check if we can do a step
            // initialize false because if there is no left_idx, we can't do a step
            let mut do_step = false;
            if let Some(left_idx) = left_idx {
                for base_check in 0..step_size {
                    // if there is one, we assume true
                    do_step = true;
                    let check_y = self.y.floor() as usize - 1 - base_check;
                    // TODO: saturation
                    for y in (check_y - 1)..=check_y {
                        if shared_state.collision_board[(left_idx, y)] {
                            do_step = false;
                            break;
                        }
                    }
                    if do_step {
                        break;
                    }
                }
            }
            if !do_step {
                self.x = left_wall + 1.0;
            }
            // self.x_vel = 0.0;
        } else if self.x + 1.0 >= right_wall {
            // Check if we can do a step
            let mut do_step = false;
            if let Some(right_idx) = right_idx {
                for base_check in 0..step_size {
                    do_step = true;
                    let check_y = self.y.floor() as usize - 1 - base_check;
                    for y in (check_y - 1)..=check_y {
                        if shared_state.collision_board[(right_idx, y)] {
                            do_step = false;
                            break;
                        }
                    }
                    if do_step {
                        break;
                    }
                }
            }
            if !do_step {
                self.x = right_wall - 2.0;
            }

            // self.x_vel = 0.0;
        }

        // need to update for bottom checking, since x checking can clamp x and change the bottom check result
        let mut x_u = self.x.floor() as usize;
        // and only update y here, because otherwise x checking will think we're inside the floor block
        self.y += self.y_vel * dt;
        let mut y_u = self.y.floor() as usize;
        if y_u >= height as usize {
            y_u = height as usize - 1;
        }

        {
            // Check below
            let x = x_u as i32;
            let y = y_u;

            // TODO: should be dynamic due to sprite size
            for x in (x - 1)..=(x + 1) {
                if x < 0 || x >= width as i32 {
                    continue;
                }
                for y in y..(height as usize).min(y + 4) as usize {
                    if shared_state.collision_board[(x as usize, y)] {
                        bottom_wall = bottom_wall.min(y as f64);
                        break;
                    }
                }
            }
        }

        // TODO: sprite size should be taken into account for top wall checking
        if self.y < 0.0 {
            self.y = 0.0;
            self.y_vel = 0.0;
        } else if self.y >= bottom_wall {
            self.y = bottom_wall - 1.0;
            // if we're going up, don't douch the jump velocity.
            if self.y_vel >= 0.0 {
                self.y_vel = 0.0;
            }
        }

        let grounded = self.y >= bottom_wall - 1.2;
        if !grounded {
            self.max_height_since_last_ground_touch =
                self.max_height_since_last_ground_touch.min(self.y);
        } else {
            if self.y - self.max_height_since_last_ground_touch > Self::DEATH_HEIGHT {
                self.dead = true;
            }
            self.max_height_since_last_ground_touch = self.y;
        }

        // Now jump input since we need grounded information
        if shared_state.pressed_keys.contains_key(&KeyCode::Char(' ')) {
            if grounded {
                self.y_vel = -20.0;
            }
        }
        shared_state.debug_info.player_y = self.y;
        shared_state.debug_info.player_x = self.x;
        shared_state.debug_info.left_wall = left_wall;
        shared_state.debug_info.bottom_wall = bottom_wall;
        shared_state.debug_info.y_vel = self.y_vel;
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        if self.dead {
            self.dead_sprite.render(
                &mut renderer,
                self.x.floor() as usize,
                self.y.floor() as usize,
                depth_base,
            );
        } else {
            self.sprite.render(
                &mut renderer,
                self.x.floor() as usize,
                self.y.floor() as usize,
                depth_base,
            );
        }

        // render bullets
        for bullet in &self.bullets {
            let x = bullet.x.floor() as usize;
            let y = bullet.y.floor() as usize;
            let pixel = Pixel::new(self.bullet_char).with_color([200, 200, 100]);
            renderer.render_pixel(x, y, pixel, depth_base);
        }
    }
}

pub struct ClearComponent;

impl Component for ClearComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        if shared_state.pressed_keys.contains_key(&KeyCode::Char('c')) {
            shared_state.decay_board.clear();
            shared_state.physics_board.clear();
        }
    }
}

pub struct ForceApplyComponent;

impl ForceApplyComponent {
    /// Returns whether an explosion happened or not.
    fn apply_force(
        shared_state: &mut SharedState,
        x: usize,
        y: usize,
        vel_change_y: f64,
        count: usize,
    ) -> bool {
        if let Ok(base_idx) = shared_state.physics_board.board[x]
            .binary_search_by(|entity| (entity.y.floor() as usize).cmp(&y))
        {
            let first_entity = &mut shared_state.physics_board.board[x][base_idx];
            first_entity.vel_y += vel_change_y;
            let first_y = first_entity.y;

            // all other ones must be above, and smaller indices
            for i in 1..count {
                if i > base_idx {
                    // reached the end.
                    break;
                }
                let idx = base_idx - i;
                // must check that it is within the expected range of first_entity (only stacked entities should experience the same explosion)
                let entity = &mut shared_state.physics_board.board[x][idx];
                if entity.y < first_y - i as f64 {
                    break;
                }
                entity.vel_y += vel_change_y;
            }
            true
        } else {
            false
        }
    }
}

impl Component for ForceApplyComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        if shared_state.pressed_keys.contains_key(&KeyCode::Char('f')) {
            let (x, y) = shared_state.mouse_info.last_mouse_pos;
            ForceApplyComponent::apply_force(shared_state, x, y, -100.0, 10);
        }
    }
}
