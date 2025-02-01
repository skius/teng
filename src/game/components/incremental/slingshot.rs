use std::time::{Duration, Instant};
use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use crate::game::{BreakingAction, Component, DebugMessage, Pixel, Renderer, SharedState, UpdateInfo};
use crate::game::components::incremental::{GamePhase, GameState};
use crate::game::components::MouseTrackerComponent;

pub struct SlingshotComponent {
    // 'Some' with screen coords of the first mouse down event during this slingshot
    first_down: Option<(usize, usize)>,
    // 'Some' with screen coords of the last mouse up event
    last_release: Option<(usize, usize)>,
    // relative (x, y) of the slingshot from the player
    slingshot: Option<(i64, i64)>,
}

impl SlingshotComponent {
    pub fn new() -> Self {
        Self {
            first_down: None,
            last_release: None,
            slingshot: None,
        }
    }
}

impl Component for SlingshotComponent {
    fn is_active(&self, shared_state: &SharedState) -> bool {
        let game_state = shared_state.extensions.get::<GameState>().unwrap();
        game_state.phase == GamePhase::Moving && game_state.new_player_state.dead_time.is_none()
    }

    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        match event {
            Event::Mouse(event) => {
                let (x, y) = (event.column as usize, event.row as usize);
                match event.kind {
                    MouseEventKind::Up(MouseButton::Left) => {
                        self.last_release = Some((x, y));
                    }
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.first_down.get_or_insert((x, y));
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

        // remove first_down if oob
        self.first_down.take_if(|(x, y)| {
            game_state.world.to_world_pos(*x, *y).is_none()
        });

        // hack for when the user exits the window while dragging
        // TODO: this doesn't work because the mouse tracker component also doesn't see mouseup events when the
        // window is unfocused. Not sure how to fix/if it can be fixed.
        // if self.first_down.is_some() && !shared_state.mouse_info.left_mouse_down && self.last_release.is_none() {
        //     self.last_release = Some(shared_state.mouse_info.last_mouse_pos);
        // }

        // if we have an in-bounds first_down and a last_release, set a slingshot
        if let Some((initial_x, initial_y)) = self.first_down {
            if let Some((last_x, last_y)) = self.last_release {
                let dx = last_x as i64 - initial_x as i64;
                let dy = last_y as i64 - initial_y as i64;
                // screen coords are flipped in y
                let dy = -dy;

                // invert because we want to apply 'slingshot' force
                slingshot = Some((-dx, -dy));
            }
        }


        // if shared_state.mouse_info.left_mouse_down {
        //     let (s_x, s_y) = shared_state.mouse_info.last_mouse_pos;
        // if let Some((s_x, s_y)) = self.last_release.take() {
        //     // update slingshot position
        //     if let Some((world_x, world_y)) = game_state.world.to_world_pos(s_x, s_y) {
        //         // compute delta to player x and player y
        //         let player = &game_state.new_player_state.entity;
        //         let (x, y) = player.position;
        //         let dx = world_x - x.floor() as i64;
        //         let dy = world_y - y.floor() as i64;
        //
        //         // invert relative position to act as force
        //         slingshot = Some((-dx, -dy));
        //
        //     }
        //
        // }

        self.slingshot = slingshot;

        if let Some((s_x, s_y)) = self.slingshot {
            // debug
            shared_state.debug_messages.push(DebugMessage::new(format!("Slingshot: ({}, {})", s_x, s_y), Instant::now() + Duration::from_secs_f64(3.0)));
            game_state.new_player_state.entity.x_drag = 0.6;
            game_state.new_player_state.entity.velocity.0 += s_x as f64 * 1.0;
            game_state.new_player_state.entity.velocity.1 += s_y as f64 * 2.0;

            self.first_down = None;
            self.last_release = None;
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        // render a line in screenspace
        if let Some((initial_x, initial_y)) = self.first_down {
            if shared_state.mouse_info.left_mouse_down {
                let (last_x, last_y) = shared_state.mouse_info.last_mouse_pos;

                let mut first_update = shared_state.mouse_info;
                let mut last_update = shared_state.mouse_info;

                // we only care about the position smoothing
                first_update.last_mouse_pos = (initial_x, initial_y);
                last_update.last_mouse_pos = (last_x, last_y);


                // draw a lind from initial to last. use the mouse interpolator
                MouseTrackerComponent::smooth_two_updates(first_update, last_update, |mi| {
                    let pixel = Pixel::new('█');
                    renderer.render_pixel(mi.last_mouse_pos.0, mi.last_mouse_pos.1, pixel, depth_base);
                });
            }
        }
    }
}