use crate::{BreakingAction, Component, SharedState, UpdateInfo};
use crossterm::event::{Event, KeyCode, KeyEvent, KeyEventKind};
use std::collections::HashMap;
use std::time::Instant;

// TODO: swap to `crokey` crate architecture?
// Needing to check for "M" when we actually mean "shift-m" is a bit confusing.

/// Contains the keys that have been pressed since the last update.
pub struct PressedKeys {
    inner: micromap::Map<KeyCode, u8, 16>,
}

impl PressedKeys {
    /// Creates a new `PressedKeys` instance.
    pub fn new() -> Self {
        Self {
            inner: micromap::Map::new(),
        }
    }

    /// Returns the raw map of pressed keys.
    pub fn inner(&self) -> &micromap::Map<KeyCode, u8, 16> {
        &self.inner
    }

    /// Not recommended to use. However, it is useful to hack key actions in other components
    /// if the update order is known.
    pub fn insert(&mut self, key: KeyCode) {
        self.inner.insert(key, 1);
    }

    /// Returns true if the given key was pressed since the last update.
    pub fn did_press_char(&self, c: char) -> bool {
        self.inner.contains_key(&KeyCode::Char(c))
    }

    /// Returns true if the given key was pressed since the last update, ignoring case.
    pub fn did_press_char_ignore_case(&self, c: char) -> bool {
        self.did_press_char(c) || self.did_press_char(c.to_ascii_uppercase())
    }

    /// Returns true if the given key was pressed since the last update.
    pub fn did_press(&self, key: KeyCode) -> bool {
        self.inner.contains_key(&key)
    }
}

/// A component that records key presses.
///
/// Manages the `SharedState::pressed_keys` field, and must be in the update order before any component
/// that uses `SharedState::pressed_keys`.
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

impl<S> Component<S> for KeyPressRecorderComponent {
    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
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

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        std::mem::swap(&mut shared_state.pressed_keys.inner, &mut self.pressed_keys);
        self.pressed_keys.clear();
    }
}

/// A component that tries to figure out the current "up"/"down" state of a key only from
/// `KeyEventKind::Press` events.
///
/// Some terminals do not send `KeyEventKind::Release` events, only `KeyEventKind::Press` events,
/// which makes it hard to figure out the current state of a key.
/// Because typically those terminals will repeat a `KeyEventKind::Press` event after some delay if the key is held
/// down, we can assume that a key is "down" if we have seen a `KeyEventKind::Press` event for it within that delay,
/// and "up" if it's been longer than that delay since the last `KeyEventKind::Press` event.
///
/// It manages the `SharedState::debounced_down_keys` field, and must appear in the update order
/// before any component that uses `SharedState::debounced_down_keys`.
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

impl<S> Component<S> for KeypressDebouncerComponent {
    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
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

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
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
