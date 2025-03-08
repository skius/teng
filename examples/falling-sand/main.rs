use std::io::stdout;
use teng::components::Component;
use teng::rendering::color::Color;
use teng::rendering::render::{HalfBlockDisplayRender, Render};
use teng::rendering::renderer::Renderer;
use teng::util::fixedupdate::FixedUpdateRunner;
use teng::util::planarvec::{Bounds, PlanarVec};
use teng::{
    DisplayInfo, Game, SetupInfo, SharedState, UpdateInfo, install_panic_handler, terminal_cleanup,
    terminal_setup,
};

fn main() -> std::io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new(stdout());
    game.install_recommended_components();
    game.add_component(Box::new(FallingSimulationComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}

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

#[derive(Default)]
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

    fn resize_discard(&mut self, width: usize, height: usize) {
        let bounds = Bounds {
            min_x: 0,
            max_x: width as i64 - 1,
            min_y: 0,
            max_y: height as i64 - 1,
        };

        self.world = PlanarVec::new(
            bounds,
            Piece {
                kind: PieceKind::Air,
            },
        );
        self.has_moved = PlanarVec::new(bounds, false);
    }
}

pub struct FallingSimulationComponent {
    hb_display: HalfBlockDisplayRender,
    fixed_update_runner: FixedUpdateRunner,
}

impl FallingSimulationComponent {
    const UPDATES_PER_SECOND: f64 = 100.0;
    const UPDATE_INTERVAL: f64 = 1.0 / Self::UPDATES_PER_SECOND;

    pub fn new() -> Self {
        Self {
            hb_display: HalfBlockDisplayRender::new(10, 10),
            fixed_update_runner: FixedUpdateRunner::new_from_rate_per_second(
                Self::UPDATES_PER_SECOND,
            ),
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
                let d_x = x;
                let d_y = y;
                let d_y = 2 * display_info.height() as i64 - d_y;
                let d_y = d_y - 1;
                self.hb_display.set_color(d_x as usize, d_y as usize, color);
            }
        }
    }

    fn update_simulation(&mut self, shared_state: &mut SharedState<FallingSimulationData>) {
        let data = &mut shared_state.custom;
        data.secs_passed += Self::UPDATE_INTERVAL;

        // std::mem::swap(&mut data.world, &mut data.old_world);
        // data.world.clear(Piece { kind: PieceKind::Air });

        data.has_moved.clear(false);

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

impl Component<FallingSimulationData> for FallingSimulationComponent {
    fn setup(
        &mut self,
        setup_info: &SetupInfo,
        shared_state: &mut SharedState<FallingSimulationData>,
    ) {
        self.on_resize(
            setup_info.display_info.width(),
            setup_info.display_info.height(),
            shared_state,
        );
    }

    fn on_resize(
        &mut self,
        width: usize,
        height: usize,
        shared_state: &mut SharedState<FallingSimulationData>,
    ) {
        self.hb_display.resize_discard(width, height * 2);
        let data = &mut shared_state.custom;
        data.resize_discard(width, height * 2);
    }

    fn update(
        &mut self,
        update_info: UpdateInfo,
        shared_state: &mut SharedState<FallingSimulationData>,
    ) {
        self.fixed_update_runner.fuel(update_info.dt);

        // add sand from mouse events
        let data = &mut shared_state.custom;

        if shared_state.mouse_info.left_mouse_down
            || shared_state.mouse_info.right_mouse_down
            || shared_state.mouse_info.middle_mouse_down
        {
            let (s_x, s_y) = shared_state.mouse_info.last_mouse_pos;

            let x = s_x as i64;
            // scale to two halfblocks per pixel and recenter to 0,0
            let y = shared_state.display_info.height() as i64 - s_y as i64;
            let y = 2 * y;
            let y = y - 1;

            if let Some(piece) = data.world.get_mut(x, y) {
                let kind = if shared_state.mouse_info.left_mouse_down {
                    PieceKind::Sand
                } else if shared_state.mouse_info.right_mouse_down {
                    PieceKind::Water
                } else {
                    PieceKind::Air
                };
                piece.kind = kind;
            } else {
                panic!("Mouse out of bounds: ({}, {})", x, y);
            }
        }

        while self.fixed_update_runner.has_gas() {
            self.fixed_update_runner.consume();
            self.update_simulation(shared_state);
        }
    }

    fn render(
        &self,
        renderer: &mut dyn Renderer,
        shared_state: &SharedState<FallingSimulationData>,
        depth_base: i32,
    ) {
        let depth_base = i32::MAX - 99;
        let data = &shared_state.custom;
        format!("FallingSimulationComponent: {}s", data.secs_passed)
            .render(renderer, 0, 0, depth_base);
        format!("sands: [{}]", data.total_pieces).render(renderer, 0, 1, depth_base);

        self.hb_display.render(renderer, 0, 0, depth_base);
    }
}
