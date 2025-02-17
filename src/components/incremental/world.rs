use crate::components::incremental::animation::Animation;
use crate::components::incremental::bidivec::BidiVec;
use crate::components::incremental::collisionboard::{CollisionBoard, CollisionCell};
use crate::components::incremental::planarvec::{Bounds, PlanarVec};
use crate::components::incremental::ui::UiBarComponent;
use crate::components::incremental::GameState;
use crate::seeds::get_u32_seed_for;
use crate::util::{get_lerp_t_i64_clamped, lerp_color};
use crate::{Component, Pixel, Render, Renderer, SetupInfo, SharedState, UpdateInfo};
use noise::{NoiseFn, Simplex};
use std::fmt::Debug;
use std::ops::{Index, IndexMut, RangeBounds};
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

struct AnimationInWorld {
    animation: Box<dyn Animation>,
    world_x: i64,
    world_y: i64,
}

impl Debug for AnimationInWorld {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AnimationInWorld")
            .field("world_x", &self.world_x)
            .field("world_y", &self.world_y)
            .finish_non_exhaustive()
    }
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
    generated_world_bounds: Bounds,
    parallax_layers: [ParallaxLayer; 3],
    parallax_mountains: [ParallaxMountains; 2],
    dirt_noise: Simplex,
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

        let parallax_layers = [
            ParallaxLayer::new(0.1, '·', 0, 80),
            ParallaxLayer::new(0.3, '.', 20, 100),
            ParallaxLayer::new(0.5, '*', 50, 150),
        ];

        let parallax_mountains = [
            ParallaxMountains::new(
                0.2,
                [43, 148, 218],
                -60,
                [
                    get_u32_seed_for("pm1.1"),
                    get_u32_seed_for("pm1.2"),
                    get_u32_seed_for("pm1.3"),
                ],
                0.6,
            ),
            ParallaxMountains::new(
                0.4,
                [62, 137, 187],
                -30,
                [
                    get_u32_seed_for("pm2.1"),
                    get_u32_seed_for("pm2.2"),
                    get_u32_seed_for("pm3.3"),
                ],
                0.8,
            ),
        ];

        let mut world = Self {
            tiles: PlanarVec::new(world_bounds, Tile::Ungenerated),
            camera_attach,
            screen_width,
            screen_height,
            ground_level: BidiVec::new(),
            collision_board: CollisionBoard::new(world_bounds),
            animations: vec![],
            world_gen: WorldGenerator::new(),
            generated_world_bounds: Bounds::empty(),
            parallax_layers,
            parallax_mountains,
            dirt_noise: Simplex::new(get_u32_seed_for("dirt_noise")),
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

    pub fn camera_center(&self) -> (i64, i64) {
        let camera_x = self.camera_attach.0;
        let camera_y = self.camera_attach.1;

        // recenter
        let center_x = camera_x + self.screen_width as i64 / 2;
        let center_y = camera_y - self.screen_height as i64 / 2;

        (center_x, center_y)
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

    /// Processes terminal resize events.
    /// The passed dimensions are the terminal's dimensions, that is, including the UI bar.
    fn on_resize(&mut self, width: usize, height: usize) {
        self.screen_width = width;
        // remove the UI bar
        self.screen_height = height - UiBarComponent::HEIGHT;
        self.expand_to_contain(self.camera_window());
    }

    /// Expands the world to at the minimum contain the given bounds.
    pub fn expand_to_contain(&mut self, bounds: Bounds) {
        let current_bounds = self.world_bounds();
        if current_bounds.contains_bounds(bounds) {
            return;
        }

        self.tiles.expand(bounds, Tile::Ungenerated);
        self.collision_board.expand(bounds);
        for pl in &mut self.parallax_layers {
            pl.expand_to_contain(bounds);
        }
        for pl in &mut self.parallax_mountains {
            pl.expand_to_contain(bounds);
        }
        self.regenerate();
    }

    fn world_bounds(&self) -> Bounds {
        self.tiles.bounds()
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

    fn regenerate_bounds(&mut self, bounds_to_regenerate: Bounds) {
        // Generates the world
        let Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        } = bounds_to_regenerate;

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

            let snow_line = self.world_gen.snow_line(x);

            for y in min_y..=max_y {
                if self
                    .get_mut(x, y)
                    .is_some_and(|t| matches!(t, Tile::Ungenerated))
                {
                    let (draw, solid) = if self.world_gen.is_solid(x, y, ground_offset_height) {
                        // ground
                        let color = lerp_color(
                            [50, 50, 50],
                            [130, 130, 130],
                            get_lerp_t_i64_clamped(-50, 30, y),
                        );
                        let mut final_pixel =
                            Pixel::new('█').with_color(color).with_bg_color(color);

                        if y >= snow_line {
                            let snow_color = [255, 255, 255];
                            final_pixel =
                                final_pixel.with_color(snow_color).with_bg_color(snow_color);
                        }

                        // Check if the ground height is below the dirt level
                        let dirt_val = self.dirt_noise.get([
                            x as f64 / dirt_wideness_factor,
                            y as f64 / (dirt_wideness_factor * 2.0),
                        ]);
                        // removal factor goes from 1.0 at y <= 0 to 0.0 at y >= 10
                        let dirt_removal_factor = 1.0 - ((y - (-5)) as f64 / 10.0).clamp(0.0, 1.0);
                        // if y is more than 6 below ground level, increase factor to 1.0 until 16 below ground
                        let below_ground_removal_factor =
                            1.0 - ((ground_offset_height - 6 - y) as f64 / 10.0).clamp(0.0, 1.0);
                        let dirt_val = dirt_val * dirt_removal_factor * below_ground_removal_factor;
                        let dirt_disabled = true;
                        if dirt_val > 0.3 && !dirt_disabled {
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
                        } else {
                            // from 100 to 300 lerp to black
                            let start_color = [0x01, 0x78, 0xc8];
                            let end_color = [0, 0, 0];
                            let t = get_lerp_t_i64_clamped(100, 300, y);
                            color = lerp_color(start_color, end_color, t);
                        }

                        (
                            Pixel::new('█').with_color(color).with_bg_color(color),
                            false,
                        )
                    };

                    if solid {
                        self.collision_board[(x, y)] = CollisionCell::Solid;
                    }

                    self[(x, y)] = Tile::Initialized(InitializedTile { draw, solid });
                }
            }
        }
    }

    pub fn regenerate(&mut self) {
        let missing_bounds = self.world_bounds().subtract(self.generated_world_bounds);
        for bounds in missing_bounds.iter() {
            if bounds.is_empty() {
                continue;
            }
            self.regenerate_bounds(*bounds);
        }
        self.generated_world_bounds = self.world_bounds();
    }
}

pub struct WorldComponent {
    parallax_enabled: bool,
}

impl WorldComponent {
    pub fn new() -> Self {
        Self {
            parallax_enabled: true,
        }
    }
}

impl Component for WorldComponent {
    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState) {
        let game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
        game_state.world.on_resize(width, height);
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        let world = &mut shared_state
            .extensions
            .get_mut::<GameState>()
            .unwrap()
            .world;

        if shared_state.pressed_keys.did_press_char_ignore_case('p') {
            self.parallax_enabled = !self.parallax_enabled;
        }

        // In case the collision board grew from physics
        world.expand_to_contain(world.collision_board.bounds());
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_parallax_stars = depth_base + 1;
        let depth_parallax_mountains = depth_base + 2;
        let depth_ground_level = depth_base + 3;

        let world = &shared_state.extensions.get::<GameState>().unwrap().world;

        let camera_x = world.camera_attach.0;
        let camera_y = world.camera_attach.1;

        let screen_width = shared_state.display_info.width();
        let screen_height = shared_state.display_info.height();
        let screen_height = screen_height - UiBarComponent::HEIGHT;

        for y in 0..screen_height {
            let world_y = camera_y - y as i64;
            // render this text latest, so we get the appropriate background color.
            // nope! by introducing bg_depth into renderer we can render it whenever we want.

            if world_y % 10 == 0 {
                format!("{:?}", world_y).render(renderer, 0, y, depth_ground_level);
            }
        }

        for x in 0..screen_width {
            let world_x = camera_x + x as i64;
            if world_x % 100 == 0 {
                format!("|{:?}", world_x).render(
                    renderer,
                    x,
                    screen_height - 1,
                    depth_ground_level,
                );
            }
        }

        for x in 0..screen_width {
            for y in 0..screen_height {
                let world_x = camera_x + x as i64;
                let world_y = camera_y - y as i64;

                if let Some(tile) = world.get(world_x, world_y) {
                    match tile {
                        Tile::Ungenerated => {
                            renderer.render_pixel(x, y, Pixel::new('u'), depth_base);
                        }
                        Tile::Initialized(tile) => {
                            renderer.render_pixel(x, y, tile.draw, depth_base);

                            // if sky, draw stars with parallax effect
                            if !tile.solid && self.parallax_enabled {
                                // reversed iter because we want to draw closest first for priority
                                for pl in world.parallax_layers.iter().rev() {
                                    if let Some(&true) = pl.get(camera_x, camera_y, x, y) {
                                        renderer.render_pixel(
                                            x,
                                            y,
                                            Pixel::new(pl.star_symbol),
                                            depth_parallax_stars,
                                        );
                                    }
                                }
                                // also for mountains
                                // reversed iter because we want to draw closest first for priority
                                for pl in world.parallax_mountains.iter().rev() {
                                    if let Some(true) = pl.get(camera_x, camera_y, x, y) {
                                        renderer.render_pixel(
                                            x,
                                            y,
                                            Pixel::new('x')
                                                .with_color(pl.color)
                                                .with_bg_color(pl.color),
                                            depth_parallax_mountains,
                                        );
                                    }
                                }
                            }
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
            let delete = animation
                .animation
                .render((screen_x, screen_y), current_time, renderer);
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
    continentalness_offset_noise: noise::Fbm<Simplex>,
    spikiness_noise: Simplex,
    spiky_offset_noise: noise::Fbm<Simplex>,
    additional_ground_offset: i64,
    squash_factor: f64,
    cheese_noise: Simplex,
    snow_noise: noise::Fbm<Simplex>,
}

impl WorldGenerator {
    fn new() -> Self {
        Self::new_with_options(
            [
                get_u32_seed_for("world_new.1"),
                get_u32_seed_for("world_new.2"),
                get_u32_seed_for("world_new.3"),
            ],
            0,
            1.0,
        )
    }

    fn new_with_options(
        seeds: [u32; 3],
        additional_ground_offset: i64,
        squash_factor: f64,
    ) -> Self {
        let mut continentalness_spline = Spline::new();
        continentalness_spline.add_point(-1.0, -80.0);
        continentalness_spline.add_point(-0.2, -20.0);
        continentalness_spline.add_point(0.0, 10.0);
        continentalness_spline.add_point(0.3, 30.0);
        continentalness_spline.add_point(0.8, 300.0);
        continentalness_spline.add_point(1.0, 400.0);

        let mut spikiness_spline = Spline::new();
        spikiness_spline.add_point(-1.0, 1.0);
        spikiness_spline.add_point(-0.7, 5.0);
        spikiness_spline.add_point(-0.2, 30.0);
        spikiness_spline.add_point(0.1, 50.0);
        spikiness_spline.add_point(0.5, 80.0);
        spikiness_spline.add_point(1.0, 120.0);

        let mut continentalness_offset_noise = noise::Fbm::<Simplex>::new(seeds[0]);
        continentalness_offset_noise.octaves = 3;

        let spikiness_noise = Simplex::new(seeds[1]);
        let mut spiky_offset_noise = noise::Fbm::<Simplex>::new(seeds[2]);
        spiky_offset_noise.octaves = 5;

        let cheese_noise = Simplex::new(get_u32_seed_for("cheese caves"));

        let mut snow_noise = noise::Fbm::<Simplex>::new(get_u32_seed_for("snow"));
        snow_noise.octaves = 4;

        Self {
            continentalness_spline,
            spikiness_spline,
            continentalness_offset_noise,
            spikiness_noise,
            spiky_offset_noise,
            additional_ground_offset,
            squash_factor,
            cheese_noise,
            snow_noise,
        }
    }

    fn is_solid(&self, x: i64, y: i64, ground_offset_height: i64) -> bool {
        // 'cheese caves'
        let wideness_factor = 100.0;
        let noise_value = self
            .cheese_noise
            .get([x as f64 / wideness_factor, 2.0 * y as f64 / wideness_factor]);

        let cave_threshold = 0.05;
        let is_cave = noise_value.abs() < cave_threshold;

        let is_cave = false;

        y <= ground_offset_height && !is_cave
    }

    fn snow_line(&self, x: i64) -> i64 {
        let wideness_factor = 100.0;
        let noise_value = self.snow_noise.get([x as f64 / wideness_factor, 0.0]);
        let snow_offset = (noise_value * 30.0) as i64;

        snow_offset + 100
    }

    fn continentalness_offset(&self, x: i64) -> f64 {
        let wideness_factor = 1000.0;

        let noise_value = self
            .continentalness_offset_noise
            .get([x as f64 / wideness_factor, 0.0]);
        let continentalness = self.continentalness_spline.get(noise_value);

        continentalness
    }

    fn spikiness(&self, x: i64) -> f64 {
        let wideness_factor = 500.0;

        let noise_value = self.spikiness_noise.get([x as f64 / wideness_factor, 0.0]);

        // TODO: higher continentalness should make spikiness more likely
        // could do that by splitting the continentalness offset noise into separate function (dont call twice per worldgen..)
        // and then using that noise to additionally index into some separate spikiness-continentalness adjustment
        // spline, whose value just gets added to the spikiness noise_value.
        // actually, could just take the continentalness offset directly and index the spline with that,
        // having something like from -inf to 80 it's 0.0, but from 80 onwards to 230 it increases to 1.0
        // because the splines currently don't clamp at the endpoints (maybe TODO?), we'd have to clamp
        // the summed noise_value (which is a spline key) ourselves.

        // let continentalness_offset = self.continentalness_offset(x);
        // let continentalness_offset = (continentalness_offset / 400.0).clamp(0.0, 1.0);
        // let noise_value = noise_value + continentalness_offset;

        let spikiness = self.spikiness_spline.get(noise_value);

        spikiness
    }

    fn spike_offset(&self, x: i64) -> f64 {
        let wideness_factor = 150.0;

        let noise_value = self
            .spiky_offset_noise
            .get([x as f64 / wideness_factor, 0.0]);
        let spikiness = self.spikiness(x);
        let spikiness_offset = noise_value * spikiness;

        spikiness_offset
    }

    fn world_height(&self, x: i64) -> i64 {
        let spikiness_offset = self.spike_offset(x);
        let continentalness_offset = self.continentalness_offset(x);

        ((spikiness_offset + continentalness_offset + self.additional_ground_offset as f64)
            * self.squash_factor) as i64
    }
}

#[derive(Debug)]
struct Spline {
    /// Pairs of key and associated values
    points: Vec<(f64, f64)>,
}

impl Spline {
    fn new() -> Self {
        Self { points: vec![] }
    }

    fn add_point(&mut self, x: f64, y: f64) {
        // keys must be increasing
        if let Some((last_x, _)) = self.points.last() {
            assert!(x > *last_x);
        }
        self.points.push((x, y));
    }

    /// Note: does not clamp. Might be a good idea to check what happens outside the range.
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

#[derive(Debug)]
struct ParallaxLayer {
    parallax_factor: f64,
    star_symbol: char,
    spawn_threshold_start: i64,
    spawn_threshold_end: i64,
    cached_board: PlanarVec<bool>,
    generated_bounds: Bounds,
    noise: Simplex,
}

impl ParallaxLayer {
    fn new(
        parallax_factor: f64,
        star_symbol: char,
        spawn_threshold_start: i64,
        spawn_threshold_end: i64,
    ) -> Self {
        Self {
            parallax_factor,
            star_symbol,
            cached_board: PlanarVec::new(Bounds::empty(), false),
            generated_bounds: Bounds::empty(),
            spawn_threshold_start,
            spawn_threshold_end,
            noise: Simplex::new(get_u32_seed_for("parallax stars")),
        }
    }

    fn to_world_pos(
        &self,
        camera_x: i64,
        camera_y: i64,
        screen_x: usize,
        screen_y: usize,
    ) -> (i64, i64) {
        // we just take the regular to_world_pos formula, but we pretend that the camera has not moved as far.

        let adjusted_c_x = camera_x as f64 * self.parallax_factor;
        let adjusted_c_y = camera_y as f64 * self.parallax_factor;
        let world_x = adjusted_c_x + screen_x as f64;
        let world_y = adjusted_c_y - screen_y as f64;
        (world_x as i64, world_y as i64)
    }

    fn get(&self, camera_x: i64, camera_y: i64, screen_x: usize, screen_y: usize) -> Option<&bool> {
        let (world_x, world_y) = self.to_world_pos(camera_x, camera_y, screen_x, screen_y);
        self.cached_board.get(world_x, world_y)
    }

    fn expand_to_contain(&mut self, world_bounds: Bounds) {
        // recompute bounds into local space
        // let bounds = Bounds {
        //     min_x: (world_bounds.min_x as f64 * self.parallax_factor).floor() as i64,
        //     max_x: (world_bounds.max_x as f64 * self.parallax_factor).ceil() as i64,
        //     min_y: (world_bounds.min_y as f64 * self.parallax_factor).floor() as i64,
        //     max_y: (world_bounds.max_y as f64 * self.parallax_factor).ceil() as i64,
        // };
        // or not? if we adjust then we sometimes don't have generated stars yet when moving right...
        let bounds = world_bounds;

        self.cached_board.expand(bounds, false);
        self.regenerate();
    }

    fn regenerate(&mut self) {
        let missing_bounds = self.cached_board.bounds().subtract(self.generated_bounds);
        for bounds in missing_bounds.iter() {
            if bounds.is_empty() {
                continue;
            }
            self.regenerate_bounds(*bounds);
        }
        self.generated_bounds = self.cached_board.bounds();
    }

    fn regenerate_bounds(&mut self, bounds_to_regenerate: Bounds) {
        // Generates the world
        let Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        } = bounds_to_regenerate;

        let star_density = 1.0;

        for x in min_x..=max_x {
            for y in min_y..=max_y {
                let noise_value = self
                    .noise
                    .get([x as f64 * star_density, y as f64 * star_density]);
                // vary threshold by y
                // 1.0 until y = 100, then 0.8 at y = 200
                // but since those y's are in world space, we need to translate to local space
                let min_threshold_world = self.spawn_threshold_start;
                let max_threshold_world = self.spawn_threshold_end;
                let min_threshold_local =
                    (min_threshold_world as f64 * self.parallax_factor).floor() as i64;
                let max_threshold_local =
                    (max_threshold_world as f64 * self.parallax_factor).ceil() as i64;

                let y_c = y.clamp(min_threshold_local, max_threshold_local);
                let yd = (y_c - min_threshold_local) as f64
                    / (max_threshold_local - min_threshold_local) as f64;
                let star_threshold = 1.0 - yd * 0.2;

                let is_star = noise_value > star_threshold;

                self.cached_board[(x, y)] = is_star;
            }
        }
    }
}

#[derive(Debug)]
struct ParallaxMountains {
    parallax_factor: f64,
    color: [u8; 3],
    generated_bounds: Bounds,
    world_bounds: Bounds,
    ground_level: BidiVec<Option<i64>>,
    world_gen: WorldGenerator,
}

impl ParallaxMountains {
    fn new(
        parallax_factor: f64,
        color: [u8; 3],
        offset: i64,
        seeds: [u32; 3],
        squash_factor: f64,
    ) -> Self {
        Self {
            parallax_factor,
            color,
            generated_bounds: Bounds::empty(),
            world_bounds: Bounds::empty(),
            world_gen: WorldGenerator::new_with_options(seeds, offset, squash_factor),
            ground_level: BidiVec::new(),
        }
    }

    fn to_world_pos(
        &self,
        camera_x: i64,
        camera_y: i64,
        screen_x: usize,
        screen_y: usize,
    ) -> (i64, i64) {
        let adjusted_c_x = camera_x as f64 * self.parallax_factor;
        let adjusted_c_y = camera_y as f64 * self.parallax_factor;
        let world_x = adjusted_c_x + screen_x as f64;
        let world_y = adjusted_c_y - screen_y as f64;
        (world_x as i64, world_y as i64)
    }

    fn get(&self, camera_x: i64, camera_y: i64, screen_x: usize, screen_y: usize) -> Option<bool> {
        let (world_x, world_y) = self.to_world_pos(camera_x, camera_y, screen_x, screen_y);

        if let Some(&Some(ground)) = self.ground_level.get(world_x) {
            if world_y <= ground {
                return Some(true);
            } else {
                return Some(false);
            }
        }

        None
    }

    fn expand_to_contain(&mut self, world_bounds: Bounds) {
        // recompute bounds into local space
        // let bounds = Bounds {
        //     min_x: (world_bounds.min_x as f64 * self.parallax_factor).floor() as i64,
        //     max_x: (world_bounds.max_x as f64 * self.parallax_factor).ceil() as i64,
        //     min_y: (world_bounds.min_y as f64 * self.parallax_factor).floor() as i64,
        //     max_y: (world_bounds.max_y as f64 * self.parallax_factor).ceil() as i64,
        // };
        // or not?
        let new_bounds = self.world_bounds.union(world_bounds);
        self.world_bounds = new_bounds;
        self.regenerate();
    }

    fn regenerate(&mut self) {
        let missing_bounds = self.world_bounds.subtract(self.generated_bounds);
        for bounds in missing_bounds.iter() {
            if bounds.is_empty() {
                continue;
            }
            self.regenerate_bounds(*bounds);
        }
        self.generated_bounds = self.world_bounds;
    }

    fn regenerate_bounds(&mut self, bounds_to_regenerate: Bounds) {
        // Generates the world
        let Bounds { min_x, max_x, .. } = bounds_to_regenerate;

        self.ground_level.grow(min_x..=max_x, None);

        for x in min_x..=max_x {
            match &mut self.ground_level[x] {
                Some(_) => {
                    // noop, we already generated this
                }
                None => {
                    let ground_offset_height = self.world_gen.world_height(x);
                    self.ground_level[x] = Some(ground_offset_height);
                }
            }
        }
    }
}
