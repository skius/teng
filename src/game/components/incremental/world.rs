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

struct WorldBounds {
    min_x: i64,
    max_x: i64,
    min_y: i64,
    max_y: i64,
}

impl WorldBounds {
    fn contains(&self, x: i64, y: i64) -> bool {
        x >= self.min_x && x <= self.max_x && y >= self.min_y && y <= self.max_y
    }
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
    top_right: Vec<Vec<Tile>>,
    bottom_right: Vec<Vec<Tile>>,
    top_left: Vec<Vec<Tile>>,
    bottom_left: Vec<Vec<Tile>>,
    /// The world position at which the top-left corner of the camera is located.
    camera_attach: (i64, i64),
    screen_width: usize,
    screen_height: usize,
    /// For every x, this stores the y value of the ground level.
    ground_level: BidiVec<i64>,
}

impl World {
    pub fn new(setup_info: &SetupInfo) -> Self {
        let screen_width = setup_info.width;
        let screen_height = setup_info.height;

        let camera_attach = (0, screen_height as i64 / 2);

        let mut world = Self {
            top_right: vec![vec![Tile::Ungenerated; 1]; 1],
            bottom_right: vec![vec![Tile::Ungenerated; 1]; 1],
            top_left: vec![vec![Tile::Ungenerated; 1]; 1],
            bottom_left: vec![vec![Tile::Ungenerated; 1]; 1],
            camera_attach,
            screen_width,
            screen_height,
            ground_level: BidiVec::new(),
        };

        world.expand_to_contain(world.camera_window());
        world
    }

    fn camera_window(&self) -> WorldBounds {
        let camera_x = self.camera_attach.0;
        let camera_y = self.camera_attach.1;

        let min_x = camera_x;
        let max_x = camera_x + self.screen_width as i64 - 1;
        let min_y = camera_y - self.screen_height as i64 + 1;
        let max_y = camera_y;

        WorldBounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    fn move_camera(&mut self, dx: i64, dy: i64) {
        self.camera_attach.0 += dx;
        self.camera_attach.1 += dy;
        self.expand_to_contain(self.camera_window());
    }

    /// Expands the world to at the minimum contain the given bounds.
    /// If `expand_y` is true, the world height will be increased to contain the bounds.
    fn expand_to_contain(&mut self, bounds: WorldBounds) {
        let want_min_x = bounds.min_x;
        let want_max_x = bounds.max_x;
        let want_min_y = bounds.min_y;
        let want_max_y = bounds.max_y;
        let actual_bounds = self.world_bounds();

        if want_min_y < actual_bounds.min_y {
            self.expand_bottom((want_min_y - actual_bounds.min_y).abs() as usize);
        }
        if want_max_y > actual_bounds.max_y {
            self.expand_top((want_max_y - actual_bounds.max_y) as usize);
        }

        if want_min_x < actual_bounds.min_x {
            self.expand_left((want_min_x - actual_bounds.min_x).abs() as usize);
        }
        if want_max_x > actual_bounds.max_x {
            self.expand_right((want_max_x - actual_bounds.max_x) as usize);
        }
    }

    fn expand_top(&mut self, amount: usize) {
        self.top_left
            .extend(repeat(vec![Tile::Ungenerated; self.top_left[0].len()]).take(amount));
        self.top_right
            .extend(repeat(vec![Tile::Ungenerated; self.top_right[0].len()]).take(amount));
    }

    fn expand_bottom(&mut self, amount: usize) {
        self.bottom_left
            .extend(repeat(vec![Tile::Ungenerated; self.bottom_left[0].len()]).take(amount));
        self.bottom_right
            .extend(repeat(vec![Tile::Ungenerated; self.bottom_right[0].len()]).take(amount));
    }

    fn expand_right(&mut self, amount: usize) {
        for row in self.top_right.iter_mut() {
            row.extend(repeat(Tile::Ungenerated).take(amount));
        }
        for row in self.bottom_right.iter_mut() {
            row.extend(repeat(Tile::Ungenerated).take(amount));
        }
    }

    fn expand_left(&mut self, amount: usize) {
        for row in self.top_left.iter_mut() {
            row.extend(repeat(Tile::Ungenerated).take(amount));
        }
        for row in self.bottom_left.iter_mut() {
            row.extend(repeat(Tile::Ungenerated).take(amount));
        }
    }

    fn world_bounds(&self) -> WorldBounds {
        let max_y = self.top_right.len() as i64 - 1;
        let min_y = -(self.bottom_right.len() as i64);
        let max_x = self.top_right[0].len() as i64 - 1;
        let min_x = -(self.top_left[0].len() as i64);

        WorldBounds {
            min_x,
            max_x,
            min_y,
            max_y,
        }
    }

    fn inside_world(&self, x: i64, y: i64) -> bool {
        self.world_bounds().contains(x, y)
    }

    fn get_index(&self, x: i64, y: i64) -> Option<WorldIndex> {
        if !self.inside_world(x, y) {
            return None;
        }

        let (q, x, y) = match (x >= 0, y >= 0) {
            (true, true) => (Quadrant::TopRight, x as usize, y as usize),
            (true, false) => (Quadrant::BottomRight, x as usize, (-y) as usize - 1),
            (false, true) => (Quadrant::TopLeft, (-x) as usize - 1, y as usize),
            (false, false) => (Quadrant::BottomLeft, (-x) as usize - 1, (-y) as usize - 1),
        };

        Some(WorldIndex { q, x, y })
    }

    pub fn get(&self, x: i64, y: i64) -> Option<&Tile> {
        // Check bounds
        if !self.inside_world(x, y) {
            return None;
        }

        let WorldIndex { q, y, x } = self.get_index(x, y)?;

        Some(&self[q][y][x])
    }

    pub fn get_mut(&mut self, x: i64, y: i64) -> Option<&mut Tile> {
        // Check bounds
        if !self.inside_world(x, y) {
            return None;
        }

        let WorldIndex { q, y, x } = self.get_index(x, y)?;

        Some(&mut self[q][y][x])
    }

    pub fn regenerate(&mut self) {
        // Generates the world
        let WorldBounds {
            min_x,
            max_x,
            min_y,
            max_y,
        } = self.world_bounds();
        let height = max_y - min_y + 1;
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
                if let Some(tile) = self.get_mut(x, y) {
                    if let Tile::Ungenerated = tile {
                        let draw = if y <= ground_offset_height {
                            Pixel::transparent().with_bg_color([139, 69, 19])
                        } else {
                            Pixel::transparent().with_bg_color([100, 100, 255])
                        };
                        *tile = Tile::Initialized(InitializedTile { draw });
                    }
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
            world.screen_height = height as usize;
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

        if shared_state.pressed_keys.contains_key(&KeyCode::Char('w')) {
            world.move_camera(0, 1);
        }
        if shared_state.pressed_keys.contains_key(&KeyCode::Char('s')) {
            world.move_camera(0, -1);
        }
        if shared_state.pressed_keys.contains_key(&KeyCode::Char('a')) {
            world.move_camera(-1, 0);
        }
        if shared_state.pressed_keys.contains_key(&KeyCode::Char('d')) {
            world.move_camera(1, 0);
        }
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

impl Index<Quadrant> for World {
    type Output = Vec<Vec<Tile>>;

    fn index(&self, q: Quadrant) -> &Self::Output {
        match q {
            Quadrant::TopRight => &self.top_right,
            Quadrant::BottomRight => &self.bottom_right,
            Quadrant::TopLeft => &self.top_left,
            Quadrant::BottomLeft => &self.bottom_left,
        }
    }
}

impl IndexMut<Quadrant> for World {
    fn index_mut(&mut self, q: Quadrant) -> &mut Self::Output {
        match q {
            Quadrant::TopRight => &mut self.top_right,
            Quadrant::BottomRight => &mut self.bottom_right,
            Quadrant::TopLeft => &mut self.top_left,
            Quadrant::BottomLeft => &mut self.bottom_left,
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
