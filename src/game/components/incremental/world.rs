use crate::game::components::incremental::animation::Animation;
use crate::game::components::incremental::bidivec::BidiVec;
use crate::game::components::incremental::collisionboard::{CollisionBoard, CollisionCell};
use crate::game::components::incremental::planarvec::{Bounds, PlanarVec};
use crate::game::components::incremental::ui::UiBarComponent;
use crate::game::components::incremental::GameState;
use crate::game::{
    BreakingAction, Component, Pixel, Render, Renderer, SetupInfo, SharedState, UpdateInfo,
};
use crossterm::event::{Event, KeyCode};
use noise::{NoiseFn, Perlin, Simplex};
use std::iter::repeat;
use std::ops::{Index, IndexMut};
use std::time::Instant;

#[derive(Debug, Clone)]
pub struct InitializedTile {
    pub draw: Pixel,
    pub solid: bool,
}

#[derive(Debug, Default, Clone)]
pub enum Tile {
    #[default]
    Ungenerated,
    Initialized(InitializedTile),
}

impl Tile {
    pub fn is_solid(&self) -> bool {
        match self {
            Tile::Ungenerated => false,
            Tile::Initialized(tile) => tile.solid,
        }
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

#[derive(Debug)]
struct AnimationInWorld {
    animation: Box<dyn Animation>,
    world_x: i64,
    world_y: i64,
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
    /// The height of the world's viewport, so excluding the UI bar.
    screen_height: usize,
    /// For every x, this stores the y value of the ground level.
    ground_level: BidiVec<Option<i64>>,
    pub collision_board: CollisionBoard,
    animations: Vec<AnimationInWorld>,
    world_gen: WorldGenerator,
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
            animations: vec![],
            world_gen: WorldGenerator::new(),
        };

        world.expand_to_contain(world.camera_window());
        world
    }

    pub fn add_animation(&mut self, animation: Box<dyn Animation>, world_x: i64, world_y: i64) {
        self.animations.push(AnimationInWorld {
            animation,
            world_x,
            world_y,
        });
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

    /// Converts a screen position to a world position, even if it is out of bounds.
    pub fn to_world_pos_oob(&self, screen_x: i64, screen_y: i64) -> (i64, i64) {
        let camera_x = self.camera_attach.0;
        let camera_y = self.camera_attach.1;

        let world_x = camera_x + screen_x;
        let world_y = camera_y - screen_y;

        (world_x, world_y)
    }

    /// Converts a screen position to a world position, if it is inside the world.
    pub fn to_world_pos(&self, screen_x: usize, screen_y: usize) -> Option<(i64, i64)> {
        let (world_x, world_y) = self.to_world_pos_oob(screen_x as i64, screen_y as i64);

        // Only return if the world position is inside the cameras windows (in particular, not in UI)
        if self.camera_window().contains(world_x, world_y) {
            Some((world_x, world_y))
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
        let current_bounds = self.world_bounds();
        if current_bounds.contains_bounds(bounds) {
            return;
        }

        self.tiles.expand(bounds, Tile::Ungenerated);
        self.collision_board.expand(bounds);
        // TODO: only regenerate the portions that are new.
        // Could solve this by keeping track of the generated bounds, and then only regenerating
        // the four bounds you get by subtracting the new world bounds from the generated bounds
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

    pub fn camera_follow(&mut self, x: i64, y: i64) {
        let camera_bounds = self.camera_window();
        // The camera should move if the target is less than 30% from the edge of the screen
        let x_threshold = (self.screen_width as f64 * 0.3) as i64;
        let y_threshold = (self.screen_height as f64 * 0.3) as i64;
        if x < camera_bounds.min_x + x_threshold {
            let move_by = camera_bounds.min_x - x + x_threshold;
            self.move_camera(-move_by, 0);
        } else if x > camera_bounds.max_x - x_threshold {
            let move_by = x - camera_bounds.max_x + x_threshold;
            self.move_camera(move_by, 0);
        }
        if y < camera_bounds.min_y + y_threshold {
            let move_by = camera_bounds.min_y - y + y_threshold;
            self.move_camera(0, -move_by);
        } else if y > camera_bounds.max_y - y_threshold {
            let move_by = y - camera_bounds.max_y + y_threshold;
            self.move_camera(0, move_by);
        }
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
        let mut noise = noise::Fbm::<Simplex>::new(42);
        noise.octaves = 5;

        let dirt_noise = noise::Simplex::new(42);
        // let dirt_noise = noise::Fbm::<Simplex>::new(50);

        let max_height_deviance = 60.0;
        let wideness_factor = 150.0;
        let dirt_wideness_factor = 20.0;

        // go over entire world, find tiles that are ungenerated and generate them
        // go from left to right and generate based on noise function
        self.ground_level.grow(min_x..=max_x, None);
        for x in min_x..=max_x {
            let ground_offset_height = match self.ground_level[x] {
                Some(ground_offset_height) => ground_offset_height,
                None => {
                    let ground_offset_height = self.world_gen.world_height(x);
                    self.ground_level[x] = Some(ground_offset_height);
                    ground_offset_height
                }
            };

            for y in min_y..=max_y {
                if self
                    .get_mut(x, y)
                    .is_some_and(|t| matches!(t, Tile::Ungenerated))
                {
                    let (draw, solid) = if y <= ground_offset_height {
                        // ground
                        self.collision_board[(x, y)] = CollisionCell::Solid;
                        // Pixel::new('█').with_color().with_bg_color([139, 69, 19]);
                        // make it grey:
                        let yd = y.clamp(-50, 30) as u8;
                        let color = [100 + yd, 100 + yd, 100 + yd];
                        let mut final_pixel =
                            Pixel::new('█').with_color(color).with_bg_color(color);

                        // Check if the ground height is below the dirt level
                        let dirt_val = dirt_noise.get([
                            x as f64 / dirt_wideness_factor,
                            y as f64 / (dirt_wideness_factor * 2.0),
                        ]);
                        // removal factor goes from 1.0 at y <= 0 to 0.0 at y >= 10
                        let dirt_removal_factor = 1.0 - ((y - (-5)) as f64 / 10.0).clamp(0.0, 1.0);
                        // if y is more than 6 below ground level, increase factor to 1.0 until 16 below ground
                        let below_ground_removal_factor =
                            1.0 - ((ground_offset_height - 6 - y) as f64 / 10.0).clamp(0.0, 1.0);
                        let dirt_val = dirt_val * dirt_removal_factor * below_ground_removal_factor;
                        if dirt_val > 0.3 {
                            // Add some dirt
                            let dirt_color = [139, 69, 19];
                            final_pixel =
                                final_pixel.with_color(dirt_color).with_bg_color(dirt_color);
                            if y == ground_offset_height {
                                // Add some grass
                                let grass_color = [0, 150, 0];
                                final_pixel = final_pixel
                                    .with_color(grass_color)
                                    .with_bg_color(grass_color);
                            }
                        }

                        (final_pixel, true)
                    } else {
                        // air
                        // this color in rgb: #0178c8
                        // at y >= 10 make it #0178c8
                        // until the colors are #b1d7fb, add 1 per y going down

                        let max_sky_threshold = 60;

                        let mut color: [u8; 3] = [0x01, 0x78, 0xc8];
                        if y < max_sky_threshold {
                            let dy = max_sky_threshold - y;
                            let dy = if dy > 255 { 255u8 } else { dy as u8 };
                            let dcolor = dy * 1;
                            if color[0].saturating_add(dcolor) < 0xb1 {
                                color[0] += dcolor;
                            } else {
                                color[0] = 0xb1;
                            }
                            if color[1].saturating_add(dcolor) < 0xd7 {
                                color[1] += dcolor;
                            } else {
                                color[1] = 0xd7;
                            }
                            if color[2].saturating_add(dcolor) < 0xfb {
                                color[2] += dcolor;
                            } else {
                                color[2] = 0xfb;
                            }
                        }
                        (Pixel::new('█').with_color(color).with_bg_color(color), false)
                        // Pixel::transparent().with_bg_color(color)
                    };
                    self[(x, y)] = Tile::Initialized(InitializedTile { draw, solid });
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

        // if shared_state.pressed_keys.contains_key(&KeyCode::Char('r')) {
        //     world.regenerate();
        // }

        // In case the collision board grew from physics
        world
            .tiles
            .expand(world.collision_board.bounds(), Tile::Ungenerated);

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
                    // continue;
                }

                if world_y % 10 == 0 && x == 0 {
                    // special case
                    format!("{:?}", world_y).render(&mut renderer, x, y, depth_base);
                    // continue;
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

        let current_time = Instant::now();
        for animation in &world.animations {
            let Some((screen_x, screen_y)) =
                world.to_screen_pos(animation.world_x, animation.world_y)
            else {
                continue;
            };
            let delete =
                animation
                    .animation
                    .render((screen_x, screen_y), current_time, &mut renderer);
            if delete {
                // TODO: remove animation
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

#[derive(Debug)]
struct WorldGenerator {
    continentalness_spline: Spline,
    spikiness_spline: Spline,
}

impl WorldGenerator {
    fn new() -> Self {
        let mut continentalness_spline = Spline::new();
        continentalness_spline.add_point(-1.0, -80.0);
        continentalness_spline.add_point(-0.2, -20.0);
        continentalness_spline.add_point(0.0, 10.0);
        continentalness_spline.add_point(0.3, 30.0);
        continentalness_spline.add_point(0.8, 100.0);
        continentalness_spline.add_point(1.0, 400.0);

        let mut spikiness_spline = Spline::new();
        spikiness_spline.add_point(-1.0, 1.0);
        spikiness_spline.add_point(-0.7, 5.0);
        spikiness_spline.add_point(-0.2, 30.0);
        spikiness_spline.add_point(0.1, 50.0);
        spikiness_spline.add_point(0.5, 80.0);
        spikiness_spline.add_point(1.0, 120.0);

        Self {
            continentalness_spline,
            spikiness_spline,
        }
    }

    fn continentalness_offset(&self, x: i64) -> f64 {
        let mut noise = noise::Fbm::<Simplex>::new(1234);
        noise.octaves = 3;

        let wideness_factor = 1000.0;

        let noise_value = noise.get([x as f64 / wideness_factor, 0.0]);
        let continentalness = self.continentalness_spline.get(noise_value);

        continentalness
    }

    fn spikiness(&self, x: i64) -> f64 {
        let noise = Simplex::new(4321);

        let wideness_factor = 500.0;

        let noise_value = noise.get([x as f64 / wideness_factor, 0.0]);
        let spikiness = self.spikiness_spline.get(noise_value);

        spikiness
    }

    fn spike_offset(&self, x: i64) -> f64 {
        let mut noise = noise::Fbm::<Simplex>::new(42);
        noise.octaves = 5;

        let wideness_factor = 150.0;

        let noise_value = noise.get([x as f64 / wideness_factor, 0.0]);
        let spikiness = self.spikiness(x);
        let spikiness_offset = (noise_value * spikiness);

        spikiness_offset
    }

    fn world_height(&self, x: i64) -> i64 {
        let spikiness_offset = self.spike_offset(x);
        let continentalness_offset = self.continentalness_offset(x);

        (spikiness_offset + continentalness_offset) as i64
    }
}

#[derive(Debug)]
struct Spline {
    /// Pairs of key and associated values
    points: Vec<(f64, f64)>,
}

impl Spline {
    fn new() -> Self {
        Self {
            points: vec![],
        }
    }

    fn add_point(&mut self, x: f64, y: f64) {
        // keys must be increasing
        if let Some((last_x, _)) = self.points.last() {
            assert!(x > *last_x);
        }
        self.points.push((x, y));
    }

    fn get(&self, x: f64) -> f64 {
        // Find the two points that x is between
        let mut left = 0;
        let mut right = self.points.len() - 1;
        while right - left > 1 {
            let mid = (left + right) / 2;
            if self.points[mid].0 < x {
                left = mid;
            } else {
                right = mid;
            }
        }

        let (x0, y0) = self.points[left];
        let (x1, y1) = self.points[right];

        let t = (x - x0) / (x1 - x0);
        y0 * (1.0 - t) + y1 * t
    }
}