use crate::{BreakingAction, Component, DebugMessage, SetupInfo, SharedState, UpdateInfo};
use crossterm::event::Event;
use serde::{Deserialize, Serialize};
use std::path::Path;
use std::time::SystemTime;

#[derive(Serialize, Deserialize, Debug)]
pub struct RecordedEvent {
    pub event: Event,
    /// The offset in ns from the start of the recording
    pub ns_offset: u128,
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Recording {
    pub events: Vec<RecordedEvent>,
    pub initial_display_size: (usize, usize),
    /// The time at which the recording stopped relative to its start.
    /// This is the same as the duration of the recording.
    /// Set when the recording is finished.
    pub duration_ns_offset: u128,
}

impl Recording {
    // TODO: handle errors
    pub fn read_from_file(path: impl AsRef<Path>) -> Self {
        let file = std::fs::File::open(path).unwrap();
        bincode::deserialize_from(file).unwrap()
    }
}

pub struct EventRecorderComponent {
    active_recording: Recording,
    recording: bool,
    current_start_time: std::time::Instant,
    current_display_size: (usize, usize),
}

impl EventRecorderComponent {
    pub fn new() -> Self {
        Self {
            recording: false,
            active_recording: Recording::default(),
            current_start_time: std::time::Instant::now(),
            current_display_size: (0, 0),
        }
    }

    fn get_new_file_path(&self) -> String {
        let timestamp = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs();
        format!("recordings/recording-{}.bin", timestamp)
    }

    pub fn start_recording(&mut self) {
        self.recording = true;
        self.active_recording = Recording {
            events: vec![],
            initial_display_size: self.current_display_size,
            duration_ns_offset: 0,
        };
        self.current_start_time = std::time::Instant::now();
    }

    pub fn stop_recording(&mut self) {
        self.recording = false;
    }

    pub fn save_recording(&mut self, path: impl AsRef<Path>) {
        assert!(!self.is_recording());
        if let Some(parent) = path.as_ref().parent() {
            std::fs::create_dir_all(parent).unwrap();
        }
        let file = std::fs::File::create(path).unwrap();
        bincode::serialize_into(file, &self.active_recording).unwrap();
    }

    pub fn is_recording(&self) -> bool {
        self.recording
    }

    pub fn record_event(&mut self, event: Event) {
        if self.is_recording() {
            let ns_offset = self.current_start_time.elapsed().as_nanos();
            self.active_recording
                .events
                .push(RecordedEvent { event, ns_offset });
        }
    }

    pub fn stop_and_save_recording(&mut self) {
        if self.is_recording() {
            self.stop_recording();
            self.save_recording(self.get_new_file_path());
        }
    }
}

impl<S> Component<S> for EventRecorderComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<S>) {
        self.current_display_size = (setup_info.width, setup_info.height);
    }

    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        self.record_event(event);
        None
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<S>) {
        self.current_display_size = (width, height);
    }

    fn on_quit(&mut self, shared_state: &mut SharedState<S>) {
        // Add 'q' key
        self.record_event(Event::Key(crossterm::event::KeyEvent::new(
            crossterm::event::KeyCode::Char('q'),
            crossterm::event::KeyModifiers::empty(),
        )));
        self.stop_and_save_recording();
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        // Check whether we start/stop a recording here, so we're starting/stopping event recording
        // at a frame boundary.
        if shared_state.pressed_keys.did_press_char_ignore_case('r') {
            if self.is_recording() {
                self.stop_and_save_recording();
                shared_state
                    .debug_messages
                    .push(DebugMessage::new_3s("Recording stopped and saved"));
            } else {
                self.start_recording();
                shared_state
                    .debug_messages
                    .push(DebugMessage::new_3s("Recording started"));
            }
        }
    }
}

// TODO: Does not currently handle resize events properly.
// Unsure how to even handle them, do we just cap the mouse events at the current display size?
pub struct EventReplayerComponent {
    recording: Recording,
    replaying: bool,
    replay_start_time: std::time::Instant,
    /// The amount of events in `recording` that have been replayed and can be skipped.
    finished_events: usize,
}

impl EventReplayerComponent {
    pub fn new(immediately_start_playing: bool, recording: Recording) -> Self {
        Self {
            recording,
            replaying: immediately_start_playing,
            replay_start_time: std::time::Instant::now(),
            finished_events: 0,
        }
    }

    fn play_events_until<S>(
        &mut self,
        current_time: std::time::Instant,
        shared_state: &mut SharedState<S>,
    ) {
        if !self.replaying {
            return;
        }
        let duration = current_time.duration_since(self.replay_start_time);
        let ns_offset = duration.as_nanos();
        let mut events_played = 0;
        for event in &self.recording.events[self.finished_events..] {
            if event.ns_offset <= ns_offset {
                shared_state
                    .fake_events_for_next_frame
                    .push(event.event.clone());
                events_played += 1;
            } else {
                break;
            }
        }
        self.finished_events += events_played;
        if self.finished_events == self.recording.events.len() {
            self.replaying = false;
            self.finished_events = 0;
            shared_state
                .debug_messages
                .push(DebugMessage::new_3s("Replay finished"));
        }
    }
}

impl<S> Component<S> for EventReplayerComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<S>) {
        assert_eq!(
            setup_info.width, self.recording.initial_display_size.0,
            "Width mismatch for replay"
        );
        assert_eq!(
            setup_info.height, self.recording.initial_display_size.1,
            "Height mismatch for replay"
        );
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        let current_time = update_info.current_time;
        self.play_events_until(current_time, shared_state);
    }
}

pub struct BenchFrameCounter {
    frame_count: usize,
    report_fn: Box<dyn Fn(usize)>,
}

impl BenchFrameCounter {
    pub fn new(report_fn: impl Fn(usize) + 'static) -> Self {
        Self {
            frame_count: 0,
            report_fn: Box::new(report_fn),
        }
    }
}

impl<S> Component<S> for BenchFrameCounter {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {
        self.frame_count += 1;
    }

    fn on_quit(&mut self, shared_state: &mut SharedState<S>) {
        // report the count
        (self.report_fn)(self.frame_count);
    }
}
