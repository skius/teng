use crate::game::components::incremental::planarvec::{Bounds, PlanarVec};
use crate::game::components::MouseTrackerComponent;
use crate::game::{
    BreakingAction, Color, Component, DisplayInfo, HalfBlockDisplayRender, MouseInfo, Render,
    Renderer, SetupInfo, SharedState, UpdateInfo,
};
use crossterm::event::Event;
use std::time::{Duration, Instant};

#[derive(Debug, Clone, Copy, PartialEq)]
enum PieceKind {
    Air,
    Sand,
    Water,
}

impl PieceKind {
    fn density(&self) -> f64 {
        match self {
            PieceKind::Air => 0.0,
            PieceKind::Sand => 2.0,
            PieceKind::Water => 1.0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct Piece {
    kind: PieceKind,
}

struct FallingSimulationData {
    secs_passed: f64,
    total_pieces: usize,
    world: PlanarVec<Piece>,
    has_moved: PlanarVec<bool>,
}

impl FallingSimulationData {
    fn new() -> Self {
        let bounds = Bounds {
            min_x: -100,
            max_x: 100,
            min_y: -100,
            max_y: 100,
        };

        Self {
            secs_passed: 0.0,
            total_pieces: 0,
            world: PlanarVec::new(
                bounds,
                Piece {
                    kind: PieceKind::Air,
                },
            ),
            has_moved: PlanarVec::new(bounds, false),
        }
    }

    fn swap(&mut self, (x1, y1): (i64, i64), (x2, y2): (i64, i64)) {
        let temp = self.world[(x1, y1)];
        self.world[(x1, y1)] = self.world[(x2, y2)];
        self.world[(x2, y2)] = temp;
    }

    fn sim_sand(&mut self, (x, y): (i64, i64)) {
        let piece = self.world[(x, y)];

        // check below
        if let Some(&below) = self.world.get(x, y - 1) {
            if below.kind.density() < piece.kind.density() {
                self.swap((x, y), (x, y - 1));
                self.has_moved[(x, y)] = true;
                self.has_moved[(x, y - 1)] = true;
                // moved, no more sim
                return;
            }
        }
        // check below and right
        if let Some(&below_right) = self.world.get(x + 1, y - 1) {
            if below_right.kind.density() < piece.kind.density() {
                self.swap((x, y), (x + 1, y - 1));
                self.has_moved[(x, y)] = true;
                self.has_moved[(x + 1, y - 1)] = true;
                // moved, no more sim
                return;
            }
        }
        // check below and left
        if let Some(&below_left) = self.world.get(x - 1, y - 1) {
            if below_left.kind.density() < piece.kind.density() {
                self.swap((x, y), (x - 1, y - 1));
                self.has_moved[(x, y)] = true;
                self.has_moved[(x - 1, y - 1)] = true;
                // moved, no more sim
                return;
            }
        }
    }

    fn sim_water(&mut self, (x, y): (i64, i64)) {
        let piece = self.world[(x, y)];

        // check below
        if let Some(&below) = self.world.get(x, y - 1) {
            if below.kind.density() < piece.kind.density() {
                self.swap((x, y), (x, y - 1));
                self.has_moved[(x, y)] = true;
                self.has_moved[(x, y - 1)] = true;
                // moved, no more sim
                return;
            }
        }
        // check below and right
        if let Some(&below_right) = self.world.get(x + 1, y - 1) {
            if below_right.kind.density() < piece.kind.density() {
                self.swap((x, y), (x + 1, y - 1));
                self.has_moved[(x, y)] = true;
                self.has_moved[(x + 1, y - 1)] = true;
                // moved, no more sim
                return;
            }
        }
        // check below and left
        if let Some(&below_left) = self.world.get(x - 1, y - 1) {
            if below_left.kind.density() < piece.kind.density() {
                self.swap((x, y), (x - 1, y - 1));
                self.has_moved[(x, y)] = true;
                self.has_moved[(x - 1, y - 1)] = true;
                // moved, no more sim
                return;
            }
        }
        // check right
        if let Some(&right) = self.world.get(x + 1, y) {
            // note: we are not checking densities anymore, since this is on the horizontal axis.
            if right.kind == PieceKind::Air {
                self.swap((x, y), (x + 1, y));
                self.has_moved[(x, y)] = true;
                self.has_moved[(x + 1, y)] = true;
                // moved, no more sim
                return;
            }
        }
        // check left
        if let Some(&left) = self.world.get(x - 1, y) {
            // note: we are not checking densities anymore, since this is on the horizontal axis.
            if left.kind == PieceKind::Air {
                self.swap((x, y), (x - 1, y));
                self.has_moved[(x, y)] = true;
                self.has_moved[(x - 1, y)] = true;
                // moved, no more sim
                return;
            }
        }
    }
}

pub struct FallingSimulationComponent {
    dt_budget: f64,
    last_mouse_info: MouseInfo,
    hb_display: HalfBlockDisplayRender,
}

impl FallingSimulationComponent {
    const UPDATES_PER_SECOND: f64 = 100.0;
    const UPDATE_INTERVAL: f64 = 1.0 / Self::UPDATES_PER_SECOND;

    pub fn new() -> Self {
        Self {
            dt_budget: 0.0,
            last_mouse_info: MouseInfo::default(),
            hb_display: HalfBlockDisplayRender::new(10, 10),
        }
    }

    fn update_render(&mut self, data: &FallingSimulationData, display_info: &DisplayInfo) {
        // TODO: add display here

        for x in data.world.x_range() {
            for y in data.world.y_range() {
                let piece = data.world[(x, y)];
                let color = match piece.kind {
                    PieceKind::Air => Color::Transparent,
                    PieceKind::Sand => Color::Rgb([255, 255, 0]),
                    PieceKind::Water => Color::Rgb([0, 0, 255]),
                };
                let d_x = x + 100;
                let d_y = y + 100;
                let d_y = 2 * display_info.height() as i64 - d_y;
                let d_y = d_y - 1;
                self.hb_display.set_color(d_x as usize, d_y as usize, color);
            }
        }
    }

    fn update_simulation(&mut self, shared_state: &mut SharedState) {
        let data = shared_state
            .extensions
            .get_mut::<FallingSimulationData>()
            .unwrap();
        data.secs_passed += Self::UPDATE_INTERVAL;

        // std::mem::swap(&mut data.world, &mut data.old_world);
        // data.world.clear(Piece { kind: PieceKind::Air });

        data.has_moved.clear(false);

        // TODO: why is total_pieces getting smaller??
        // ah. because we're overwriting data.world when moving, even if something else already moved
        // there. fix: read from new_world maybe?
        // or for performance reasons just keep track of each piece and then move those?
        data.total_pieces = 0;

        // go over every piece (that is not air) and update it
        for x in data.world.x_range() {
            for y in data.world.y_range().rev() {
                if data.has_moved[(x, y)] {
                    continue;
                }
                let piece = data.world[(x, y)];
                if piece.kind == PieceKind::Air {
                    continue;
                }

                match piece.kind {
                    PieceKind::Air => {
                        // do nothing
                    }
                    PieceKind::Sand => {
                        data.sim_sand((x, y));
                    }
                    PieceKind::Water => {
                        data.sim_water((x, y));
                    }
                }
                data.has_moved[(x, y)] = true;
            }
        }

        for x in data.world.x_range() {
            for y in data.world.y_range() {
                let piece = data.world[(x, y)];
                if piece.kind != PieceKind::Air {
                    data.total_pieces += 1;
                }
            }
        }

        self.update_render(data, &shared_state.display_info);
    }
}

impl Component for FallingSimulationComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        self.hb_display
            .resize_discard(setup_info.width, setup_info.height * 2);
        shared_state.extensions.insert(FallingSimulationData::new());
    }

    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        // // get mouse and set everything to 'sand' on LMB

        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let dt = update_info.dt;
        self.dt_budget += dt;

        // add sand from mouse events
        let data = shared_state
            .extensions
            .get_mut::<FallingSimulationData>()
            .unwrap();

        if shared_state.mouse_info.left_mouse_down
            || shared_state.mouse_info.right_mouse_down
            || shared_state.mouse_info.middle_mouse_down
        {
            let (s_x, s_y) = shared_state.mouse_info.last_mouse_pos;

            let x = s_x as i64 - 100;
            // turn y around
            let y = shared_state.display_info.height() as i64 - s_y as i64;
            // scale to two halfblocks per pixel and recenter to 0,0
            let y = (y * 2) - 100;

            if let Some(piece) = data.world.get_mut(x, y) {
                let kind = if shared_state.mouse_info.left_mouse_down {
                    PieceKind::Sand
                } else if shared_state.mouse_info.right_mouse_down {
                    PieceKind::Water
                } else {
                    PieceKind::Air
                };
                piece.kind = kind;
            }
        }

        while self.dt_budget >= Self::UPDATE_INTERVAL {
            self.update_simulation(shared_state);
            self.dt_budget -= Self::UPDATE_INTERVAL;
        }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX - 99;
        let data = shared_state
            .extensions
            .get::<FallingSimulationData>()
            .unwrap();
        format!("FallingSimulationComponent: {}", data.secs_passed).render(
            &mut renderer,
            0,
            0,
            depth_base,
        );
        format!("sands: [{}]", data.total_pieces).render(&mut renderer, 0, 1, depth_base);

        self.hb_display.render(&mut renderer, 0, 0, depth_base);
    }
}
