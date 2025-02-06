use std::time::{Duration, Instant};
use crossterm::event::{Event, MouseButton, MouseEvent, MouseEventKind};
use crate::game::{BreakingAction, Color, Component, DebugMessage, HalfBlockDisplayRender, Pixel, Render, Renderer, SetupInfo, SharedState, UpdateInfo};
use crate::game::components::incremental::{GamePhase, GameState};
use crate::game::components::incremental::ui::UiBarComponent;
use crate::game::components::MouseTrackerComponent;
use crate::game::util::for_coord_in_line;

pub struct SlingshotComponent {
    // 'Some' with screen coords of the first mouse down event during this slingshot
    first_down: Option<(usize, usize)>,
    // 'Some' with screen coords of the last mouse up event
    last_release: Option<(usize, usize)>,
    // relative (x, y) of the slingshot from the player
    slingshot: Option<(i64, i64)>,
    half_block_display_render: HalfBlockDisplayRender,
}

impl SlingshotComponent {
    pub fn new() -> Self {
        Self {
            first_down: None,
            last_release: None,
            slingshot: None,
            half_block_display_render: HalfBlockDisplayRender::new(0, 0),
        }
    }
}

impl Component for SlingshotComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        self.half_block_display_render = HalfBlockDisplayRender::new(setup_info.width, 2 * setup_info.height);
    }

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
                    MouseEventKind::Down(MouseButton::Left) => {
                        self.first_down.get_or_insert((x, y));
                        // we have no release for _this down_ yet
                        self.last_release = None;
                    }
                    _ => {}
                }
            }
            Event::Resize(width, height) => {
                self.half_block_display_render.resize_discard(width as usize, 2 * (height as usize));
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let mut game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
        // don't do anything except reset if we're dead
        if game_state.new_player_state.dead_time.is_some() {
            self.first_down = None;
            self.last_release = None;
            self.slingshot = None;
            self.half_block_display_render.clear();
            return;
        }

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

        // if we have an in-bounds first_down and a last_release (anywhere), set a slingshot
        if let Some((initial_x, initial_y)) = self.first_down {
            if let Some((last_x, last_y)) = self.last_release {
                let dx = last_x as i64 - initial_x as i64;
                let dy = last_y as i64 - initial_y as i64;
                // screen coords are flipped in y
                let dy = -dy;

                // invert because we want to apply 'slingshot' force
                slingshot = Some((-dx, -dy));
            } else {
                // we must still be pressing
                game_state.new_player_state.paused = true;
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
            // add velocity
            // game_state.new_player_state.entity.velocity.0 += s_x as f64 * 1.0;
            // game_state.new_player_state.entity.velocity.1 += s_y as f64 * 2.0;
            // override velocity
            game_state.new_player_state.entity.velocity.0 = s_x as f64 * 4.0;
            game_state.new_player_state.entity.velocity.1 = s_y as f64 * 4.0;

            self.first_down = None;
            self.last_release = None;
            game_state.new_player_state.paused = false;
        }


        // prepare render:
        // render a line in screenspace
        self.half_block_display_render.clear();
        if let Some((initial_x, initial_y)) = self.first_down {
            if shared_state.mouse_info.left_mouse_down {
                let (last_x, last_y) = shared_state.mouse_info.last_mouse_pos;

                let start = (initial_x as i64, initial_y as i64 * 2);
                let end = (last_x as i64, last_y as i64 * 2);

                // draw a lind from initial to last. use the mouse interpolator
                for_coord_in_line(start, end, |x, y| {
                    let x = x as usize;
                    let y = y as usize;
                    // don't render over UI at all. even if depth was appropriate, because we mess with background etc this could be ugly.
                    // so just hardcode that we don't render there
                    if y / 2 >= shared_state.display_info.height() - UiBarComponent::HEIGHT {
                        return;
                    }
                    self.half_block_display_render.set_color(x, y, Color::Rgb([255; 3]));
                });
            }
        }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        self.half_block_display_render.render(&mut renderer, 0, 0, depth_base);
    }
}