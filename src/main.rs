//! Demonstrates how to match on modifiers like: Control, alt, shift.
//!
//! cargo run --example event-poll-read

mod game;
mod physics;

use crate::game::components::elevator::ElevatorComponent;
use crate::game::components::incremental::boundschecker::BoundsCheckerComponent;
use crate::game::components::incremental::falling::FallingSimulationComponent;
use crate::game::components::video::VideoComponent;
use crate::game::components::{
    incremental, video, ClearComponent, DebugInfoComponent, DecayComponent, FPSLockerComponent,
    FloodFillComponent, ForceApplyComponent, KeyPressRecorderComponent, MouseTrackerComponent,
    PhysicsComponent, PlayerComponent, QuitterComponent, SimpleDrawComponent,
};
use crate::game::seeds::set_seed;
use crate::game::{DisplayRenderer, Game, Pixel, Render, Renderer, Sprite};
use crossterm::event::{KeyEvent, KeyboardEnhancementFlags, MouseButton, MouseEventKind};
use crossterm::style::{Color, Colored, Colors};
use crossterm::terminal::size;
use crossterm::{
    cursor,
    cursor::position,
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute, queue, style,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::any::Any;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io::{stdout, Stdout, Write};
use std::ops::Deref;
use std::thread::sleep;
use std::time::Instant;
use std::{io, time::Duration};

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

impl std::io::Write for CustomBufWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.buf.extend_from_slice(buf);
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.stdout.write_all(&self.buf)?;
        self.stdout.flush()?;
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

fn main() -> io::Result<()> {
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

    if let Err(e) = game.run() {
        println!("Error: {:?}", e);
    }

    cleanup()?;

    Ok(())
}
