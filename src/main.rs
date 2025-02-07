//! Demonstrates how to match on modifiers like: Control, alt, shift.
//!
//! cargo run --example event-poll-read

mod game;
mod physics;

use crate::game::components::elevator::ElevatorComponent;
use crate::game::components::incremental::falling::FallingSimulationComponent;
use crate::game::components::video::VideoComponent;
use crate::game::components::{
    incremental, video, ClearComponent, DebugInfoComponent, DecayComponent, FPSLockerComponent,
    FloodFillComponent, ForceApplyComponent, KeyPressRecorderComponent, MouseTrackerComponent,
    PhysicsComponent, PlayerComponent, QuitterComponent, SimpleDrawComponent,
};
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
use std::io::{stdout, Stdout, Write};
use std::ops::Deref;
use std::thread::sleep;
use std::time::Instant;
use std::{io, time::Duration};
use crate::game::components::incremental::boundschecker::BoundsCheckerComponent;
use crate::game::seeds::set_seed;

const HELP: &str = r#"Blocking poll() & non-blocking read()
 - Keyboard, mouse and terminal resize events enabled
 - Prints "." every second if there's no event
 - Hit "c" to print current cursor position
 - Use Esc to quit
"#;

const MAX_WIDTH: usize = 600;
const MAX_HEIGHT: usize = 400;

struct BoardWriter {
    board: [[u8; MAX_WIDTH]; MAX_HEIGHT],
    width: usize,
    height: usize,
}

impl std::io::Write for BoardWriter {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let mut x = 0;
        let mut y = 0;
        for &byte in buf {
            if byte == b'\n' {
                x = 0;
                y += 1;
            } else {
                if x < self.width && y < self.height {
                    self.board[y][x] = byte;
                }
                x += 1;
            }
        }
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        let mut stdout = stdout();
        for y in 0..self.height {
            stdout.write_all(&self.board[y][0..self.width])?;
            if y < self.height - 1 {
                println!();
            }
        }
        stdout.flush()?;
        Ok(())
    }
}

/// Custom buffer writer that _only_ flushes explicitly
/// Surprisingly leads to a speedup from 2000 fps to 4800 fps on a full screen terminal
/// Update: Since diff rendering, there is no big difference between this and Stdout directly.
struct CustomBufWriter {
    buf: Vec<u8>,
    flush_num: usize,
    stdout: Stdout,
}

impl CustomBufWriter {
    fn new() -> Self {
        Self {
            buf: vec![],
            flush_num: 0,
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
        // self.flush_num += 1;
        // eprintln!("Flushed {}", self.flush_num);
        Ok(())
    }
}

/// Returns whether a flood fill happened or not
fn flood_fill(board: &mut Vec<Vec<bool>>) -> bool {
    // determine inaccessible regions starting from the border. 'true' determines a line and must
    // not be crossed.
    let mut inaccessible = vec![vec![true; board[0].len()]; board.len()];
    let mut visited = vec![vec![false; board[0].len()]; board.len()];
    let mut stack = vec![];
    for x in 0..board[0].len() {
        if !board[0][x] {
            stack.push((0, x as i32));
        }
        if !board[board.len() - 1][x] {
            stack.push((board.len() as i32 - 1, x as i32));
        }
    }
    for y in 0..board.len() {
        if !board[y][0] {
            stack.push((y as i32, 0));
        }
        if !board[y][board[0].len() - 1] {
            stack.push((y as i32, board[0].len() as i32 - 1));
        }
    }

    while let Some((y, x)) = stack.pop() {
        if y < 0 || y >= board.len() as i32 || x < 0 || x >= board[0].len() as i32 {
            // oob, skip
            continue;
        }
        let x = x as usize;
        let y = y as usize;
        if board[y][x] {
            // wall, skip
            continue;
        }
        if visited[y][x] {
            // visited, skip
            continue;
        }
        visited[y][x] = true;
        inaccessible[y][x] = false;
        stack.push((y as i32 - 1, x as i32));
        stack.push((y as i32 + 1, x as i32));
        stack.push((y as i32, x as i32 - 1));
        stack.push((y as i32, x as i32 + 1));
    }

    // fill inaccessible regions
    let mut flood_fill_happened = false;
    for y in 0..board.len() {
        for x in 0..board[0].len() {
            // if it's inaccessible and not part of the initial input walls
            if inaccessible[y][x] && !board[y][x] {
                board[y][x] = true;
                flood_fill_happened = true;
            }
        }
    }

    flood_fill_happened
}

fn max_color(a: char, b: char) -> char {
    let colors = ['█', '▓', '▒', '░'];
    let Some(a_index) = colors.iter().position(|&c| c == a) else {
        return b;
    };
    let Some(b_index) = colors.iter().position(|&c| c == b) else {
        return a;
    };
    colors[a_index.min(b_index)]
}

fn game_loop(stdout: &mut Stdout) -> io::Result<()> {
    let mut stdout = CustomBufWriter::new();

    let sprite = [['▁', '▄', '▁'], ['▗', '▀', '▖']];
    let sprite = Sprite::new(sprite, 0, 0);

    let mut debug_messages = vec![];
    let mut debug_line_deletion_timestamps = vec![];

    let mut board = [[' '; MAX_WIDTH]; MAX_HEIGHT];
    let (t_width, t_height) = size()?;
    let mut width = t_width as usize;
    let mut height = t_height as usize;

    let mut renderer = DisplayRenderer::new_with_sink(width, height, stdout);

    let mut physics_board = physics::PhysicsBoard::new(MAX_WIDTH);
    let mut start_drag_height = height;
    let mut start_drag_time = Instant::now();

    let mut pixel_inception_times = [[std::time::Instant::now(); MAX_WIDTH]; MAX_HEIGHT];
    let pixel_colors = ['█', '▓', '▒', '░'];
    let pixel_decay_time = Duration::from_millis(300);

    // let mut board_writer = BoardWriter {
    //     board,
    //     width,
    //     height,
    // };

    let mut current_time;
    let mut last_time = std::time::Instant::now() - Duration::from_millis(1);

    let mut write_frame_info = true;
    let mut draw_debug_messages = true;

    let mut drawing = false;
    let mut draw_char = '█';
    let mut last_mouse_pos = (0, 0);

    let mut rgb_color: [u8; 3] = [245, 200, 186];
    let mut selected_rgb_index = 0;

    let mut ff_board = vec![vec![false; width]; height];
    let mut ff_drawing = false;

    let mut last_resize_event = Some(std::time::Instant::now());

    let fps_update_interval = Duration::from_millis(300);
    let mut fps = 0.0;
    let mut target_fps = 120.0;
    let mut last_fps_time = std::time::Instant::now() - fps_update_interval;

    let mut max_frametime_time = std::time::Instant::now();
    let mut max_frametime = Duration::from_millis(0);
    loop {
        for (i, timestamp) in debug_line_deletion_timestamps.iter().enumerate().rev() {
            if std::time::Instant::now() > *timestamp {
                debug_messages = debug_messages[i + 1..].to_vec();
                debug_line_deletion_timestamps = debug_line_deletion_timestamps[i + 1..].to_vec();
                break;
            }
        }
        let mut write_debug = |s: String| {
            debug_line_deletion_timestamps.push(std::time::Instant::now() + Duration::from_secs(5));
            debug_messages.push(s);
            // if more than 5 lines, delete all but the last 5
            if debug_messages.len() > 5 {
                debug_messages = debug_messages[debug_messages.len() - 5..].to_vec();
                debug_line_deletion_timestamps = debug_line_deletion_timestamps
                    [debug_line_deletion_timestamps.len() - 5..]
                    .to_vec();
            }
        };

        // board = [[b' '; MAX_WIDTH]; MAX_HEIGHT];
        // queue!(stdout, cursor::MoveTo(0, 0))?;
        current_time = std::time::Instant::now();

        if current_time - max_frametime_time > Duration::from_secs(5) {
            max_frametime = Duration::from_millis(0);
        }
        let delta_time = current_time - last_time;
        let delta_time_ns = delta_time.as_nanos();
        if delta_time > max_frametime {
            max_frametime = delta_time;
            max_frametime_time = current_time;
        }

        let frametime_string = format!("Frame time: {} ns", delta_time_ns);
        let max_frametime_string =
            format!("Max frame time (past 5s): {} ns", max_frametime.as_nanos());
        if current_time - last_fps_time > fps_update_interval {
            last_fps_time = current_time;
            if fps == 0.0 {
                fps = 1_000_000_000_f64 / delta_time_ns as f64;
            } else {
                fps = 0.5 * fps + 0.5 * (1_000_000_000_f64 / delta_time_ns as f64);
            }
        }
        let fps_string = format!("FPS: {:.2}", fps);
        let entity_count_string = format!(
            "Entities: {}",
            physics_board
                .board
                .iter()
                .map(|col| col.len())
                .sum::<usize>()
        );

        let rgb_string = format!(
            "r: {:3} g: {:3} b: {:3}",
            rgb_color[0], rgb_color[1], rgb_color[2]
        );

        // Wait up to 1s for another event
        // if poll(Duration::from_nanos(1_000_000 /*1000*/))? {
        if poll(Duration::from_nanos(0 /*1000*/))? {
            // It's guaranteed that read() won't block if `poll` returns `Ok(true)`
            let event = read()?;
            // write_debug(format!("{:?}", event));
            match event {
                Event::Resize(t_width, t_height) => {
                    width = t_width as usize;
                    height = t_height as usize;
                    last_resize_event = Some(std::time::Instant::now());
                    ff_board = vec![vec![false; width]; height];
                    renderer.resize_discard(width, height);
                }
                Event::Mouse(event) => {
                    // println!("Mouse event: {:?}\r", event);
                    last_mouse_pos = (event.column, event.row);
                    match event.kind {
                        MouseEventKind::Down(MouseButton::Left) => {
                            drawing = true;
                            start_drag_height = height;
                            start_drag_time = Instant::now();
                        }
                        MouseEventKind::Down(MouseButton::Right) => {
                            ff_drawing = true;
                        }
                        MouseEventKind::Up(MouseButton::Left) => {
                            drawing = false;
                        }
                        MouseEventKind::Up(MouseButton::Right) => {
                            ff_drawing = false;
                            for y in 0..height {
                                for x in 0..width {
                                    if ff_board[y][x] {
                                        board[y][x] = draw_char;
                                        pixel_inception_times[y][x] = std::time::Instant::now();
                                        ff_board[y][x] = false;
                                    }
                                }
                            }
                        }
                        MouseEventKind::Drag(MouseButton::Left) | MouseEventKind::Moved => {}
                        MouseEventKind::ScrollDown => {
                            rgb_color[selected_rgb_index] =
                                rgb_color[selected_rgb_index].wrapping_sub(1);
                        }
                        MouseEventKind::ScrollUp => {
                            rgb_color[selected_rgb_index] =
                                rgb_color[selected_rgb_index].wrapping_add(1);
                        }
                        MouseEventKind::ScrollLeft => {}
                        MouseEventKind::ScrollRight => {}
                        _ => {}
                    }
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('r'),
                    ..
                }) => {
                    selected_rgb_index = 0;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('g'),
                    ..
                }) => {
                    selected_rgb_index = 1;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('b'),
                    ..
                }) => {
                    selected_rgb_index = 2;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('i'),
                    ..
                }) => {
                    write_frame_info = !write_frame_info;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('d'),
                    ..
                }) => {
                    draw_debug_messages = !draw_debug_messages;
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Char('c'),
                    ..
                }) => {
                    board = [[' '; MAX_WIDTH]; MAX_HEIGHT];
                    physics_board.clear();
                }
                Event::Key(KeyEvent {
                    code: KeyCode::Esc | KeyCode::Char('q'),
                    ..
                }) => {
                    break;
                }
                _ => {}
            }
        }

        if drawing {
            let (x_t, y_t) = last_mouse_pos;
            let x = x_t as usize;
            let y = y_t as usize;
            if x < width && y < height {
                board[y][x] = draw_char;
                pixel_inception_times[y][x] = std::time::Instant::now();
            }
        }
        if ff_drawing {
            let (x_t, y_t) = last_mouse_pos;
            let x = x_t as usize;
            let y = y_t as usize;
            if x < width && y < height {
                ff_board[y][x] = true;
                if flood_fill(&mut ff_board) {
                    write_debug("Flood fill happened".to_string());
                }
            }
        }

        physics_board.update(delta_time.as_secs_f64(), height, &mut write_debug);

        for y in 0..height {
            for x in 0..width {
                if !pixel_colors.contains(&board[y][x]) {
                    continue;
                }
                let inception_time = pixel_inception_times[y][x];
                let age = current_time - inception_time;
                let age_iters = age.as_nanos() / pixel_decay_time.as_nanos();
                let age_index = pixel_colors.len().min(age_iters as usize);
                if age_index == pixel_colors.len() {
                    // turn into physics object
                    physics_board.add_entity(x, y, board[y][x]);
                    board[y][x] = ' ';
                    continue;
                }
                board[y][x] = pixel_colors[age_index];
            }
        }

        // render phase
        // renderer.render_pixel(0, 0, Pixel::new('█').with_color(rgb_color), i32::MIN);
        renderer.set_default_fg_color(rgb_color);

        for y in 0..height {
            for x in 0..width {
                let depth = if board[y][x] == ' ' { -1 } else { 10 };
                renderer.render_pixel(x, y, Pixel::new(board[y][x]), depth);
            }
        }

        for (y, col) in ff_board.iter().enumerate() {
            for (x, &val) in col.iter().enumerate() {
                if val {
                    renderer.render_pixel(x, y, Pixel::new(draw_char), 9);
                }
            }
        }

        for col in physics_board.board.iter() {
            for entity in col {
                let x = entity.x.floor() as usize;
                let y = entity.y.floor() as usize;
                if x < width && y < height {
                    renderer.render_pixel(x, y, Pixel::new(entity.c), 5);
                }
            }
        }

        let mut render_y = 0;
        let frame_info_depth = 20;
        if write_frame_info {
            frametime_string.render(&mut renderer, 0, render_y, frame_info_depth);
            render_y += 1;

            max_frametime_string.render(&mut renderer, 0, render_y, frame_info_depth);
            render_y += 1;

            fps_string.render(&mut renderer, 0, render_y, frame_info_depth);
            render_y += 1;

            entity_count_string.render(&mut renderer, 0, render_y, frame_info_depth);
            render_y += 1;

            rgb_string.render(&mut renderer, 0, render_y, frame_info_depth);
            render_y += 1;

            // draw sprite
            sprite.render(&mut renderer, 0, render_y, frame_info_depth);
            render_y += sprite.height();
        }
        let debug_msg_depth = 20;
        if draw_debug_messages {
            for line in &debug_messages {
                line.render(&mut renderer, 0, render_y, debug_msg_depth);
                render_y += 1;
            }
        }

        renderer.flush()?;
        // stdout.flush()?;

        // This leads to slowdown when events are being processed, since they can now be processed
        // at most at the fps
        // let target_s_per_frame = 1.0 / target_fps;
        // let elapsed_s = (std::time::Instant::now() - current_time).as_secs_f64();
        // if elapsed_s < target_s_per_frame {
        //     sleep(Duration::from_secs_f64(target_s_per_frame - elapsed_s));
        // }

        last_time = current_time;
    }

    Ok(())
}

fn render_loop() -> io::Result<()> {
    let (width, height) = size()?;
    let mut renderer = game::DisplayRenderer::new(width as usize, height as usize);
    let mut iters = 0;
    let mut last_size = (width, height);
    loop {
        let (width, height) = size()?;
        if (width, height) != last_size {
            renderer.resize_discard(width as usize, height as usize);
            last_size = (width, height);
        }
        for y in 0..height as usize {
            renderer.render_pixel(0, y, game::Pixel::new('█'), 1);
            renderer.render_pixel(width as usize - 1, y, game::Pixel::new('█'), 1);
        }
        for x in 0..width as usize {
            renderer.render_pixel(x, 0, game::Pixel::new('█').with_color([100, 200, 100]), 1);
            renderer.render_pixel(x, height as usize - 1, game::Pixel::new('█'), 1);
        }

        "hello".render(&mut renderer, 3, 4, 5);
        "world"
            .with_color([100, 200, 100])
            .render(&mut renderer, 3, 5, 5);

        renderer.flush()?;

        iters += 1;
        if iters > 20000 {
            return Ok(());
        }
    }
}

fn main() -> io::Result<()> {
    execute!(stdout(), crossterm::terminal::EnterAlternateScreen)?;
    // println!("{}", HELP);
    // println!("{:?}", size()?);
    // println!("{}", char::default());
    // sleep(Duration::from_secs(1));

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnableMouseCapture)?;
    // don't print cursor
    execute!(stdout, cursor::Hide)?;
    // enable keyboard enhancements
    // actually, doesnt work on windows terminal.
    // execute!(stdout, crossterm::event::PushKeyboardEnhancementFlags(KeyboardEnhancementFlags::REPORT_EVENT_TYPES | KeyboardEnhancementFlags::REPORT_ALL_KEYS_AS_ESCAPE_CODES))?;

    fn cleanup() {
        let mut stdout = io::stdout();
        execute!(stdout, DisableMouseCapture).unwrap();
        execute!(stdout, cursor::Show).unwrap();

        // show cursor
        execute!(
            stdout,
            crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
        )
        .unwrap();

        disable_raw_mode().unwrap();

        execute!(stdout, crossterm::terminal::LeaveAlternateScreen).unwrap();
    }
    // install panic handler
    std::panic::set_hook(Box::new(|pinfo| {
        cleanup();
        eprintln!("{}", pinfo);
    }));
    
    // read the seed from args or use default "42"
    let seed = std::env::args().nth(1).unwrap_or("42".to_string());
    let seed = seed.parse::<u64>().unwrap_or_else(|_| {
        // if the seed is not a number, generate a random seed
        println!("Invalid seed, using random seed");
        rand::random()
    });
    set_seed(seed);
    

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

    // if let Err(e) = game_loop(&mut stdout) {
    //     println!("Error: {:?}\r", e);
    // }
    // if let Err(e) = render_loop() {
    //     println!("Error: {:?}\r", e);
    // }

    cleanup();

    Ok(())
}
