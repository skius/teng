use crate::util::for_coord_in_line;
use crate::{BreakingAction, Component, SharedState, UpdateInfo};
use crossterm::event::{Event, MouseEvent, MouseEventKind};

/// Information about the current *state* of the mouse.
/// If you are interested in mouse button presses, see `MousePressedInfo`.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct MouseInfo {
    // x, y
    /// The last known position of the mouse.
    pub last_mouse_pos: (usize, usize),
    /// Is the left mouse button currently down?
    pub left_mouse_down: bool,
    /// Is the right mouse button currently down?
    pub right_mouse_down: bool,
    /// Is the middle mouse button currently down?
    pub middle_mouse_down: bool,
}

/// Information about mouse button presses since last frame.
#[derive(Default, Debug, PartialEq)]
pub struct MousePressedInfo {
    /// Has the left mouse button been pressed since the last frame?
    pub left: bool,
    /// Has the right mouse button been pressed since the last frame?
    pub right: bool,
    /// Has the middle mouse button been pressed since the last frame?
    pub middle: bool,
}

impl MousePressedInfo {
    /// Has any mouse button been pressed since the last frame?
    pub fn any(&self) -> bool {
        self.left || self.right || self.middle
    }
}

/// Information about mouse button releases since last frame.
#[derive(Default, Debug, PartialEq)]
pub struct MouseReleasedInfo {
    /// Has the left mouse button been released since the last frame?
    pub left: bool,
    /// Has the right mouse button been released since the last frame?
    pub right: bool,
    /// Has the middle mouse button been released since the last frame?
    pub middle: bool,
}

/// Aggregates mouse events since last frame.
///
/// Use this to get various interpolation mechanics for mouse events, for example, a component
/// that draws a line following the mouse cursor can use this struct to interpolate a continuous
/// line between individual mouse events.
/// This is necessary because the terminal may skip sending mouse events for some screen coordinates
/// if the mouse moves too quickly, resulting in discontinuous recorded mouse positions.
pub struct MouseEvents {
    events: Vec<MouseInfo>,
    has_new_this_frame: bool,
}

impl MouseEvents {
    /// Creates an empty `MouseEvents` struct.
    pub fn new() -> Self {
        Self {
            events: vec![],
            has_new_this_frame: false,
        }
    }

    /// Records a new mouse event during this frame.
    pub fn push(&mut self, event: MouseInfo) {
        self.events.push(event);
    }

    /// Returns true if there has been a new mouse event this frame.
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

/// Component that tracks mouse state and events.
///
/// Should be early in the update order to ensure that other components can use the mouse state.
pub struct MouseTrackerComponent {
    last_mouse_info: MouseInfo,
    did_press_left: bool,
    did_press_right: bool,
    did_press_middle: bool,
    did_release_left: bool,
    did_release_right: bool,
    did_release_middle: bool,
    mouse_events: MouseEvents,
}

impl MouseTrackerComponent {
    /// Creates a new `MouseTrackerComponent`.
    pub fn new() -> Self {
        Self {
            last_mouse_info: MouseInfo::default(),
            did_press_left: false,
            did_press_right: false,
            did_press_middle: false,
            did_release_left: false,
            did_release_right: false,
            did_release_middle: false,
            mouse_events: MouseEvents::new(),
        }
    }

    /// Updates a [`MouseInfo`] struct with the information from a `MouseEvent`.
    pub fn update_mouse_info(event: MouseEvent, mouse_info: &mut MouseInfo) {
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
    ///
    /// If the two `MouseInfo` structs differ in button state, the first one is passed to every
    /// closure call except the last one, where the second one is passed.
    /// For example, if we're interpolating from "mouse at `(0,0)`, left mouse up" to "mouse at `(0,10)`, left mouse down",
    /// then `f` will be called with "mouse at `(0,i)`, left mouse up" for `i` in `0..=9`,
    /// and finally with "mouse at `(0,10)`, left mouse down".
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

impl<S> Component<S> for MouseTrackerComponent {
    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        if let Event::Mouse(event) = event {
            Self::update_mouse_info(event, &mut self.last_mouse_info);
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
                MouseEvent {
                    kind: MouseEventKind::Up(button),
                    ..
                } => match button {
                    // Note: we are sticky-setting did_release_*, same as bove
                    crossterm::event::MouseButton::Left => {
                        self.did_release_left = true;
                    }
                    crossterm::event::MouseButton::Right => {
                        self.did_release_right = true;
                    }
                    crossterm::event::MouseButton::Middle => {
                        self.did_release_middle = true;
                    }
                },
                _ => {}
            }
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        shared_state.mouse_info = self.last_mouse_info;
        shared_state.mouse_pressed.right = self.did_press_right;
        shared_state.mouse_pressed.left = self.did_press_left;
        shared_state.mouse_pressed.middle = self.did_press_middle;
        shared_state.mouse_released.right = self.did_release_right;
        shared_state.mouse_released.left = self.did_release_left;
        shared_state.mouse_released.middle = self.did_release_middle;
        std::mem::swap(&mut self.mouse_events, &mut shared_state.mouse_events);
        self.mouse_events.events.clear();
        // always have the last mouse info in the queue
        self.mouse_events.push(self.last_mouse_info);
        self.mouse_events.has_new_this_frame = false;

        self.did_press_left = false;
        self.did_press_right = false;
        self.did_press_middle = false;
        self.did_release_left = false;
        self.did_release_right = false;
        self.did_release_middle = false;
    }
}
