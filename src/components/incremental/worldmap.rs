use crate::components::incremental::planarvec::{Bounds, PlanarVec};
use crate::components::incremental::world::Tile;
use crate::components::incremental::GameState;
use crate::rendering::color::Color;
use crate::rendering::render::{HalfBlockDisplayRender, Render};
use crate::{Component, Renderer, SetupInfo, SharedState, UpdateInfo};
use std::ops::RangeInclusive;

pub struct WorldMapComponent {
    window_width: usize,
    // height in half pixels.
    window_height: usize,
    view_width: usize,
    view_height: usize,
    // in world coordinates
    base_y: i64,
    // contains colors for the pixels
    rendered_map: PlanarVec<[u8; 3]>,
    current_bounds: Bounds,
    display: HalfBlockDisplayRender,
    display_attach_x: usize,
    display_attach_y: usize,
    attach_padding: usize,
    enabled: bool,
}

impl WorldMapComponent {
    pub fn new(
        window_width: usize,
        window_height: usize,
        view_width: usize,
        view_height: usize,
        base_y: i64,
    ) -> Self {
        // want to assert that window_width evenly divides view_width
        // want to assert that window_height evenly divides view_height
        assert_eq!(view_width % window_width, 0);
        assert_eq!(view_height % window_height, 0);

        Self {
            window_width,
            window_height,
            view_width,
            view_height,
            base_y,
            rendered_map: PlanarVec::new(Bounds::empty(), [0, 0, 0]),
            current_bounds: Bounds::empty(),
            display: HalfBlockDisplayRender::new(window_width, window_height),
            display_attach_x: 0,
            display_attach_y: 0,
            attach_padding: 6,
            enabled: true,
        }
    }

    fn world_pixels_per_window_pixel(&self) -> (usize, usize) {
        // the vertical ratio is divided by two because we're in half pixels. a square region (visually) in the
        // terminal takes up twice as many x pixels as y pixels.
        (
            self.view_width / self.window_width,
            self.view_height / self.window_height / 2,
        )
    }

    fn expand_rendered_map_around_x(&mut self, game_state: &mut GameState, x: i64) {
        let (pixels_per_window_x, pixels_per_window_y) = self.world_pixels_per_window_pixel();
        let x_map = x / pixels_per_window_x as i64;
        let range = x_map - self.window_width as i64 / 2..=x_map + self.window_width as i64 / 2;
        self.expand_rendered_map(game_state, range);
    }

    fn expand_rendered_map(&mut self, game_state: &mut GameState, range: RangeInclusive<i64>) {
        let base_y_map = self.base_y / self.world_pixels_per_window_pixel().1 as i64;

        let bounds = Bounds {
            min_x: *range.start(),
            max_x: *range.end(),
            min_y: base_y_map - self.window_height as i64 / 2,
            max_y: base_y_map + self.window_height as i64 / 2,
        };
        self.rendered_map.expand(bounds, [0, 0, 0]);

        let bounds_in_world_coords = Bounds {
            min_x: bounds.min_x * self.world_pixels_per_window_pixel().0 as i64,
            max_x: (bounds.max_x + 1) * self.world_pixels_per_window_pixel().0 as i64 - 1,
            min_y: bounds.min_y * self.world_pixels_per_window_pixel().1 as i64,
            max_y: (bounds.max_y + 1) * self.world_pixels_per_window_pixel().1 as i64 - 1,
        };
        game_state.world.expand_to_contain(bounds_in_world_coords);

        let new_bounds = self.rendered_map.bounds().subtract(self.current_bounds);
        self.current_bounds = self.rendered_map.bounds();
        for new_bounds in new_bounds {
            self.render_bounds(game_state, new_bounds);
        }
    }

    fn render_bounds(&mut self, game_state: &mut GameState, bounds: Bounds) {
        let Bounds {
            min_x,
            max_x,
            min_y,
            max_y,
        } = bounds;

        let (pixels_per_window_x, pixels_per_window_y) = self.world_pixels_per_window_pixel();
        let pixels_per_window_x = pixels_per_window_x as i64;
        let pixels_per_window_y = pixels_per_window_y as i64;

        for y in min_y..=max_y {
            for x in min_x..=max_x {
                // we want to render the window pixel (x, y).
                // this pixel is composed of the average of the colors of the pixels in the world at locations
                // (x * pixels_per_window_x, y * pixels_per_window_y) to ((x + 1) * pixels_per_window_x - 1, (y + 1) * pixels_per_window_y - 1)

                let world_x_min = x * pixels_per_window_x;
                let world_x_max = (x + 1) * pixels_per_window_x - 1;
                let world_y_min = y * pixels_per_window_y;
                let world_y_max = (y + 1) * pixels_per_window_y - 1;

                let mut color = [0, 0, 0];
                let mut count = 0;
                for world_y in world_y_min..=world_y_max {
                    for world_x in world_x_min..=world_x_max {
                        if let Some(Tile::Initialized(tile)) =
                            game_state.world.get(world_x, world_y)
                        {
                            let tile_color = match tile.draw.color {
                                Color::Rgb(c) => c,
                                // not handling transparency and other color types for now
                                _ => [0, 0, 0],
                            };
                            color[0] += tile_color[0] as i64;
                            color[1] += tile_color[1] as i64;
                            color[2] += tile_color[2] as i64;
                            count += 1;
                        }
                    }
                }
                if count > 0 {
                    color[0] /= count;
                    color[1] /= count;
                    color[2] /= count;
                }
                let color = [color[0] as u8, color[1] as u8, color[2] as u8];
                self.rendered_map[(x, y)] = color;
            }
        }
    }
}

impl Component for WorldMapComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        self.on_resize(setup_info.width, setup_info.height, shared_state);
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState) {
        self.display_attach_x = width - self.attach_padding - self.window_width;
        self.display_attach_y = self.attach_padding / 2;
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        if shared_state.pressed_keys.did_press_char_ignore_case('m') {
            self.enabled = !self.enabled;
        }
        if !self.enabled {
            return;
        }

        let mut game_state = shared_state.extensions.get_mut::<GameState>().unwrap();
        let player_x = game_state.new_player_state.entity.position.0.floor() as i64;
        let player_y = game_state.new_player_state.entity.position.1.floor() as i64;
        let camera_center_attach = game_state.world.camera_center().0;
        self.expand_rendered_map_around_x(&mut game_state, camera_center_attach);

        // render into display based on the player's position
        let player_x_map = player_x / self.world_pixels_per_window_pixel().0 as i64;
        let player_y_map = player_y / self.world_pixels_per_window_pixel().1 as i64;
        let camera_center_x_map =
            camera_center_attach / self.world_pixels_per_window_pixel().0 as i64;
        let base_y_map = self.base_y / self.world_pixels_per_window_pixel().1 as i64;
        let max_y_in_map = base_y_map + self.window_height as i64 / 2;

        for x in 0..self.window_width {
            for y in 0..self.window_height {
                let y_map = base_y_map - self.window_height as i64 / 2 + y as i64;
                let x_map = camera_center_x_map - self.window_width as i64 / 2 + x as i64;

                let screen_x = x;
                let screen_y = self.window_width - y - 1;

                if x_map == player_x_map
                    && (y_map == player_y_map || player_y_map >= max_y_in_map && screen_y == 0)
                {
                    self.display
                        .set_color(screen_x, screen_y, Color::Rgb([255, 255, 255]));
                    continue;
                }

                let color = self.rendered_map[(x_map, y_map)];
                self.display
                    .set_color(screen_x, screen_y, Color::Rgb(color));
            }
        }

        // apply a circular filter to the display, everything outside should have transparent color
        let center_x = self.window_width / 2;
        let center_y = self.window_height / 2;
        let radius = (self.window_width / 2) as i32;
        for y in 0..self.window_height {
            for x in 0..self.window_width {
                let dx = x as i32 - center_x as i32;
                let dy = y as i32 - center_y as i32;
                let distance = dx * dx + dy * dy;
                if distance >= radius * radius {
                    self.display.set_color(x, y, Color::Transparent);
                } else if distance >= (radius - 1) * (radius - 1) {
                    // self.display.set_color(x, y, Color::Rgb([58, 63, 95]));
                    // self.display.set_color(x, y, Color::Rgb([75, 46, 32]));
                    self.display.set_color(x, y, Color::Rgb([115, 94, 45]));
                }
            }
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX - 10;
        if !self.enabled {
            return;
        }
        // render the display
        self.display.render(
            renderer,
            self.display_attach_x,
            self.display_attach_y,
            depth_base,
        );
    }
}
