use crate::game::components::incremental::ui::UiBarComponent;
use crate::game::components::incremental::GameState;
use crate::game::{
    BreakingAction, Component, Pixel, Render, Renderer, SetupInfo, SharedState, UpdateInfo,
};
use crossterm::event::{Event, KeyCode};
use noise::{NoiseFn, Simplex};
use std::iter::repeat;
use std::ops::{Index, IndexMut};
use crate::game::components::incremental::bidivec::BidiVec;
use crate::game::components::incremental::collisionboard::{CollisionBoard, CollisionCell};
use crate::game::components::incremental::planarvec::{Bounds, PlanarVec};

#[derive(Debug, Clone)]
pub struct InitializedTile {
    pub draw: Pixel,
}

#[derive(Debug, Default, Clone)]
pub enum Tile {
    #[default]
    Ungenerated,
    Initialized(InitializedTile),
}

enum Quadrant {
    TopRight,
    BottomRight,
    TopLeft,
    BottomLeft,
}

struct WorldIndex {
    q: Quadrant,
    y: usize,
    x: usize,
}

/// A world is a 2D grid of tiles.
///
/// The world is divided into four quadrants: top-left, top-right, bottom-left, and bottom-right.
/// coordinates x >= 0, y >= 0 are in the top-right quadrant
/// coordinates x < 0, y >= 0 are in the top-left quadrant
/// coordinates x >= 0, y < 0 are in the bottom-right quadrant
/// coordinates x < 0, y < 0 are in the bottom-left quadrant
///
/// Invariants:
/// * The heights (the lengths of the two tops, and the lengths of the two bottoms) are aligned
/// * The widths (the lengths of the two rights\[..] and the lengths of the two lefts\[..]) are aligned
#[derive(Debug)]
pub struct World {
    tiles: PlanarVec<Tile>,
    /// The world position at which the top-left corner of the camera is located.
    camera_attach: (i64, i64),
    screen_width: usize,
    screen_height: usize,
    /// For every x, this stores the y value of the ground level.
    ground_level: BidiVec<i64>,
    pub collision_board: CollisionBoard,
}

impl World {
    pub fn new(setup_info: &SetupInfo) -> Self {
        let screen_width = setup_info.width;
        let screen_height = setup_info.height - UiBarComponent::HEIGHT;

        let camera_attach = (0, screen_height as i64 / 2);

        let world_bounds = Bounds {
            min_x: -1,
            max_x: 1,
            min_y: -1,
            max_y: 1,
        };

        let mut world = Self {
            tiles: PlanarVec::new(world_bounds, Tile::Ungenerated),
            camera_attach,
            screen_width,
            screen_height,
            ground_level: BidiVec::new(),
            collision_board: CollisionBoard::new(world_bounds),
        };

        world.expand_to_contain(world.camera_window());
        world
    }

    pub fn camera_window(&self) -> Bounds {
        let camera_x = self.camera_attach.0;
        let camera_y = self.camera_attach.1;

        let min_x = camera_x;
        let max_x = camera_x + self.screen_width as i64 - 1;
        let min_y = camera_y - self.screen_height as i64 + 1;
        let max_y = camera_y;

        Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    pub fn to_screen_pos(&self, world_x: i64, world_y: i64) -> Option<(usize, usize)> {
        let camera_x = self.camera_attach.0;
        let camera_y = self.camera_attach.1;

        let screen_x = (world_x - camera_x) as usize;
        let screen_y = (camera_y - world_y) as usize;

        if screen_x < self.screen_width && screen_y < self.screen_height {
            Some((screen_x, screen_y))
        } else {
            None
        }
    }

    pub fn move_camera(&mut self, dx: i64, dy: i64) {
        self.camera_attach.0 += dx;
        self.camera_attach.1 += dy;
        self.expand_to_contain(self.camera_window());
    }

    /// Expands the world to at the minimum contain the given bounds.
    fn expand_to_contain(&mut self, bounds: Bounds) {
        self.tiles.expand(bounds, Tile::Ungenerated);
        self.collision_board.expand(bounds);
        self.regenerate();
    }

    fn world_bounds(&self) -> Bounds {
        self.tiles.bounds()
    }

    fn inside_world(&self, x: i64, y: i64) -> bool {
        self.world_bounds().contains(x, y)
    }

    pub fn get(&self, x: i64, y: i64) -> Option<&Tile> {
        self.tiles.get(x, y)
    }

    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut Tile> {
        self.tiles.get_mut(x, y)
    }

    pub fn regenerate(&mut self) {
        // Generates the world
        let Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        } = self.world_bounds();

        // let noise = noise::Simplex::new(42);
        let noise = noise::Fbm::<Simplex>::new(42);

        // go over entire world, find tiles that are ungenerated and generate them
        // go from left to right and generate based on noise function
        self.ground_level.grow(min_x..=max_x, 0);
        for x in min_x..=max_x {
            let noise_value = noise.get([x as f64 / 70.0, 0.0]);
            let ground_offset_height = (noise_value * 30.0) as i64;
            // from min_y to ground_offset_height, make it brown ground, above blue sky

            self.ground_level[x] = ground_offset_height;

            for y in min_y..=max_y {
                if self.get_mut(x, y).is_some_and(|t| matches!(t, Tile::Ungenerated)) {
                    let draw = if y <= ground_offset_height {
                        // ground
                        self.collision_board[(x, y)] = CollisionCell::Solid;
                        Pixel::transparent().with_bg_color([139, 69, 19])
                    } else {
                        // air
                        Pixel::transparent().with_bg_color([100, 100, 255])
                    };
                    self[(x,y)] = Tile::Initialized(InitializedTile { draw });
                }
            }
        }
    }
}

pub struct WorldComponent {}

impl WorldComponent {
    pub fn new() -> Self {
        Self {}
    }
}

impl Component for WorldComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        if let Event::Resize(width, height) = event {
            let game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
            let world = &mut game_state.world;
            world.screen_width = width as usize;
            world.screen_height = height as usize - UiBarComponent::HEIGHT;
            world.expand_to_contain(world.camera_window());
        }

        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let world = &mut shared_state
            .extensions
            .get_mut::<GameState>()
            .unwrap()
            .world;

        if shared_state.pressed_keys.contains_key(&KeyCode::Char('r')) {
            world.regenerate();
        }
        
        world.tiles.expand(world.collision_board.bounds(), Tile::Ungenerated);

        // if shared_state.pressed_keys.contains_key(&KeyCode::Char('w')) {
        //     world.move_camera(0, 1);
        // }
        // if shared_state.pressed_keys.contains_key(&KeyCode::Char('s')) {
        //     world.move_camera(0, -1);
        // }
        // if shared_state.pressed_keys.contains_key(&KeyCode::Char('a')) {
        //     world.move_camera(-1, 0);
        // }
        // if shared_state.pressed_keys.contains_key(&KeyCode::Char('d')) {
        //     world.move_camera(1, 0);
        // }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let world = &shared_state.extensions.get::<GameState>().unwrap().world;

        let camera_x = world.camera_attach.0;
        let camera_y = world.camera_attach.1;

        let screen_width = shared_state.display_info.width();
        let screen_height = shared_state.display_info.height();
        let screen_height = screen_height - UiBarComponent::HEIGHT;

        for y in 0..screen_height {
            for x in 0..screen_width {
                let world_x = camera_x + x as i64;
                let world_y = camera_y - y as i64;

                if world_y == 0 && x == 0 {
                    // special case
                    "ground->".render(&mut renderer, x, y, depth_base);
                    continue;
                }

                if let Some(tile) = world.get(world_x, world_y) {
                    match tile {
                        Tile::Ungenerated => {
                            renderer.render_pixel(x, y, Pixel::new('u'), depth_base);
                        }
                        Tile::Initialized(tile) => {
                            renderer.render_pixel(x, y, tile.draw, depth_base);
                        }
                    }
                } else {
                    renderer.render_pixel(x, y, Pixel::new('x'), depth_base);
                }
            }
        }
    }
}

impl Index<(i64, i64)> for World {
    type Output = Tile;

    fn index(&self, (x, y): (i64, i64)) -> &Self::Output {
        self.get(x, y).unwrap()
    }
}

impl IndexMut<(i64, i64)> for World {
    fn index_mut(&mut self, (x, y): (i64, i64)) -> &mut Self::Output {
        self.get_mut(x, y).unwrap()
    }
}
