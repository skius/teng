use std::time::{Duration, Instant};
use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use crate::game::{BreakingAction, Component, DebugMessage, SharedState, UpdateInfo};
use crate::game::components::incremental::{GamePhase, GameState};

pub struct SlingshotComponent {
    // 'Some' with screen coords of the last mouse up event (in game world)
    last_release: Option<(usize, usize)>,
    // relative (x, y) of the slingshot
    slingshot: Option<(f64, f64)>,
}

impl SlingshotComponent {
    pub fn new() -> Self {
        Self {
            last_release: None,
            slingshot: None,
        }
    }
}

impl Component for SlingshotComponent {
    fn is_active(&self, shared_state: &SharedState) -> bool {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        game_state.phase == GamePhase::Moving
    }
    
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        match event {
            Event::Mouse(event) => {
                let (x, y) = (event.column as usize, event.row as usize);
                match event.kind {
                    MouseEventKind::Up(MouseButton::Left) => {
                        self.last_release = Some((x, y));
                    }
                    _ => {}
                }
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let mut game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
        let mut slingshot = None;
        if let Some((s_x, s_y)) = self.last_release.take() {
            // update slingshot position
            if let Some((world_x, world_y)) = game_state.world.to_world_pos(s_x, s_y) {
                // compute delta to player x and player y
                slingshot = Some((world_x as f64, world_y as f64));
            }

        }

        self.slingshot = slingshot;
        
        if let Some((s_x, s_y)) = self.slingshot {
            // debug 
            shared_state.debug_messages.push(DebugMessage::new(format!("Slingshot: ({}, {})", s_x, s_y), Instant::now() + Duration::from_secs_f64(3.0)));
        }
    }
}