//! A visual tool to check the functionality of `Bounds::union` and `Bounds::subtract`

use std::collections::VecDeque;
use std::io;
use std::io::stdout;
use std::time::Instant;
use teng::components::Component;
use teng::rendering::display::Display;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::Render;
use teng::rendering::renderer::Renderer;
use teng::util::planarvec::Bounds;
use teng::util::{get_lerp_t_u16, lerp_color};
use teng::{
    Game, SetupInfo, SharedState, UpdateInfo, install_panic_handler, terminal_cleanup,
    terminal_setup,
};

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new_with_custom_buf_writer();
    game.install_recommended_components();
    game.add_component(Box::new(PathFindingComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}

pub struct PathFindingComponent {
    obstacle_field: Display<bool>,
    dist_field: Display<u16>,
    direction_field: Display<(i8, i8)>,
    target: (usize, usize),
}

impl PathFindingComponent {
    pub fn new() -> Self {
        Self {
            dist_field: Display::new(0, 0, 0),
            direction_field: Display::new(0, 0, (0, 0)),
            obstacle_field: Display::new(0, 0, false),
            target: (0, 0),
        }
    }

    fn direction_to_char(dir: (i8, i8)) -> char {
        match dir {
            (0, 1) => '↓',
            (0, -1) => '↑',
            (1, 0) => '→',
            (-1, 0) => '←',
            (1, 1) => '↘',
            (1, -1) => '↗',
            (-1, 1) => '↙',
            (-1, -1) => '↖',
            _ => ' ',
        }
    }
}

impl Component for PathFindingComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<()>) {
        self.on_resize(
            setup_info.display_info.width(),
            setup_info.display_info.height(),
            shared_state,
        );
    }
    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<()>) {
        self.dist_field = Display::new(width, height, 9999);
        self.direction_field = Display::new(width, height, (0, 0));
        self.obstacle_field = Display::new(width, height, false);
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<()>) {
        let mut compute_fields = false;
        if shared_state.mouse_info.left_mouse_down {
            let new_target = shared_state.mouse_info.last_mouse_pos;
            if new_target != self.target {
                self.target = new_target;
                compute_fields = true;
            }
        }
        shared_state.mouse_events.for_each_linerp_sticky(|mi| {
            let (x, y) = mi.last_mouse_pos;
            if mi.right_mouse_down {
                if !self.obstacle_field[(x, y)] {
                    compute_fields = true;
                }
                self.obstacle_field[(x, y)] = true;
            }
            if mi.middle_mouse_down {
                if self.obstacle_field[(x, y)] {
                    compute_fields = true;
                }
                self.obstacle_field[(x, y)] = false;
            }
        });

        if compute_fields {
            self.dist_field.clear();

            // run bfs starting from target and fill dist_field
            let mut queue = VecDeque::new();
            queue.push_back((self.target, 0));
            while let Some((pos, dist)) = queue.pop_front() {
                if self.dist_field[pos] != 9999 {
                    continue;
                }
                if self.obstacle_field[pos] {
                    continue;
                }
                self.dist_field[pos] = dist;
                for (dx, dy) in &[(0, 1), (0, -1), (1, 0), (-1, 0)] {
                    let new_pos = (pos.0 as isize + dx, pos.1 as isize + dy);
                    if new_pos.0 < 0
                        || new_pos.0 >= self.dist_field.width() as isize
                        || new_pos.1 < 0
                        || new_pos.1 >= self.dist_field.height() as isize
                    {
                        continue;
                    }
                    queue.push_back(((new_pos.0 as usize, new_pos.1 as usize), dist + 1));
                }
            }

            // compute direction to go to reach target
            for y in 0..self.direction_field.height() {
                for x in 0..self.direction_field.width() {
                    let mut min_dist = 9999;
                    let mut best_dir = (0, 0);
                    // take into account diagonals
                    for (dx, dy) in &[
                        (0, 1),
                        (0, -1),
                        (1, 0),
                        (-1, 0),
                        (1, 1),
                        (1, -1),
                        (-1, 1),
                        (-1, -1),
                    ] {
                        let new_pos = (x as isize + dx, y as isize + dy);
                        if new_pos.0 < 0
                            || new_pos.0 >= self.direction_field.width() as isize
                            || new_pos.1 < 0
                            || new_pos.1 >= self.direction_field.height() as isize
                        {
                            continue;
                        }
                        let dist = self.dist_field[(new_pos.0 as usize, new_pos.1 as usize)];
                        if dist < min_dist {
                            min_dist = dist;
                            best_dir = (*dx, *dy);
                        }
                    }
                    self.direction_field[(x, y)] = (best_dir.0 as i8, best_dir.1 as i8);
                }
            }
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<()>, depth_base: i32) {
        let obstacle_depth = depth_base + 1;
        let target_depth = depth_base + 2;

        for y in 0..self.direction_field.height() {
            for x in 0..self.direction_field.width() {
                let dist = self.dist_field[(x, y)];
                // map dist to color
                // map low dist to yellow, high dist to red
                let yellow = [255, 255, 0];
                let red = [255, 0, 0];
                let t = get_lerp_t_u16(0, 300, dist);
                let color = lerp_color(yellow, red, t);

                let dir = self.direction_field[(x, y)];
                let c = Self::direction_to_char(dir);
                c.with_color(color).render(renderer, x, y, depth_base);
            }
        }

        for y in 0..self.obstacle_field.height() {
            for x in 0..self.obstacle_field.width() {
                if self.obstacle_field[(x, y)] {
                    "█".render(renderer, x, y, obstacle_depth);
                }
            }
        }

        let (target_x, target_y) = self.target;
        "X".render(renderer, target_x, target_y, target_depth);
    }
}
