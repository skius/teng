use crate::ball::Ball;
use crossterm::event::{Event, MouseEvent, MouseEventKind};
use std::io;
use teng::components::Component;
use teng::rendering::display::Display;
use teng::rendering::pixel::Pixel;
use teng::rendering::render::Render;
use teng::rendering::renderer::Renderer;
use teng::util::fixedupdate::FixedUpdateRunner;
use teng::{
    BreakingAction, Game, SetupInfo, SharedState, UpdateInfo, install_panic_handler,
    terminal_cleanup, terminal_setup,
};

mod ball {
    use teng::rendering::display::Display;
    use teng::rendering::pixel::Pixel;
    use teng::rendering::renderer::Renderer;

    #[derive(Clone)]
    pub struct Ball {
        // in local space
        x: f64,
        // in loca lspace
        y: f64,
        // in local space
        pub x_vel: f64,
        // in local space
        pub y_vel: f64,
        pub radius: f64,
        pub mass: f64,
        old_x: f64,
        old_y: f64,
    }

    impl Ball {
        pub fn new(x: f64, y: f64, radius: f64) -> Self {
            let (local_x, local_y) = Self::world_to_local(x, y);
            Self {
                x: local_x,
                y: local_y,
                x_vel: 0.0,
                y_vel: 0.0,
                radius,
                mass: radius * radius,
                old_x: local_x,
                old_y: local_y,
            }
        }

        pub fn set_world_x(&mut self, x: f64) {
            self.x = x;
        }

        pub fn set_world_y(&mut self, y: f64) {
            self.y = y * 2.0;
        }

        pub fn world_x(&self) -> f64 {
            self.x
        }

        pub fn world_y(&self) -> f64 {
            self.y / 2.0
        }

        pub fn local_x(&self) -> f64 {
            self.x
        }

        pub fn local_y(&self) -> f64 {
            self.y
        }

        pub fn world_to_local(x: f64, y: f64) -> (f64, f64) {
            (x, y * 2.0)
        }
        pub fn local_to_world(x: f64, y: f64) -> (f64, f64) {
            (x, y / 2.0)
        }

        fn update_local(&mut self, dt: f64, bottom_height_local: f64) -> (f64, f64) {
            let (old_x, old_y) = (self.x, self.y);
            self.old_x = old_x;
            self.old_y = old_y;

            self.x += self.x_vel * dt;
            self.y += self.y_vel * dt;

            if self.y + self.radius >= bottom_height_local {
                self.y = (bottom_height_local - self.radius).floor();
                self.y_vel = -self.y_vel * 0.8;
            }

            (old_x, old_y)
        }

        // calls f with local space
        fn for_each_coord_in_outline(&self, mut f: impl FnMut(f64, f64) -> bool) {
            let mut x = self.radius as i64 + 1;
            let mut y = 0;
            let mut err = 0;

            let center_x = self.x as i64;
            let center_y = self.y as i64;

            while x >= y {
                if f((center_x + x) as f64, (center_y + y) as f64) {
                    return;
                }
                if f((center_x + y) as f64, (center_y + x) as f64) {
                    return;
                }
                if f((center_x - y) as f64, (center_y + x) as f64) {
                    return;
                }
                if f((center_x - x) as f64, (center_y + y) as f64) {
                    return;
                }
                if f((center_x - x) as f64, (center_y - y) as f64) {
                    return;
                }
                if f((center_x - y) as f64, (center_y - x) as f64) {
                    return;
                }
                if f((center_x + y) as f64, (center_y - x) as f64) {
                    return;
                }
                if f((center_x + x) as f64, (center_y - y) as f64) {
                    return;
                }

                y += 1;
                if err <= 0 {
                    err += 2 * y + 1;
                }
                if err > 0 {
                    x -= 1;
                    err -= 2 * x + 1;
                }
            }
        }

        // calls f with local space
        fn for_each_coord_in_filled(
            &self,
            mut f: impl FnMut(f64, f64) -> bool,
            radius_adjustment: f64,
        ) {
            let center_x = self.x as i64;
            let center_y = self.y as i64;

            let mut x = (self.radius + radius_adjustment) as i64;
            let mut y = 0;
            let mut err = 0.0;

            while x >= y {
                for i in center_x - x..=center_x + x {
                    if f(i as f64, (center_y + y) as f64) {
                        return;
                    }
                    if f(i as f64, (center_y - y) as f64) {
                        return;
                    }
                }
                for i in center_x - y..=center_x + y {
                    if f(i as f64, (center_y + x) as f64) {
                        return;
                    }
                    if f(i as f64, (center_y - x) as f64) {
                        return;
                    }
                }

                y += 1;
                if err <= 0.0 {
                    err += 2.0 * y as f64 + 1.0;
                }
                if err > 0.0 {
                    x -= 1;
                    err -= 2.0 * x as f64 + 1.0;
                }
            }
        }

        pub fn render(&self, renderer: &mut dyn Renderer, render_outline: bool, depth: i32) {
            // rasterize the circle, filling it in
            // account for the fact that a pixel has 2:1 aspect ratio, so half the y radius
            // TODO: Fairly sure I'm redrawing some pixels

            let pixel = Pixel::new('X');

            self.for_each_coord_in_filled(
                |x, y| {
                    let (x, y) = Self::local_to_world(x, y);
                    if y < 0.0 || x < 0.0 {
                        return false;
                    }
                    renderer.render_pixel(x as usize, y as usize, pixel, depth);
                    false
                },
                0.0,
            );

            if !render_outline {
                return;
            }

            let depth_radius = depth + 1;

            // todo: think about inlining for_each_coord here?
            let pixel = Pixel::new('X').with_color([255, 0, 0]);
            self.for_each_coord_in_outline(|x, y| {
                let (x, y) = Self::local_to_world(x, y);
                if y < 0.0 || x < 0.0 {
                    return false;
                }
                renderer.render_pixel(x as usize, y as usize, pixel, depth_radius);
                false
            });
        }
    }

    pub fn update_balls(
        dt: f64,
        balls: &mut [Ball],
        bottom_wall_height_world: f64,
        is_solid_world: impl Fn(f64, f64) -> bool,
    ) {
        let bottom_wall_height_local = bottom_wall_height_world * 2.0;
        let is_solid_local = |x: f64, y: f64| is_solid_world(x, y / 2.0);

        for ball in balls.iter_mut() {
            // first handle floor collisions
            ball.update_local(dt, bottom_wall_height_local);
        }

        // then handle ball-ball collisions
        for i in 0..balls.len() {
            for j in i + 1..balls.len() {
                let (balls1, balls2) = balls.split_at_mut(j);
                let ball1 = &mut balls1[i];
                let ball2 = &mut balls2[0];
                let dx = ball1.x - ball2.x;
                let dy = ball1.y - ball2.y;
                let distance = (dx * dx + dy * dy).sqrt();
                let overlap = ball1.radius + ball2.radius - distance;
                if overlap > 0.0 {
                    let overlap = overlap / 2.0;
                    let dx = dx / distance * overlap;
                    let dy = dy / distance * overlap;
                    ball1.x += dx;
                    ball1.y += dy;
                    ball2.x -= dx;
                    ball2.y -= dy;
                    // also update velocities, but take into account the mass of each ball
                    let ball1_mass = ball1.mass;
                    let ball2_mass = ball2.mass;
                    let normal_x = dx / overlap;
                    let normal_y = dy / overlap;
                    let relative_velocity_x = ball1.x_vel - ball2.x_vel;
                    let relative_velocity_y = ball1.y_vel - ball2.y_vel;
                    let dot_product =
                        relative_velocity_x * normal_x + relative_velocity_y * normal_y;
                    if dot_product < 0.0 {
                        let impulse = 2.0 * dot_product / (ball1_mass + ball2_mass);
                        ball1.x_vel -= impulse * normal_x * ball2_mass;
                        ball1.y_vel -= impulse * normal_y * ball2_mass;
                        ball2.x_vel += impulse * normal_x * ball1_mass;
                        ball2.y_vel += impulse * normal_y * ball1_mass;
                    }
                }
            }
        }

        // then handle ball-solid collisions
        for ball in balls.iter_mut() {
            // then handle collisions with solid objects
            let mut closest_hit = None;
            let mut closest_distance_2 = f64::INFINITY;

            ball.for_each_coord_in_filled(
                |x, y| {
                    if is_solid_local(x, y) {
                        let dx = x - ball.x;
                        let dy = y - ball.y;
                        let distance = dx * dx + dy * dy;
                        if distance < closest_distance_2 {
                            closest_distance_2 = distance;
                            closest_hit = Some((x, y));
                        }
                    }

                    false
                },
                1.0,
            );

            if let Some((x, y)) = closest_hit {
                // find the closest point on the outline
                let dx = x - ball.x;
                let dy = y - ball.y;
                let distance = (dx * dx + dy * dy).sqrt();

                // undo the move that we did
                // NOTE: important to do this after ball-ball, because another ball could've moved us into the solid. and we want to undo that.
                ball.x = ball.old_x;
                ball.y = ball.old_y;

                // points from solid surface to ball
                let normal_x = -dx / distance;
                let normal_y = -dy / distance;

                // first, just move the ball out of the collision by translating it by the overlap along the normal
                // let overlap = ball.radius - distance;
                // ball.x += normal_x * overlap;
                // ball.y += normal_y * overlap;

                // bounce off against normal, reduce velocities to 80%

                let x_vel = ball.x_vel;
                let y_vel = ball.y_vel;
                let dot = x_vel * normal_x + y_vel * normal_y;
                // only if velocities are going towards collision
                if dot > 0.0 {
                    // this is to avoid immediate collision again and vanishing velocities due to stacking reductions.
                    // however, when a ball is 'stuck' on a solid object and due to gravity it thinks it's moving away,
                    // we're not actually moving away, but still skipping the velocity reduction, so our y velocity builds up infinitely due to gravity.
                    // to fix this, we need to check if we're actually moving away from the object.
                    continue;
                }

                // reflect velocities against normal
                let r_x = x_vel - 2.0 * dot * normal_x;
                let r_y = y_vel - 2.0 * dot * normal_y;

                ball.x_vel = r_x * 0.8;
                ball.y_vel = r_y * 0.8;
            }
        }
    }
}

struct CircleRasterizerComponent {
    free_balls: Vec<Ball>,
    current_ball: Option<Ball>,
    center_samples: Vec<(f64, f64)>,
    fixed_update_runner: FixedUpdateRunner,
    // TODO: add mouse_released struct to shared state
    did_hold_last: bool,
    default_radius: f64,
    static_collision: Display<bool>,
}

impl Default for CircleRasterizerComponent {
    fn default() -> Self {
        Self {
            free_balls: vec![],
            current_ball: None,
            center_samples: vec![],
            did_hold_last: false,
            fixed_update_runner: FixedUpdateRunner::new(1.0 / 60.0),
            default_radius: 10.0,
            static_collision: Display::new(0, 0, false),
        }
    }
}

const MAX_SAMPLES: usize = 5;

impl Component for CircleRasterizerComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState<()>) {
        self.on_resize(
            setup_info.display_info.width(),
            setup_info.display_info.height(),
            shared_state,
        );
    }

    fn on_resize(&mut self, width: usize, height: usize, shared_state: &mut SharedState<()>) {
        self.static_collision.resize_keep(width, height);
    }

    fn on_event(
        &mut self,
        event: Event,
        shared_state: &mut SharedState<()>,
    ) -> Option<BreakingAction> {
        if let Event::Mouse(MouseEvent {
            kind: kind @ (MouseEventKind::ScrollDown | MouseEventKind::ScrollUp),
            ..
        }) = event
        {
            let delta = match kind {
                MouseEventKind::ScrollDown => -1.0,
                MouseEventKind::ScrollUp => 1.0,
                _ => 0.0,
            };
            if let Some(current_ball) = &mut self.current_ball {
                current_ball.radius += delta;
                current_ball.mass = current_ball.radius * current_ball.radius;
            }
            self.default_radius += delta;
        }

        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<()>) {
        if shared_state.pressed_keys.did_press_char_ignore_case('c') {
            self.free_balls.clear();
        }

        if shared_state.pressed_keys.did_press_char_ignore_case('r') {
            self.static_collision.fill(false);
        }

        if shared_state.mouse_info.left_mouse_down {
            let world_x = shared_state.mouse_info.last_mouse_pos.0 as f64;
            let world_y = shared_state.mouse_info.last_mouse_pos.1 as f64;
            let current_ball = self
                .current_ball
                .get_or_insert_with(|| Ball::new(world_x, world_y, self.default_radius));

            current_ball.set_world_x(world_x);
            current_ball.set_world_y(world_y);
            current_ball.x_vel = 0.0;
            if !self.did_hold_last {
                // first time we're holding again, so we clear the samples
                self.center_samples.clear();
            }
            self.did_hold_last = true;
        } else if self.did_hold_last {
            // must have current ball
            let current_ball = self.current_ball.as_mut().unwrap();
            // just released
            self.did_hold_last = false;
            // compute a force based on average velocity over the samples
            let mut sum_x_delta = 0.0;
            let mut sum_y_delta = 0.0;
            for i in 1..self.center_samples.len() {
                let (x1, y1) = self.center_samples[i - 1];
                let (x2, y2) = self.center_samples[i];
                sum_x_delta += x2 - x1;
                sum_y_delta += y2 - y1;
            }
            let delta_length = self.center_samples.len() as f64 / 60.0;
            let avg_x_vel = sum_x_delta / delta_length;
            let avg_y_vel = sum_y_delta / delta_length;
            let strength = 1.0;
            current_ball.x_vel = avg_x_vel * strength;
            current_ball.y_vel = avg_y_vel * strength;
            // release ball
            self.free_balls.push(current_ball.clone());
            self.current_ball = None;
        }

        if let Some(current_ball) = &mut self.current_ball {
            shared_state.debug_info.custom.insert(
                "Circle Radius".to_string(),
                format!("{:.2}", current_ball.radius),
            );
            shared_state.debug_info.custom.insert(
                "Circle Center (local)".to_string(),
                format!("({}, {})", current_ball.local_x(), current_ball.local_y()),
            );
            shared_state.debug_info.custom.insert(
                "Circle Center (world)".to_string(),
                format!("({}, {})", current_ball.world_x(), current_ball.world_y()),
            );
        }

        if let Some(first_ball) = self.free_balls.first() {
            shared_state.debug_info.custom.insert(
                "First Ball Center (local)".to_string(),
                format!("({:.2}, {:.2})", first_ball.local_x(), first_ball.local_y()),
            );
            shared_state.debug_info.custom.insert(
                "First Ball Center (world)".to_string(),
                format!("({:.2}, {:.2})", first_ball.world_x(), first_ball.world_y()),
            );
            shared_state.debug_info.custom.insert(
                "First Ball velocity".to_string(),
                format!("({:.2}, {:.2})", first_ball.x_vel, first_ball.y_vel),
            );
        }

        update_balls(
            update_info.dt,
            &mut self.free_balls,
            shared_state.display_info.height() as f64,
            &self.static_collision,
        );

        self.fixed_update_runner.fuel(update_info.dt);
        while self.fixed_update_runner.has_gas() {
            self.fixed_update_runner.consume();
            if let Some(current_ball) = &mut self.current_ball {
                self.center_samples
                    .push((current_ball.local_x(), current_ball.local_y()));
                if self.center_samples.len() > MAX_SAMPLES {
                    self.center_samples.remove(0);
                }
            }
        }

        // update static collision board
        shared_state.mouse_events.for_each_linerp_only_fresh(|mi| {
            if mi.right_mouse_down {
                self.static_collision
                    .set(mi.last_mouse_pos.0, mi.last_mouse_pos.1, true);
            }
        })
    }

    fn render(&self, renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        for ball in &self.free_balls {
            ball.render(renderer, false, depth_base);
        }
        if let Some(current_ball) = &self.current_ball {
            current_ball.render(renderer, true, depth_base + 10);
        }

        // render static collision board
        for x in 0..self.static_collision.width() {
            for y in 0..self.static_collision.height() {
                if self.static_collision[(x, y)] {
                    renderer.render_pixel(
                        x,
                        y,
                        Pixel::new('O').with_color([0, 255, 0]),
                        depth_base,
                    );
                }
            }
        }
    }
}

fn update_balls(
    dt: f64,
    balls: &mut [Ball],
    bottom_wall_height: f64,
    static_collision: &Display<bool>,
) {
    for i in 0..balls.len() {
        // update velocities (TODO: move to ball module)
        let ball = &mut balls[i];
        ball.y_vel = ball.y_vel + 80.0 * dt;
        // x drag
        ball.x_vel = ball.x_vel + ball.x_vel.signum() * -10.0 * dt;

        // ball.update(dt, bottom_wall_height);
    }

    let is_solid_world = |x: f64, y: f64| {
        if x < 0.0 || y < 0.0 {
            return false;
        }
        *static_collision
            .get(x as usize, y as usize)
            .unwrap_or(&false)
    };

    ball::update_balls(dt, balls, bottom_wall_height, is_solid_world);

    //
    // // check if it hits static collision
    // // TODO: this does not work well yet. balls slowly drift through the wall
    // for ball in balls.iter_mut() {
    //     let mut closest_hit = None;
    //     let mut closest_distance_2 = f64::INFINITY;
    //
    //     ball.for_each_coord_in_filled(|x, y| {
    //         let x_u = x as usize;
    //         let y_u = y as usize;
    //
    //         if let Some(true) = static_collision.get(x_u, y_u) {
    //             let dx = x - ball.x;
    //             let dy = y - ball.y;
    //             let distance = dx*dx + dy*dy;
    //             if distance < closest_distance_2 {
    //                 closest_distance_2 = distance;
    //                 closest_hit = Some((x, y));
    //             }
    //
    //         }
    //
    //         false
    //     }, 1.0);
    //
    //     if let Some((x, y)) = closest_hit {
    //         // find the closest point on the outline
    //         let dx = x - ball.x;
    //         let dy = y - ball.y;
    //         let distance = (dx*dx + dy*dy).sqrt();
    //
    //
    //
    //         // points from solid surface to ball
    //         let normal_x = -dx / distance;
    //         let normal_y = -dy / distance;
    //
    //         // first, just move the ball out of the collision by translating it by the overlap along the normal
    //         // TODO: cannot just use radius here, since that is not the same for x and y
    //         // let overlap = ball.radius - distance;
    //         // let overlap_x = ball.radius - distance;
    //         // let overlap_y = ball.radius/2.0 - distance;
    //         // // assert!(overlap >= 0.0);
    //         // let move_by_x = (normal_x * overlap_x).round();
    //         // let move_by_y = (normal_y * overlap_y).round();
    //         // if move_by_x.abs() > 0.5 {
    //         //     ball.x += move_by_x;
    //         // }
    //         // if move_by_y.abs() > 0.5 {
    //         //     ball.y += move_by_y;
    //         // }
    //
    //         // continue;
    //
    //         // bounce off against normal, reduce velocities to 80%
    //
    //         // TODO: nonlinearity of radius y
    //
    //         let x_vel = ball.x_vel;
    //         let y_vel = ball.y_vel;
    //         let dot = x_vel * normal_x + y_vel * normal_y;
    //         // only if velocities are going towards collision
    //         if dot > 0.0 {
    //             continue;
    //         }
    //
    //         // reflect velocities against normal
    //         let r_x = x_vel - 2.0 * dot * normal_x;
    //         let r_y = y_vel - 2.0 * dot * normal_y;
    //
    //         ball.x_vel = r_x * 0.8;
    //         ball.y_vel = r_y * 0.8;
    //
    //         // // reflect velocities:
    //         // let dot_product = ball.x_vel * normal_x + ball.y_vel * normal_y;
    //         //
    //         //
    //         // // bounce off against the normal
    //         // let dot_product = ball.x_vel * normal_x + ball.y_vel * normal_y;
    //         // if dot_product < 0.0 {
    //         //     let impulse = 2.0 * dot_product / (1.0 + 1.0);
    //         //     ball.x_vel -= impulse * normal_x;
    //         //     ball.y_vel -= impulse * normal_y;
    //         // }
    //     }
    // }
    //
    // // then check each ball against each other
    // for i in 0..balls.len() {
    //     for j in i+1..balls.len() {
    //         let (balls1, balls2) = balls.split_at_mut(j);
    //         let ball1 = &mut balls1[i];
    //         let ball2 = &mut balls2[0];
    //         let dx = ball1.x - ball2.x;
    //         let dy = ball1.y - ball2.y;
    //         // account for skewed y-scale
    //         let dy = dy * 2.0;
    //         let distance = (dx*dx + dy*dy).sqrt();
    //         let overlap = ball1.radius + ball2.radius - distance;
    //         if overlap > 0.0 {
    //             let overlap = overlap / 2.0;
    //             let dx = dx / distance * overlap;
    //             let dy = dy / distance * overlap;
    //             ball1.x += dx;
    //             ball1.y += dy;
    //             ball2.x -= dx;
    //             ball2.y -= dy;
    //             // also update velocities, but take into account the mass of each ball
    //             let ball1_mass = ball1.mass;
    //             let ball2_mass = ball2.mass;
    //             let normal_x = dx / overlap;
    //             let normal_y = dy / overlap;
    //             let relative_velocity_x = ball1.x_vel - ball2.x_vel;
    //             let relative_velocity_y = ball1.y_vel - ball2.y_vel;
    //             let dot_product = relative_velocity_x * normal_x + relative_velocity_y * normal_y;
    //             if dot_product < 0.0 {
    //                 // let impulse = 2.0 * dot_product / (1.0 + 1.0);
    //                 let impulse = 2.0 * dot_product / (ball1_mass + ball2_mass);
    //                 ball1.x_vel -= impulse * normal_x * ball2_mass;
    //                 ball1.y_vel -= impulse * normal_y * ball2_mass;
    //                 ball2.x_vel += impulse * normal_x * ball1_mass;
    //                 ball2.y_vel += impulse * normal_y * ball1_mass;
    //                 // ball1.x_vel -= impulse * normal_x;
    //                 // ball1.y_vel -= impulse * normal_y;
    //                 // ball2.x_vel += impulse * normal_x;
    //                 // ball2.y_vel += impulse * normal_y;
    //             }
    //
    //         }
    //     }
    // }
}

fn main() -> io::Result<()> {
    terminal_setup()?;
    install_panic_handler();

    let mut game = Game::new_with_custom_buf_writer();
    game.install_recommended_components();
    game.add_component(Box::new(CircleRasterizerComponent::default()));
    game.run()?;

    terminal_cleanup()?;

    Ok(())
}
