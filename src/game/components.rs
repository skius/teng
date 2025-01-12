use crate::game::display::Display;
use crate::game::{
    BreakingAction, Component, MouseInfo, Render, Renderer, SharedState, UpdateInfo,
};
use crossterm::event::{Event, KeyCode, KeyEvent, MouseEvent, MouseEventKind};
use std::time::{Duration, Instant};
use smallvec::SmallVec;

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
        }
    }
}

impl DebugInfoComponent {
    const MAX_FRAMETIME_WINDOW: Duration = Duration::from_secs(5);
    const FPS_UPDATE_INTERVAL: Duration = Duration::from_millis(200);
}

impl Component for DebugInfoComponent {
    fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
        self.num_events += 1;
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        self.num_update_calls += 1;
        let UpdateInfo {
            last_time,
            current_time,
            ..
        } = update_info;

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
            self.frames_since_last_fps = 0;
            self.last_fps_time = current_time;
        }
        self.target_fps = shared_state.target_fps;
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let mut y = 0;
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
        // format!("Events: {}", self.num_events).render(&mut renderer, 0, y, depth_base);
        // y += 1;
        // format!("Frames since last FPS: {}", self.frames_since_last_fps).render(&mut renderer, 0, y, depth_base);
        // y += 1;
        // format!("Update calls: {}", self.num_update_calls).render(&mut renderer, 0, y, depth_base);
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
    fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
        match event {
            Event::Key(KeyEvent {
                code: KeyCode::Char('l'),
                ..
            }) => {
                self.locked = !self.locked;
            }
            Event::Mouse(me) => {
                match me.kind {
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
                }
            }
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
}

impl MouseTrackerComponent {
    pub fn new() -> Self {
        Self {
            last_mouse_info: MouseInfo::default(),
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
    pub fn smooth_two_updates(
        first: MouseInfo,
        second: MouseInfo,
        mut f: impl FnMut(MouseInfo),
    ) {
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
    fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
        if let Event::Mouse(event) = event {
            Self::fill_mouse_info(event, &mut self.last_mouse_info);
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        shared_state.mouse_info = self.last_mouse_info;
    }
}

pub struct QuitterComponent;

impl Component for QuitterComponent {
    fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
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
}

impl FloodFillComponent {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            has_content: false,
            board: Display::new(width, height, false),
            visited: Display::new(width, height, false),
            stack: vec![],
            last_mouse_info: MouseInfo::default(),
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
                    shared_state.decay_board[(x, y)] = DecayElement::new_with_time('█', current_time);
                    self.board[(x, y)] = false;
                }
            }
        }
    }
}

impl Component for FloodFillComponent {
    fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
        match event {
            Event::Resize(width, height) => {
                self.board.resize_discard(width as usize, height as usize);
                self.visited.resize_discard(width as usize, height as usize);
            }
            Event::Mouse(event) => {
                // TODO: last_mouse_info could be default initialized if there has been no event previously.
                // so we should turn it into option and only smooth if there is a previous one (or just take the new info twice)
                let mut new_mouse_info = self.last_mouse_info;
                MouseTrackerComponent::fill_mouse_info(event, &mut new_mouse_info);
                MouseTrackerComponent::smooth_two_updates(self.last_mouse_info, new_mouse_info, |mouse_info| {
                    if mouse_info.right_mouse_down {
                        let (x, y) = mouse_info.last_mouse_pos;
                        self.board.set(x, y, true);
                    }
                });
                self.last_mouse_info = new_mouse_info;
                // if self.last_mouse_info.right_mouse_down {
                //     let (x, y) = self.last_mouse_info.last_mouse_pos;
                //     self.board.set(x, y, true);
                // }
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        // TODO: if we start a frame with no_content, then receive a mouse down, then mouse up event, and only then call update(),
        // we will not process. Instead we should start each frame with "received event: false" and set it to true when we receive an event.
        // then in update() we can check if we received an event since last update and if so, process the board.
        // in particular it should be "received mouse down event" or something, because we want to on_release() if we received ONLY a mouse up event.
        if shared_state.mouse_info.right_mouse_down {
            // We must have received some mouse events since last release.
            self.has_content = true;
            // Tracking and updating of board state happens on event handling as to not skip any
            // pixels at low frame rates (i.e., being able to update more than one pixel per frame).
            // For performance reasons, flood fill still only happens once per frame.
            if self.flood_fill() {
                // Print debug message
            }
        } else {
            if self.has_content {
                self.has_content = false;
                self.on_release(shared_state, update_info.current_time);
            }
        }
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
    fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
        if let Event::Mouse(event) = event {
            // TODO: same issue as in floodfill
            let mut new_mouse_info = self.last_mouse_info;
            MouseTrackerComponent::fill_mouse_info(event, &mut new_mouse_info);
            MouseTrackerComponent::smooth_two_updates(self.last_mouse_info, new_mouse_info, |mouse_info| {
                if mouse_info.left_mouse_down {
                    let x = mouse_info.last_mouse_pos.0 as u16;
                    let y = mouse_info.last_mouse_pos.1 as u16;
                    self.draw_queue.push((x, y));
                }
            });
            self.last_mouse_info = new_mouse_info;
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        for (x, y) in self.draw_queue.drain(..) {
            shared_state.decay_board[(x as usize, y as usize)] = DecayElement::new_with_time('█', update_info.current_time);
        }
        // also current pixel, in case we're holding the button and not moving
        if self.last_mouse_info.left_mouse_down {
            let (x, y) = self.last_mouse_info.last_mouse_pos;
            shared_state.decay_board[(x, y)] = DecayElement::new_with_time('█', update_info.current_time);
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
        Self { c, inception_time: None }
    }

    pub fn new_with_time(c: char, inception_time: Instant) -> Self {
        Self { c, inception_time: Some(inception_time) }
    }
}

pub struct DecayComponent {

}

impl DecayComponent {
    const DECAY_TIME: Duration = Duration::from_millis(500);
    const DECAY_STAGES: [char; 4] = ['█', '▓', '▒', '░'];

    pub fn new() -> Self {
        Self {}
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