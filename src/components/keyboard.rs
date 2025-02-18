use std::collections::HashMap;
use std::time::Instant;
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use crate::{BreakingAction, Component, SharedState, UpdateInfo};

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
