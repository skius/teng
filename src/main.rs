//! Demonstrates how to match on modifiers like: Control, alt, shift.
//!
//! cargo run --example event-poll-read

mod physics;

use crossterm::event::{KeyEvent, MouseButton, MouseEventKind};
use crossterm::style::{Color, Colored, Colors};
use crossterm::terminal::size;
use crossterm::{
    cursor,
    cursor::position,
    event::{poll, read, DisableMouseCapture, EnableMouseCapture, Event, KeyCode},
    execute, queue, style,
    terminal::{disable_raw_mode, enable_raw_mode},
};
use std::io::{stdout, Stdout, Write};
use std::thread::sleep;
use std::time::Instant;
use std::{io, time::Duration};

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
            if inaccessible[y][x] {
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
    let mut debug_messages = vec![];
    let mut debug_line_deletion_timestamps = vec![];

    let mut board = [[' '; MAX_WIDTH]; MAX_HEIGHT];
    let (t_width, t_height) = size()?;
    let mut width = t_width as usize;
    let mut height = t_height as usize;

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

    let mut rgb_color = [0u8, 0, 0];
    let mut selected_rgb_index = 0;

    let mut ff_board = vec![vec![false; width]; height];
    let mut ff_drawing = false;

    let mut last_resize_event = Some(std::time::Instant::now());

    let fps_update_interval = Duration::from_millis(300);
    let mut fps = 0.0;
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
        execute!(stdout, cursor::MoveTo(0, 0))?;
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
        
        let rgb_string = format!("r: {:3} g: {:3} b: {:3}", rgb_color[0], rgb_color[1], rgb_color[2]);

        // Wait up to 1s for another event
        if poll(Duration::from_nanos(1000))? {
            // It's guaranteed that read() won't block if `poll` returns `Ok(true)`
            let event = read()?;
            write_debug(format!("{:?}", event));
            match event {
                Event::Resize(t_width, t_height) => {
                    width = t_width as usize;
                    height = t_height as usize;
                    last_resize_event = Some(std::time::Instant::now());
                    ff_board = vec![vec![false; width]; height];
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
                    // for y in 0..height {
                    //     for x in 0..width {
                    //         if ff_board[y][x] {
                    //             board[y][x] = draw_char;
                    //             pixel_inception_times[y][x] = std::time::Instant::now();
                    //         }
                    //     }
                    // }
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

        // writeln!(stdout, "{}", frametime_string)?;
        // writeln!(stdout, "{}", fps_string)?;

        let mut write_board = board.clone();

        for (y, col) in ff_board.iter().enumerate() {
            for (x, &val) in col.iter().enumerate() {
                if val {
                    write_board[y][x] = draw_char;
                }
            }
        }

        for col in physics_board.board.iter() {
            for entity in col {
                let x = entity.x.floor() as usize;
                let y = entity.y.floor() as usize;
                if x < width && y < height {
                    write_board[y][x] = max_color(write_board[y][x], entity.c);
                }
            }
        }

        let mut x = 0;
        let mut y = 0;
        if write_frame_info {
            for c in frametime_string.chars() {
                if x < width && y < height {
                    write_board[y][x] = c;
                }
                x += 1;
            }
            x = 0;
            y += 1;
            for c in max_frametime_string.chars() {
                if x < width && y < height {
                    write_board[y][x] = c;
                }
                x += 1;
            }
            x = 0;
            y += 1;
            for c in fps_string.chars() {
                if x < width && y < height {
                    write_board[y][x] = c;
                }
                x += 1;
            }
            y += 1;
            x = 0;
            for c in entity_count_string.chars() {
                if x < width && y < height {
                    write_board[y][x] = c;
                }
                x += 1;
            }
            y += 1;
            x = 0;
            for c in rgb_string.chars() {
                if x < width && y < height {
                    write_board[y][x] = c;
                }
                x += 1;
            }
            y += 1;
        }

        if draw_debug_messages {
            for line in &debug_messages {
                x = 0;
                for c in line.chars() {
                    if x < width && y < height {
                        write_board[y][x] = c;
                    }
                    x += 1;
                }
                y += 1;
            }
        }

        for y in 0..height {
            for x in 0..width {
                // write!(stdout, "{}", write_board[y][x])?;
                queue!(
                    stdout,
                    style::SetColors(
                        Colored::ForegroundColor(Color::Rgb {
                            r: rgb_color[0],
                            g: rgb_color[1],
                            b: rgb_color[2]
                        })
                        .into()
                    ),
                    style::Print(write_board[y][x]),
                    style::ResetColor
                )?;
            }
            // stdout.write_all(
            //     &write_board[y][0..width]
            //         .iter()
            //         .collect::<String>()
            //         .as_bytes(),
            // )?;
            if y < height {
                queue!(stdout, cursor::MoveToNextLine(1))?;
                // execute!(stdout, cursor::MoveToNextLine(1))?;
            }
        }

        stdout.flush()?;

        last_time = current_time;
    }

    Ok(())
}

fn main() -> io::Result<()> {
    println!("{}", HELP);
    println!("{:?}", size()?);
    sleep(Duration::from_secs(1));

    enable_raw_mode()?;

    let mut stdout = io::stdout();
    execute!(stdout, EnableMouseCapture)?;
    // don't print cursor
    execute!(stdout, cursor::Hide)?;

    if let Err(e) = game_loop(&mut stdout) {
        println!("Error: {:?}\r", e);
    }

    execute!(stdout, DisableMouseCapture)?;
    execute!(stdout, cursor::Show)?;

    // show cursor
    execute!(
        stdout,
        crossterm::terminal::Clear(crossterm::terminal::ClearType::All)
    )?;

    disable_raw_mode()
}
