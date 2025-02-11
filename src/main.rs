//! Demonstrates how to match on modifiers like: Control, alt, shift.
//!
//! cargo run --example event-poll-read

mod game;
mod physics;

use crate::game::components::incremental::rasterize::RasterizeComponent;
use crate::game::components::{incremental, DebugInfoComponent, FPSLockerComponent, KeyPressRecorderComponent, KeypressDebouncerComponent, MouseTrackerComponent, QuitterComponent};
use crate::game::seeds::set_seed;
use crate::game::Game;
use crossterm::{
    cursor,
    event::{DisableMouseCapture, EnableMouseCapture},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::io::{stdout, Stdout, Write};
use std::time::Instant;
use crate::game::components::fpschecker::FpsCheckerComponent;
use crate::game::components::incremental::worldmap::WorldMapComponent;

/// Custom buffer writer that _only_ flushes explicitly
/// Surprisingly leads to a speedup from 2000 fps to 4800 fps on a full screen terminal
/// Update: Since diff rendering, there is no big difference between this and Stdout directly.
struct CustomBufWriter {
    buf: Vec<u8>,
    stdout: Stdout,
}

impl CustomBufWriter {
    fn new() -> Self {
        Self {
            buf: vec![],
            stdout: stdout(),
        }
    }
}

impl Write for CustomBufWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut lock = self.stdout.lock();
        lock.write_all(&self.buf)?;
        lock.flush()?;
        self.buf.clear();
        Ok(())
    }
}

fn terminal_setup() -> io::Result<()> {
    let mut stdout = stdout();

    execute!(stdout, crossterm::terminal::EnterAlternateScreen)?;
    enable_raw_mode()?;
    execute!(stdout, EnableMouseCapture)?;
    // don't print cursor
    execute!(stdout, cursor::Hide)?;

    Ok(())
}

fn cleanup() -> io::Result<()> {
    let mut stdout = stdout();
    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, cursor::Show)?;

    // show cursor
    execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    disable_raw_mode()?;

    execute!(stdout, crossterm::terminal::LeaveAlternateScreen)?;

    Ok(())
}

fn process_inputs() {
    // read the seed from args or use default "42"
    let seed = std::env::args().nth(1).unwrap_or("42".to_string());
    let seed = seed.parse::<u64>().unwrap_or_else(|_| {
        // if the seed is not a number, hash or generate random
        if seed == "random" {
            return rand::random();
        } else {
            let mut hasher = DefaultHasher::new();
            seed.hash(&mut hasher);
            hasher.finish()
        }
    });
    set_seed(seed);
}

fn fps_test() {
    let mut max_x = 252;
    let mut stdout = stdout().lock();
    let mut curr_x = 0;
    let mut target_fps = 144.0;
    let mut frame_time = 1.0 / target_fps;
    let mut last_time = Instant::now();
    let mut curr_time = Instant::now();
    loop {
        curr_time = Instant::now();
        let elapsed = curr_time - last_time;
        if elapsed.as_secs_f64() >= frame_time {
            last_time = curr_time;
            curr_x += 1;
            if curr_x >=  2 * max_x {
                curr_x = 0;
            }
            // bounce back and forth
            let num_leading_spaces = if curr_x < max_x {
                curr_x
            } else {
                2 * max_x - curr_x
            };
            for _ in 0..num_leading_spaces {
                let _ = write!(stdout, " ");
            }
            let _ = write!(stdout, "█");
            let _ = write!(stdout, "\n");
            let _ = stdout.flush();
        }
    }
}

fn main() -> io::Result<()> {
    // fps_test();
    // panic!("done");
    
    terminal_setup()?;

    // install panic handler
    std::panic::set_hook(Box::new(|pinfo| {
        cleanup().unwrap();
        eprintln!("{}", pinfo);
    }));

    process_inputs();

    let sink = CustomBufWriter::new();
    let mut game = Game::new(sink);
    game.add_component(Box::new(FPSLockerComponent::new(150.0)));
    game.add_component(Box::new(KeyPressRecorderComponent::new()));
    // needs to be early in the update loop
    game.add_component(Box::new(KeypressDebouncerComponent::new(520)));
    // game.add_component(Box::new(ClearComponent));
    game.add_component(Box::new(MouseTrackerComponent::new()));
    game.add_component(Box::new(QuitterComponent));
    // game.add_component(Box::new(ForceApplyComponent));
    // game.add_component(Box::new(PhysicsComponent::new()));
    // game.add_component(Box::new(DecayComponent::new()));
    // game.add_component_with(|width, height| Box::new(FloodFillComponent::new(width, height)));
    // game.add_component(Box::new(SimpleDrawComponent::new()));
    // game.add_component_with(|width, height| Box::new(PlayerComponent::new(1, height)));
    // game.add_component_with(|width, height| Box::new(incremental::PlayerComponent::new(1, height)));
    game.add_component(Box::new(incremental::GameComponent::new()));
    game.add_component(Box::new(DebugInfoComponent::new()));
    // game.add_component(Box::new(BoundsCheckerComponent::new()));
    // game.add_component(Box::new(VideoComponent::new()));
    // game.add_component_with(|width, height| Box::new(ElevatorComponent::new(width, height)));
    // game.add_component(Box::new(FallingSimulationComponent::new()));
    // game.add_component(Box::new(RasterizeComponent::new()));
    // game.add_component(Box::new(FpsCheckerComponent::new()));
    game.add_component(Box::new(WorldMapComponent::new(30, 30, 600, 600, 50)));

    let res = game.run();

    cleanup()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    Ok(())
}
