use crate::{BreakingAction, Component, SharedState};
use crossterm::event::{Event, KeyCode, KeyEvent};

/// A component that quits the game when the user presses 'q'.
pub struct QuitterComponent;

impl<S> Component<S> for QuitterComponent {
    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<S>,
    ) -> Option<BreakingAction> {
        if matches!(
            event,
            Event::Key(KeyEvent {
                // TODO: Add breakingaction to update() and move this there and used shared_state?
                code: KeyCode::Char('q' | 'Q'),
                ..
            })
        ) {
            Some(BreakingAction::Quit)
        } else {
            None
        }
    }
}
