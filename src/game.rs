use std::io;
use std::io::{Stdout, Write};
use std::ops::{Index, IndexMut};
use std::time::{Duration, Instant};
use crossterm::event::{Event, MouseEvent, MouseEventKind};
use crossterm::queue;
mod renderer;
mod render;
pub mod components;
mod display;

pub use renderer::*;
pub use render::*;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Pixel {
    c: char,
    color: Option<[u8; 3]>,
}

impl Pixel {
    pub fn new(c: char) -> Self {
        Self { c, color: None }
    }

    pub fn with_color(self, color: [u8; 3]) -> Self {
        Self { color: Some(color), c: self.c }
    }
}

impl Default for Pixel {
    fn default() -> Self {
        Self { c: ' ', color: None }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UpdateInfo {
    last_time: Instant,
    current_time: Instant,
}

pub enum BreakingAction {
    Quit,
}

#[derive(Clone, Copy, Debug, Default)]
struct MouseInfo {
    // x, y
    last_mouse_pos: (usize, usize),
    left_mouse_down: bool,
    right_mouse_down: bool,
    middle_mouse_down: bool
}

#[derive(Debug, Default)]
pub struct SharedState {
    mouse_info: MouseInfo,
}


/// A game component that can listen to events, perform logic, and render itself.
pub trait Component {
    /// Called when an event is received. This could happen multiple times per frame.
    fn on_event(&mut self, event: Event) -> Option<BreakingAction> { None }
    /// Called once per frame to update the component's state.
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {}
    /// Called once per frame to render the component. Each component has 100 depth available
    /// starting from the base.
    fn render(&self, renderer: &mut dyn Renderer, depth_base: i32) {}
}


pub struct Game<W: Write> {
    display_renderer: DisplayRenderer<W>,
    components: Vec<Box<dyn Component>>,
    shared_state: SharedState,
}

impl<W: Write> Game<W> {
    const TARGET_FPS: f64 = 150.0;

    pub fn new(sink: W) -> Self {
        let (width, height) = crossterm::terminal::size().unwrap();
        let display_renderer = DisplayRenderer::new_with_sink(width as usize, height as usize, sink);
        Self {
            display_renderer,
            components: Vec::new(),
            shared_state: SharedState::default(),

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
    
    pub fn add_component_init(&mut self, init_fn: impl FnOnce(usize, usize) -> Box<dyn Component>) {
        self.components.insert(0, init_fn(self.width(), self.height()));
    }

    pub fn run(&mut self) -> io::Result<()> {
        // Setup phase
        self.setup()?;

        let nanos_per_frame = (1.0 / Self::TARGET_FPS * 1_000_000_000.0) as u64;

        // Game loop
        let mut last_frame = Instant::now();
        let mut now = Instant::now();
        loop {

            let update_info = UpdateInfo {
                last_time: last_frame,
                current_time: now,
            };

            // Allocate half the time for events, rest for update and render
            let event_nanos = nanos_per_frame / 2;
            if let Some(action) = self.consume_events(Duration::from_nanos(event_nanos))? {
                match action {
                    BreakingAction::Quit => break,
                }
            }

            self.update(update_info);
            self.render()?;
            self.display_renderer.reset_screen();


            // Sleep until the next frame

            let current = Instant::now();
            let this_frame_so_far = current.duration_since(now);
            let remaining_time = Duration::from_nanos(nanos_per_frame).saturating_sub(this_frame_so_far);
            std::thread::sleep(remaining_time);
            last_frame = now;
            now = Instant::now();
        }


        Ok(())
    }

    fn consume_events(&mut self, max_duration: Duration) -> io::Result<Option<BreakingAction>> {
        let mut timeout = Timeout::new(max_duration);

        let poll_duration = Duration::from_nanos(1);

        while !timeout.is_elapsed() {
            if crossterm::event::poll(timeout.leftover().min(poll_duration))? {
                let event = crossterm::event::read()?;
                if let Some(action) = self.on_event(event) {
                    return Ok(Some(action));
                }
            } else {
                break;
            }
        }

        Ok(None)
    }

    fn on_event(&mut self, event: Event) -> Option<BreakingAction> {
        for component in self.components.iter_mut() {
            if let Some(action) = component.on_event(event.clone()) {
                return Some(action);
            }
        }

        self.on_event_game(event);

        None
    }

    fn on_event_game(&mut self, event: Event) -> Option<BreakingAction> {
        match event {
            Event::Resize(width, height) => {
                self.display_renderer.resize_discard(width as usize, height as usize);
            }
            _ => {}
        }

        None
    }

    fn update(&mut self, update_info: UpdateInfo) {
        for component in self.components.iter_mut() {
            component.update(update_info, &mut self.shared_state);
        }
    }

    fn render(&mut self) -> io::Result<()> {
        for component in self.components.iter() {
            component.render(&mut self.display_renderer, 10);
        }
        self.display_renderer.flush()
    }

    fn setup(&mut self) -> io::Result<()> {

        Ok(())
    }
}

struct Timeout {
    end: Instant,
}

impl Timeout {
    fn new(duration: Duration) -> Self {
        Self { end: Instant::now() + duration }
    }

    fn leftover(&self) -> Duration {
        self.end.saturating_duration_since(Instant::now())
    }

    fn is_elapsed(&self) -> bool {
        self.leftover() == Duration::from_secs(0)
    }
}




