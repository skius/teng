mod math;

use std::io;
use teng::components::Component;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::{HalfBlockDisplayRender, Render};
use teng::rendering::renderer::Renderer;
use teng::{install_panic_handler, terminal_cleanup, terminal_setup, Game, SetupInfo, SharedState, UpdateInfo};
use teng::rendering::color::Color;
use teng::util::fixedupdate::FixedUpdateRunner;
use crate::math::Vec2;

/// Ball-shaped collision entity
#[derive(Debug)]
struct Entity {
    pos: Vec2,
    vel: Vec2,
    accel: Vec2,
    radius: f64,
}

impl Entity {
    const DEFAULT_ACCEL: Vec2 = Vec2 { x: 0.0, y: -10.0 };

    fn new_at(x: f64, y: f64) -> Self {
        Self {
            pos: Vec2::new(x, y),
            vel: Vec2::new(0.0, 0.0),
            accel: Self::DEFAULT_ACCEL,
            radius: 0.5,
        }
    }
    
    fn with_velocity(self, vel: Vec2) -> Self {
        Self {
            vel,
            ..self
        }
    }
    
    #[inline]
    fn new_accel(&self) -> Vec2 {
        // Derive new acceleration from current position. Avoid using anything except the current
        // position to get error bounds from Verlet.
        Self::DEFAULT_ACCEL
    }
    
    #[inline]
    fn update(&mut self, dt: f64) {
        // velocity Verlet integration: https://en.wikipedia.org/wiki/Verlet_integration#Velocity_Verlet
        self.pos += self.vel * dt + 0.5 * self.accel * dt * dt;
        let new_accel = self.new_accel();
        self.vel += 0.5 * (self.accel + new_accel) * dt;
        self.accel = new_accel;
    }
}

#[derive(Debug, Default)]
struct GameState {
    entities: Vec<Entity>,
}

struct PhysicsComponent {
    fur: FixedUpdateRunner,
}

impl PhysicsComponent {
    fn new() -> Self {
        Self {
            fur: FixedUpdateRunner::new_from_rate_per_second(60.0),
        }
    }

    fn update_physics(&self, dt: f64, state: &mut GameState) {
        for entity in &mut state.entities {
            // Verlet
            entity.update(dt);
        }
    }
}

impl Component<GameState> for PhysicsComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        let mut total_iterations = 0;
        let mut total_duration_secs = 0.0;

        let dt = update_info.dt;
        self.fur.fuel(dt);
        while self.fur.has_gas() {
            self.fur.consume();
            let physics_dt = self.fur.fixed_dt();
            total_iterations += 1;
            let start = std::time::Instant::now();
            self.update_physics(physics_dt, &mut shared_state.custom);
            let duration = start.elapsed();
            total_duration_secs += duration.as_secs_f64();
        }
        if total_iterations > 0 {
            shared_state.debug_info.custom.insert("average_physics_tick_ms_cost".to_string(), format!("{:.5}", total_duration_secs / (total_iterations as f64) * 1000.0));
        }
    }
}

struct GameComponent {
    hbd: HalfBlockDisplayRender,
}

impl GameComponent {
    fn new() -> Self {
        Self {
            hbd: HalfBlockDisplayRender::new(0, 0),
        }
    }
}

impl Component<GameState> for GameComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<GameState>) {
        self.on_resize(setup_info.display_info.width(), setup_info.display_info.height(), shared_state);
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<GameState>) {
        self.hbd.resize_discard(width, 2 * height);
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        let height = self.hbd.height() as i64;
        let width = self.hbd.width() as i64;

        let (mouse_x, mouse_y) = shared_state.mouse_info.last_mouse_pos;
        let mouse_x = mouse_x as i64;
        let mouse_y = mouse_y as i64 * 2;
        let mouse_y = height - mouse_y;

        // add entity on mouse click
        if shared_state.mouse_info.left_mouse_down {

            // spawn 100 in a radius of 3 around the mouse
            for _ in 0..100 {
                let x = mouse_x + (rand::random::<i64>() % 3);
                let y = mouse_y + (rand::random::<i64>() % 3);
                shared_state.custom.entities.push(Entity::new_at(x as f64, y as f64).with_velocity((2.0, 0.0).into()));
            }
        }

        // render entities
        self.hbd.clear();
        for entity in &shared_state.custom.entities {
            let (x, y) = entity.pos.floor_to_i64();
            // swap y axis, entity y grows upwards
            let y = height - y;
            // ignore oob
            if x < 0 || x >= width || y < 0 || y >= height {
                continue;
            }
            self.hbd.set_color(x as usize, y as usize, Color::Rgb([255, 0, 0]));
        }
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState<GameState>, depth_base: i32) {
        self.hbd.render(renderer, 0, 0, depth_base);
    }
}

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();
    // we need to exit on panic, see TODO in teng::install_panic_handler
    let old_hook = std::panic::take_hook();
    std::panic::set_hook(Box::new(move |panic_info| {
        old_hook(panic_info);
        std::process::exit(1);
    }));


    let mut game = Game::new_with_custom_buf_writer();
    game.install_recommended_components();
    game.add_component(Box::new(GameComponent::new()));
    game.add_component(Box::new(PhysicsComponent::new()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
