use clap::Parser;
use std::hash::{DefaultHasher, Hash, Hasher};
use std::io;
use std::io::{stdout, Stdout, Write};
use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::time::Instant;
use teng::components::eventrecorder::{
    BenchFrameCounter, EventRecorderComponent, EventReplayerComponent, Recording,
};
use teng::components::{
    incremental, DebugInfoComponent, FPSLockerComponent, KeyPressRecorderComponent,
    KeypressDebouncerComponent, MouseTrackerComponent, QuitterComponent,
};
use teng::seeds::set_seed;
use teng::Game;

/// A game running inside the terminal.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// The seed to use for the game.
    #[clap(short, long, default_value = "42")]
    seed: String,

    /// Run a benchmark with the given recording
    #[clap(short, long)]
    benchmark: Option<PathBuf>,
}

fn process_seed(seed: String) {
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

/// Run a simple FPS test in non-raw terminal mode
#[allow(unused)]
fn fps_test() {
    let max_x = 252;
    let mut stdout = stdout().lock();
    let mut curr_x = 0;
    let target_fps = 144.0;
    let frame_time = 1.0 / target_fps;
    let mut last_time = Instant::now();
    let mut curr_time;
    loop {
        curr_time = Instant::now();
        let elapsed = curr_time - last_time;
        if elapsed.as_secs_f64() >= frame_time {
            last_time = curr_time;
            curr_x += 1;
            if curr_x >= 2 * max_x {
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

/// For benchmarking.
static FRAME_COUNT: OnceLock<usize> = OnceLock::new();

/// If we're benchmarking, append the result to the bench.csv file with the following columns:
/// - Current git hash
/// - Unix time
/// - Seed
/// - Frames
fn save_bench_result(recording_path: &Path) {
    let mut file = std::fs::OpenOptions::new()
        .append(true)
        .create(true)
        .open("bench.csv")
        .unwrap();

    let git_hash = std::process::Command::new("git")
        .args(&["rev-parse", "HEAD"])
        .output()
        .unwrap();
    let git_hash = String::from_utf8(git_hash.stdout).unwrap();
    let git_hash = git_hash.trim();

    let time = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let seed = recording_path.file_stem().unwrap().to_str().unwrap();

    let frames = FRAME_COUNT.get().unwrap();

    writeln!(file, "{},{},{},{}", git_hash, time, seed, frames).unwrap();
}

fn main() -> io::Result<()> {
    let args = Args::parse();

    // fps_test();
    // panic!("done");

    teng::terminal_setup()?;
    teng::install_panic_handler();

    process_seed(args.seed);

    let mut game = Game::new_with_custom_buf_writer();
    game.add_component(Box::new(KeyPressRecorderComponent::new()));
    game.add_component(Box::new(EventRecorderComponent::new()));

    // if we're benchmarking, run the benchmark
    if let Some(recording) = &args.benchmark {
        let recording = Recording::read_from_file(recording);
        game.add_component(Box::new(EventReplayerComponent::new(true, recording)));
        game.add_component(Box::new(BenchFrameCounter::new(|num| {
            FRAME_COUNT.set(num).unwrap()
        })));
    }

    game.add_component(Box::new(FPSLockerComponent::new(150.0)));
    // needs to be early in the update loop
    game.add_component(Box::new(KeypressDebouncerComponent::new(520)));
    game.add_component(Box::new(MouseTrackerComponent::new()));
    game.add_component(Box::new(QuitterComponent));
    game.add_component(Box::new(incremental::GameComponent::new()));
    game.add_component(Box::new(DebugInfoComponent::new()));
    // game.add_component(Box::new(BoundsCheckerComponent::new()));
    // game.add_component(Box::new(VideoComponent::new()));
    // game.add_component(Box::new(FallingSimulationComponent::new()));
    // game.add_component(Box::new(RasterizeComponent::new()));

    let res = game.run();

    teng::terminal_cleanup()?;

    if let Err(err) = res {
        println!("Error: {:?}", err);
    }

    // If we're benchmarking, report the frames
    if let Some(frame_count) = FRAME_COUNT.get() {
        println!("Frames: {}", frame_count);
    }

    if let Some(recording) = &args.benchmark {
        save_bench_result(recording);
    }

    Ok(())
}
