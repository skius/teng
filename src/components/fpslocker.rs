use crossterm::event::{Event, MouseEventKind};
use crate::{BreakingAction, Component, SetupInfo, SharedState, UpdateInfo};

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

impl<S> Component<S> for FpsLockerComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<S>) {
        shared_state.target_fps = Some(self.default_fps);
    }

    fn on_event(&mut self, event: Event, shared_state: &mut SharedState<S>) -> Option<BreakingAction> {
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

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        if shared_state.pressed_keys.did_press_char_ignore_case('l') {
            self.locked = !self.locked;
        }
        shared_state.target_fps = self.locked.then_some(self.default_fps);
    }
}