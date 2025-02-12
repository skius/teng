use anymap::AnyMap;
use crossterm::event::{Event, KeyCode, MouseEvent, MouseEventKind};
use crossterm::queue;
use smallvec::SmallVec;
use std::any::Any;
use std::collections::HashSet;
use std::io;
use std::io::{Stdout, Write};
use std::ops::{Index, IndexMut};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

pub mod components;
pub mod display;
mod render;
mod renderer;
pub mod seeds;
pub mod util;

use crate::game::components::incremental::ui::UiBarComponent;
use crate::game::components::{DebugInfo, DebugInfoComponent, MouseEvents, PressedKeys};
use crate::game::display::Display;
use crate::game::Color::Transparent;
pub use render::*;
pub use renderer::*;

#[derive(Clone, Copy, Debug, PartialEq, Default)]
pub enum Color {
    #[default]
    Default,
    /// A transparent color does not overwrite the existing color
    /// If there is no other color, it will behave the same as default.
    Transparent,
    Rgb([u8; 3]),
}

impl Color {
    pub fn unwrap_or(self, other: [u8; 3]) -> [u8; 3] {
        match self {
            Color::Default => other,
            Color::Transparent => other,
            Color::Rgb(c) => c,
        }
    }

    pub fn is_solid(self) -> bool {
        match self {
            Color::Default => true,
            Color::Transparent => false,
            Color::Rgb(_) => true,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixel {
    c: char,
    color: Color,
    bg_color: Color,
}

impl Pixel {
    pub fn new(c: char) -> Self {
        Self {
            c,
            color: Color::Default,
            bg_color: Color::Transparent,
        }
    }

    pub fn transparent() -> Self {
        Self {
            c: ' ',
            color: Transparent,
            bg_color: Transparent,
        }
    }

    pub fn with_color(self, color: [u8; 3]) -> Self {
        Self {
            color: Color::Rgb(color),
            c: self.c,
            bg_color: self.bg_color,
        }
    }

    pub fn with_bg_color(self, bg_color: [u8; 3]) -> Self {
        Self {
            bg_color: Color::Rgb(bg_color),
            c: self.c,
            color: self.color,
        }
    }

    pub fn put_over(self, other: Pixel) -> Self {
        // works with priorities: transparent < default < color
        // and other < self

        let mut new_pixel = self;
        if new_pixel.color == Transparent {
            new_pixel.color = other.color;
            new_pixel.c = other.c;
        }
        if new_pixel.bg_color == Transparent {
            new_pixel.bg_color = other.bg_color;
        }
        new_pixel
    }
}

impl Default for Pixel {
    fn default() -> Self {
        Self {
            c: ' ',
            color: Color::Default,
            bg_color: Color::Default,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UpdateInfo {
    last_time: Instant,
    current_time: Instant,
    dt: f64,
    // the dt that the computations took without sleeping
    actual_dt: f64,
}

pub enum BreakingAction {
    Quit,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseInfo {
    // x, y
    last_mouse_pos: (usize, usize),
    left_mouse_down: bool,
    right_mouse_down: bool,
    middle_mouse_down: bool,
}

pub struct DisplayInfo {
    _height: usize,
    _width: usize,
}

impl DisplayInfo {
    pub(super) fn new(width: usize, height: usize) -> Self {
        Self {
            _width: width,
            _height: height,
        }
    }

    pub fn width(&self) -> usize {
        self._width
    }

    pub fn height(&self) -> usize {
        self._height
    }
}

pub struct DebugMessage {
    message: String,
    expiry_time: Instant,
}

impl DebugMessage {
    pub fn new(message: String, expiry_time: Instant) -> Self {
        Self {
            message,
            expiry_time,
        }
    }

    pub fn new_3s(message: impl Into<String>) -> Self {
        Self::new(message.into(), Instant::now() + Duration::from_secs(3))
    }
}

#[derive(Default, Debug, PartialEq)]
pub struct MousePressedInfo {
    left: bool,
    right: bool,
    middle: bool,
}

pub struct SharedState {
    mouse_info: MouseInfo,
    mouse_pressed: MousePressedInfo,
    mouse_events: MouseEvents,
    target_fps: Option<f64>,
    display_info: DisplayInfo,
    pressed_keys: PressedKeys,
    debounced_down_keys: HashSet<KeyCode>,
    debug_info: DebugInfo,
    debug_messages: SmallVec<[DebugMessage; 16]>,
    extensions: AnyMap,
    components_to_add: Vec<Box<dyn Component>>,
    fake_events_for_next_frame: Vec<Event>,
}

impl SharedState {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            mouse_info: MouseInfo::default(),
            mouse_pressed: MousePressedInfo::default(),
            mouse_events: MouseEvents::new(),
            target_fps: Some(150.0),
            display_info: DisplayInfo::new(width, height),
            pressed_keys: PressedKeys::new(),
            debounced_down_keys: HashSet::new(),
            debug_info: DebugInfo::new(),
            debug_messages: SmallVec::new(),
            extensions: AnyMap::new(),
            components_to_add: Vec::new(),
            fake_events_for_next_frame: Vec::new(),
        }
    }

    pub fn resize(&mut self, width: usize, height: usize) {
        self.display_info = DisplayInfo::new(width, height);
    }
}

pub struct SetupInfo {
    pub width: usize,
    pub height: usize,
}

/// A game component that can listen to events, perform logic, and render itself.
pub trait Component: Any {
    /// Called in the very beginning. Useful to initialize more components or extension states.
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {}
    /// Called to determine if this component is active. If not, none of the other methods will be invoked.
    fn is_active(&self, shared_state: &SharedState) -> bool {
        true
    }
    /// Called when the terminal is resized.
    /// Note that Resize events are also passed to on_event, so this is not strictly necessary.
    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState) {}
    /// Called when the game exits. Useful for cleanup.
    fn on_quit(&mut self, shared_state: &mut SharedState) {}
    /// Called when an event is received. This could happen multiple times per frame. Runs before update.
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        None
    }
    /// Called once per frame to update the component's state. Runs after the frame's events have been processed.
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {}
    /// Called once per frame to render the component. Each component has 100 depth available
    /// starting from the base.
    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {}
}

pub struct Game<W: Write> {
    display_renderer: DisplayRenderer<W>,
    components: Vec<Box<dyn Component>>,
    shared_state: SharedState,
    event_read_thread_handle: Option<std::thread::JoinHandle<()>>,
    event_reader: Receiver<Event>,
    event_read_stop_signal: std::sync::mpsc::Sender<()>,
}

impl<W: Write> Game<W> {
    pub fn new(sink: W) -> Self {
        let (width, height) = crossterm::terminal::size().unwrap();
        let width = width as usize;
        let height = height as usize;
        let display_renderer = DisplayRenderer::new_with_sink(width, height, sink);

        let (event_writer, event_reader) = std::sync::mpsc::channel();
        let (event_read_stop_signal, event_read_stop_receiver) = std::sync::mpsc::channel();

        let event_read_thread_handle = std::thread::spawn(move || loop {
            if crossterm::event::poll(Duration::from_millis(10)).unwrap() {
                if let Ok(event) = crossterm::event::read() {
                    event_writer.send(event).unwrap();
                }
            }
            if let Ok(_) = event_read_stop_receiver.try_recv() {
                break;
            }
        });

        Self {
            display_renderer,
            components: Vec::new(),
            shared_state: SharedState::new(width, height),
            event_read_thread_handle: Some(event_read_thread_handle),
            event_reader,
            event_read_stop_signal,
        }
    }

    fn width(&self) -> usize {
        self.display_renderer.width()
    }

    fn height(&self) -> usize {
        self.display_renderer.height()
    }

    pub fn add_component(&mut self, component: Box<dyn Component>) {
        self.components.push(component);
    }

    pub fn add_component_with(&mut self, init_fn: impl FnOnce(usize, usize) -> Box<dyn Component>) {
        self.components.push(init_fn(self.width(), self.height()));
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Setup phase
        self.setup()?;

        // Game loop
        let mut last_frame = Instant::now();
        let mut now = Instant::now();
        // how much longer the last sleep() slept than expected.
        let mut last_overhead = Duration::from_nanos(0);

        // how long the last frame's computations took
        let mut last_actual_dt = 1.0;

        // this didn't end up working nicely. Could try an adaptive approach like here:
        // https://stackoverflow.com/a/6942771
        // let mut last_nanos_per_frame = 1;
        // let mut last_frame_nanos = 0;
        // let mut last_frame_overhead = 0;
        // let mut frame_delay = 0;
        loop {
            let nanos_per_frame = if let Some(target_fps) = self.shared_state.target_fps {
                (1.0 / target_fps * 1_000_000_000.0) as u64
            } else {
                0
            };

            let update_info = UpdateInfo {
                last_time: last_frame,
                current_time: now,
                dt: (now - last_frame).as_secs_f64(),
                actual_dt: last_actual_dt,
            };

            if let Some(action) = self.consume_events()? {
                match action {
                    BreakingAction::Quit => break,
                }
            }

            self.update(update_info);
            self.render()?;
            self.display_renderer.reset_screen();

            // Sleep until the next frame
            let current = Instant::now();
            last_actual_dt = current.duration_since(now).as_secs_f64();
            let this_frame_so_far = current.duration_since(now);
            let remaining_time =
                Duration::from_nanos(nanos_per_frame).saturating_sub(this_frame_so_far);
            // sleep less by last frame's overhead
            let remaining_time = remaining_time.saturating_sub(last_overhead);
            std::thread::sleep(remaining_time);
            let new_now = Instant::now();

            let time_slept = new_now.duration_since(current);
            let overhead = time_slept.saturating_sub(remaining_time);
            last_overhead = overhead;

            // // note: 'last' from perspective of next iteration
            last_frame = now;
            now = new_now;
        }

        self.cleanup();

        Ok(())
    }

    fn consume_events(&mut self) -> io::Result<Option<BreakingAction>> {
        // let mut timeout = Timeout::new(max_duration);
        //
        // let poll_duration = Duration::from_nanos(1);
        //
        // while !timeout.is_elapsed() {
        //     if crossterm::event::poll(timeout.leftover().min(poll_duration))? {
        //         let event = crossterm::event::read()?;
        //         if let Some(action) = self.on_event(event) {
        //             return Ok(Some(action));
        //         }
        //     } else {
        //         break;
        //     }
        // }

        while let Ok(event) = self.event_reader.try_recv() {
            if let Some(action) = self.on_event(event) {
                return Ok(Some(action));
            }
        }
        
        // fake events for next frame
        let events = std::mem::replace(&mut self.shared_state.fake_events_for_next_frame, vec![]);
        for event in events {
            if let Some(action) = self.on_event(event) {
                return Ok(Some(action));
            }
        }

        Ok(None)
    }

    fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
        for component in self.components.iter_mut() {
            if !component.is_active(&self.shared_state) {
                continue;
            }
            if let Some(action) = component.on_event(event.clone(), &mut self.shared_state) {
                return Some(action);
            }
        }

        self.on_event_game(event);

        None
    }

    fn on_event_game(&mut self, event: Event) -> Option<BreakingAction> {
        match event {
            Event::Resize(width, height) => {
                self.on_resize(width as usize, height as usize);
            }
            _ => {}
        }

        None
    }

    fn on_resize(&mut self, width: usize, height: usize) {
        self.display_renderer.resize_discard(width, height);
        self.shared_state.resize(width, height);
        for component in self.components.iter_mut() {
            component.on_resize(width, height, &mut self.shared_state);
        }
    }

    fn update(&mut self, update_info: UpdateInfo) {
        for component in self.components.iter_mut() {
            if !component.is_active(&self.shared_state) {
                continue;
            }
            component.update(update_info, &mut self.shared_state);
        }
        self.update_game(update_info);
    }

    fn swap_component<C: Component>(
        &mut self,
        new: impl FnOnce(usize, usize) -> Box<dyn Component>,
    ) {
        let mut found = false;
        for idx in 0..self.components.len() {
            if (&*self.components[idx]).type_id() == std::any::TypeId::of::<C>() {
                self.components.remove(idx);
                found = true;
                break;
            }
        }
        if !found {
            self.add_component(new(self.width(), self.height()));
        }
    }

    // fn swap_component_dynamic(&mut self, new: Box<dyn Component>) {
    //     let mut found = false;
    //     for idx in 0..self.components.len() {
    //         if (&*self.components[idx]).type_id() == (&*new).type_id() {
    //             self.components.remove(idx);
    //             found = true;
    //             break;
    //         }
    //     }
    //     if !found {
    //         self.add_component(new);
    //     }
    // }

    fn update_game(&mut self, update_info: UpdateInfo) {
        if self
            .shared_state
            .pressed_keys
            .did_press_char_ignore_case('i')
        {
            self.swap_component::<DebugInfoComponent>(|width, height| {
                Box::new(DebugInfoComponent::new())
            });
        }
        for new_component in self.shared_state.components_to_add.drain(..) {
            self.components.push(new_component);
        }
    }

    fn render(&mut self) -> io::Result<()> {
        for (idx, component) in self.components.iter().enumerate() {
            if !component.is_active(&self.shared_state) {
                continue;
            }
            component.render(
                &mut self.display_renderer,
                &self.shared_state,
                idx as i32 * 100,
            );
        }
        self.display_renderer.flush()
    }

    fn setup(&mut self) -> io::Result<()> {
        let setup_info = SetupInfo {
            width: self.width(),
            height: self.height(),
        };
        let mut already_setup_components = 0;
        while already_setup_components < self.components.len() {
            let component = &mut self.components[already_setup_components];
            component.setup(&setup_info, &mut self.shared_state);
            for new_component in self.shared_state.components_to_add.drain(..) {
                self.components.push(new_component);
            }
            already_setup_components += 1;
        }
        Ok(())
    }

    fn cleanup(&mut self) {
        for component in self.components.iter_mut() {
            component.on_quit(&mut self.shared_state);
        }

        self.event_read_stop_signal.send(()).unwrap();
        self.event_read_thread_handle
            .take()
            .unwrap()
            .join()
            .unwrap();
    }
}

struct Timeout {
    end: Instant,
}

impl Timeout {
    fn new(duration: Duration) -> Self {
        Self {
            end: Instant::now() + duration,
        }
    }

    fn leftover(&self) -> Duration {
        self.end.saturating_duration_since(Instant::now())
    }

    fn is_elapsed(&self) -> bool {
        self.leftover() == Duration::from_secs(0)
    }
}
