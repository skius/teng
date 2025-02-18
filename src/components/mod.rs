use crate::rendering::renderer::Renderer;
use crate::{BreakingAction, SetupInfo, SharedState, UpdateInfo};
use crossterm::event::Event;
use std::any::Any;

pub mod debuginfo;
pub mod eventrecorder;
pub mod fpslocker;
pub mod keyboard;
pub mod mouse;
pub mod quitter;

/// A game component that can listen to events, perform logic, and render itself.
/// Components are the main way to extend the game's functionality.
pub trait Component<S = ()>: Any {
    /// Called in the very beginning. Useful to initialize more components or extension states.
    /// Called also if `is_active` returns false.
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<S>) {}
    /// Called to determine if this component is active. If not, none of the other methods will be invoked.
    fn is_active(&self, shared_state: &SharedState<S>) -> bool {
        true
    }
    /// Called when the terminal is resized.
    /// Note that Resize events are also passed to on_event, so this is not strictly necessary.
    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<S>) {}
    /// Called when the game exits. Useful for cleanup.
    fn on_quit(&mut self, shared_state: &mut SharedState<S>) {}
    /// Called when an event is received. This could happen multiple times per frame. Runs before update.
    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        None
    }
    /// Called once per frame to update the component's state. Runs after the frame's events have been processed.
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<S>) {}
    /// Called once per frame to render the component. Each component has 100 depth available
    /// starting from the base.
    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<S>, depth_base: i32) {}
}
