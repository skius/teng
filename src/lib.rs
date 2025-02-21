#![doc = include_str!("../README.md")]

use anymap::AnyMap;
use crossterm::event::{DisableMouseCapture, EnableMouseCapture, Event, KeyCode};
use crossterm::terminal::{disable_raw_mode, enable_raw_mode};
use crossterm::{cursor, execute};
use smallvec::SmallVec;
use std::any::Any;
use std::collections::HashSet;
use std::io;
use std::io::{stdout, Stdout, Write};
use std::sync::mpsc::Receiver;
use std::time::{Duration, Instant};

pub mod components;
pub mod rendering;
pub mod seeds;
pub mod util;

use crate::components::debuginfo::{DebugInfo, DebugInfoComponent, DebugMessage};
use crate::components::fpslocker::FpsLockerComponent;
use crate::components::keyboard::{KeyPressRecorderComponent, PressedKeys};
use crate::components::mouse::{MouseEvents, MouseInfo, MousePressedInfo, MouseTrackerComponent};
use crate::components::quitter::QuitterComponent;
use crate::components::Component;
use crate::rendering::renderer::DisplayRenderer;

/// Information about the time since the last frame.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct UpdateInfo {
    /// The beginning of the last frame.
    pub last_time: Instant,
    /// The beginning of the current frame.
    pub current_time: Instant,
    /// The time in seconds that has passed since the last frame.
    pub dt: f64,
    /// The time in seconds that the last frame took without the sleeping to reach target fps.
    pub actual_dt: f64,
}

/// Actions that can be taken by components.
pub enum BreakingAction {
    /// Quit the loop.
    Quit,
}

/// Information about the screen size.
#[derive(Clone)]
pub struct DisplayInfo {
    _height: usize,
    _width: usize,
}

impl DisplayInfo {
    pub fn new(width: usize, height: usize) -> Self {
        Self {
            _width: width,
            _height: height,
        }
    }

    /// Returns the width of the screen.
    pub fn width(&self) -> usize {
        self._width
    }

    /// Returns the height of the screen.
    pub fn height(&self) -> usize {
        self._height
    }
}

// note: the anymap is a part of SharedState because plugin libraries may want to be generic over the kind
// of SharedState<S> they support, so moving `extensions` into a users custom data would not allow
// those plugins to access the anymap anymore.
/// The shared state that is passed to all components when they are executed.
///
/// # Custom state
/// **teng** supports custom state that can be shared between components.
/// This can be used to store game state, configuration, etc.
/// For often used state, `SharedState` allows embedding that state directly into the struct via the generic parameter.
/// For arbitrary data that may not be known statically, `SharedState` contains an `AnyMap` that can store arbitrary data.
///
/// See `examples/ecs/` for an example of how to use embedded custom state.
pub struct SharedState<S = ()> {
    pub mouse_info: MouseInfo,
    pub mouse_pressed: MousePressedInfo,
    pub mouse_events: MouseEvents,
    pub target_fps: Option<f64>,
    pub display_info: DisplayInfo,
    pub pressed_keys: PressedKeys,
    pub debounced_down_keys: HashSet<KeyCode>,
    pub debug_info: DebugInfo,
    pub debug_messages: SmallVec<[DebugMessage; 16]>,
    pub extensions: AnyMap,
    pub components_to_add: Vec<Box<dyn Component<S>>>,
    pub fake_events_for_next_frame: Vec<Event>,
    pub remove_components: HashSet<std::any::TypeId>,
    pub whitelisted_components: Option<HashSet<std::any::TypeId>>,
    pub custom: S,
}

impl<S: Default + 'static> SharedState<S> {
    fn new(width: usize, height: usize) -> Self {
        Self {
            mouse_info: MouseInfo::default(),
            mouse_pressed: MousePressedInfo::default(),
            mouse_events: MouseEvents::new(),
            target_fps: None,
            display_info: DisplayInfo::new(width, height),
            pressed_keys: PressedKeys::new(),
            debounced_down_keys: HashSet::new(),
            debug_info: DebugInfo::new(),
            debug_messages: SmallVec::new(),
            extensions: AnyMap::new(),
            components_to_add: Vec::new(),
            fake_events_for_next_frame: Vec::new(),
            remove_components: HashSet::new(),
            whitelisted_components: None,
            custom: S::default(),
        }
    }

    fn resize(&mut self, width: usize, height: usize) {
        self.display_info = DisplayInfo::new(width, height);
    }

    fn is_component_active(&self, component: &dyn Component<S>) -> bool {
        if let Some(whitelist) = &self.whitelisted_components {
            if !whitelist.contains(&component.type_id()) {
                return false;
            }
        }
        component.is_active(self)
    }
}

/// Information used during the setup phase of the game.
pub struct SetupInfo {
    /// The current screen size.
    pub display_info: DisplayInfo,
}

/// The driving type of a game.
///
/// A `Game` consists of a number of components that are executed in a loop and act on some shared state.
///
/// # Example
/// ```rust ,no_run
/// use teng::{install_panic_handler, terminal_cleanup, terminal_setup, Game};
///
/// terminal_setup().unwrap();
/// install_panic_handler();
///
/// let mut game: Game<_, ()> = Game::new_with_custom_buf_writer();
/// game.install_recommended_components();
/// game.run().unwrap();
///
/// terminal_cleanup().unwrap();
/// ```
pub struct Game<W: Write, S = ()> {
    display_renderer: DisplayRenderer<W>,
    components: Vec<Box<dyn Component<S>>>,
    shared_state: SharedState<S>,
    event_read_thread_handle: Option<std::thread::JoinHandle<()>>,
    event_reader: Receiver<Event>,
    event_read_stop_signal: std::sync::mpsc::Sender<()>,
}

impl<S: Default + 'static> Game<CustomBufWriter, S> {
    /// Creates a new game with a sink that only flushes once every frame.
    /// This is the recommended sink.
    pub fn new_with_custom_buf_writer() -> Self {
        let buf_writer = CustomBufWriter::new();
        Self::new(buf_writer)
    }
}

impl<S: Default + 'static> Game<Stdout, S> {
    pub fn new_with_stdout() -> Self {
        let stdout = stdout();
        Self::new(stdout)
    }
}

impl<W: Write, S: Default + 'static> Game<W, S> {
    /// Creates a new game with the given sink.
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
            shared_state: SharedState::<S>::new(width, height),
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

    /// Adds a component to the game.
    ///
    /// The component will be added to the end of the list of components, giving it a later
    /// update order, but a higher render priority.
    pub fn add_component(&mut self, component: Box<dyn Component<S>>) {
        self.components.push(component);
    }

    // TODO: remove this? or rework once we have a new() function on the Component trait
    #[doc(hidden)]
    pub fn add_component_with(
        &mut self,
        init_fn: impl FnOnce(usize, usize) -> Box<dyn Component<S>>,
    ) {
        self.components.push(init_fn(self.width(), self.height()));
    }

    /// Runs the game loop.
    ///
    /// This function will block until the game loop is finished, which happens when a component
    /// returns [`BreakingAction::Quit`].
    pub fn run(&mut self) -> io::Result<()> {
        // TODO: think about taking ownership of self and making `event_read_thread_handle` non-optional
        // Right now it feels like you can just run `run` multiple times, but this will not spawn new event reader threads.
        // Better fix perhaps: Move the event read setup into self.setup().

        // Setup phase
        self.setup()?;

        // Game loop
        let mut last_frame = Instant::now();
        let mut now = Instant::now();
        // how much longer the last sleep() slept than expected.
        let mut last_overhead = Duration::from_nanos(0);

        // how long the last frame's computations took
        let mut last_actual_dt = 1.0;

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
            if !self.shared_state.is_component_active(component.as_ref()) {
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
            if !self.shared_state.is_component_active(component.as_ref()) {
                continue;
            }
            component.on_resize(width, height, &mut self.shared_state);
        }
    }

    fn update(&mut self, update_info: UpdateInfo) {
        for component in self.components.iter_mut() {
            if !self.shared_state.is_component_active(component.as_ref()) {
                continue;
            }
            component.update(update_info, &mut self.shared_state);
        }
        self.update_game(update_info);
    }

    fn swap_component<C: Component<S>>(
        &mut self,
        new: impl FnOnce(usize, usize) -> Box<dyn Component<S>>,
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
        // TODO: Only swap component if it existed in the first place, or add some config option
        if self
            .shared_state
            .pressed_keys
            .did_press_char_ignore_case('i')
        {
            self.swap_component::<DebugInfoComponent>(|width, height| {
                Box::new(DebugInfoComponent::new())
            });
        }
        for remove_component in self.shared_state.remove_components.drain() {
            self.components.retain(|c| c.type_id() != remove_component);
        }
        for new_component in self.shared_state.components_to_add.drain(..) {
            // TODO: these components need to be setup() as well
            self.components.push(new_component);
        }
    }

    fn render(&mut self) -> io::Result<()> {
        for (idx, component) in self.components.iter().enumerate() {
            if !self.shared_state.is_component_active(component.as_ref()) {
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
            display_info: self.shared_state.display_info.clone(),
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

    pub fn install_recommended_components(&mut self) {
        self.add_component(Box::new(KeyPressRecorderComponent::new()));
        self.add_component(Box::new(FpsLockerComponent::new(144.0)));
        self.add_component(Box::new(MouseTrackerComponent::new()));
        self.add_component(Box::new(QuitterComponent));
    }
}

/// Sets up the terminal for the game.
///
/// This function should be called before any other terminal functions.
/// It sets up the terminal for raw mode, hides the cursor, tells the terminal to send mouse events,
/// and enters the alternate screen.
///
/// It is recommended to call `install_panic_handler` after this function, and `terminal_cleanup` after the game loop.
///
/// Note: If you are stuck in a bad terminal state, you can try running `reset` in the terminal.
pub fn terminal_setup() -> io::Result<()> {
    let mut stdout = stdout();

    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    enable_raw_mode()?;
    execute!(stdout, EnableMouseCapture)?;
    // don't print cursor
    execute!(stdout, cursor::Hide)?;

    Ok(())
}

/// Cleans up the terminal after the game.
///
/// This function should be called after the game loop has finished. It resets everything done
/// by `terminal_setup`.
pub fn terminal_cleanup() -> io::Result<()> {
    let mut stdout = stdout();
    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, cursor::Show)?;

    // show cursor
    execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    disable_raw_mode()?;

    execute!(stdout, crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}

/// Installs a panic handler that cleans up the terminal before panicking.
///
/// Without this, the panic message would not be displayed properly because we're in a different
/// terminal mode and in the alternate screen.
pub fn install_panic_handler() {
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |pinfo| {
        terminal_cleanup().unwrap();
        eprintln!("{}", pinfo);
        old_hook(pinfo);
    }));
}

/// Custom buffer writer that _only_ flushes explicitly
///
/// Surprisingly leads to a speedup from 2000 fps to 4800 fps on a full screen terminal
/// Update: Since diff rendering, the difference between this and Stdout directly is smaller.
pub struct CustomBufWriter {
    buf: Vec<u8>,
    stdout: Stdout,
}

impl CustomBufWriter {
    fn new() -> Self {
        Self {
            buf: vec![],
            stdout: stdout(),
        }
    }
}

impl Write for CustomBufWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut lock = self.stdout.lock();
        lock.write_all(&self.buf)?;
        lock.flush()?;
        self.buf.clear();
        Ok(())
    }
}
