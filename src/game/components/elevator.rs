use std::time::Instant;
use crossterm::event::Event;
use crossterm::event::MouseEventKind::{ScrollDown, ScrollUp};
use rand::prelude::ThreadRng;
use rand::Rng;
use crate::game::{BreakingAction, Component, Render, Renderer, SharedState, Sprite, UpdateInfo};

pub struct ElevatorComponent {
    elevator: Elevator,
    x: usize,
    rng: ThreadRng,
    spawn_rate: f64,
    next_spawn: Instant,
}

impl ElevatorComponent {
    pub fn new(width: usize, height: usize) -> Self {
        let mut elevator = Elevator::default();
        elevator.state = ElevatorState::Stopped;
        elevator.target_queue = vec![];
        elevator.current_pos = height as f64 - 2.0;
        elevator.base_height = height as f64 - 2.0;
        let max_level = elevator.base_height as u16 / Elevator::HEIGHT_OF_FLOOR as u16 - 1;
        elevator.max_level = max_level;
        elevator.exit_target = Some(0);
        Self {
            elevator,
            x: 60,
            rng: rand::thread_rng(),
            spawn_rate: 0.3,
            next_spawn: Instant::now(),
        }
    }
}

impl Component for ElevatorComponent {
    fn on_event(&mut self, event: Event, shared_state: &mut SharedState) -> Option<BreakingAction> {
        match event {
            Event::Mouse(me) => {
                if me.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Left) {
                    let (width, height) = (me.column, me.row);
                    let height_from_floor = self.elevator.base_height - height as f64;
                    let level = ((height_from_floor as f64 + 1.0) / Elevator::HEIGHT_OF_FLOOR).floor() as Level;
                    self.elevator.exit_target = Some(level);
                }
                if me.kind == crossterm::event::MouseEventKind::Down(crossterm::event::MouseButton::Right) {
                    self.elevator.exit_target = None;
                }
                if me.kind == ScrollUp {
                    self.spawn_rate *= 1.1;
                } else if me.kind == ScrollDown {
                    self.spawn_rate /= 1.1;
                }
            }
            _ => {}
        }
        None
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {

        let height = shared_state.display_info.height();
        // TODO: this needs more changes for resizing to work properly
        let dt = update_info.current_time - update_info.last_time;
        let dt = dt.as_secs_f64();
        self.elevator.update(dt);
        shared_state.debug_info.target_queue = self.elevator.target_queue.clone();

        {
            shared_state.debug_info.elevator_info.total = self.elevator.total_members;
            shared_state.debug_info.elevator_info.total_finished = self.elevator.finished_waiting_times.len();
            shared_state.debug_info.elevator_info.total_in_elevator_now = self.elevator.members.len();
            shared_state.debug_info.elevator_info.total_waiting_now = self.elevator.waiting_members.len();
            let avg_wait_time_finished = self.elevator.finished_waiting_times.iter().sum::<f64>() / self.elevator.finished_waiting_times.len() as f64;
            shared_state.debug_info.elevator_info.avg_wait_time_finished = avg_wait_time_finished;
            let avg_wait_time_overall = (self.elevator.finished_waiting_times.iter().sum::<f64>() + self.elevator.members.iter().map(|member| member.start_time.elapsed().as_secs_f64()).sum::<f64>() + self.elevator.waiting_members.iter().map(|member| member.start_time.elapsed().as_secs_f64()).sum::<f64>() ) / self.elevator.total_members as f64;
            shared_state.debug_info.elevator_info.avg_wait_time_overall = avg_wait_time_overall;
            let mut max_wait_time = self.elevator.max_wait_time;
            for member in self.elevator.members.iter().chain(self.elevator.waiting_members.iter()) {
                let wait_time = member.start_time.elapsed().as_secs_f64();
                max_wait_time = max_wait_time.max(wait_time);
            }
            shared_state.debug_info.elevator_info.max_wait_time = max_wait_time;
            shared_state.debug_info.elevator_info.spawn_rate = self.spawn_rate;
            shared_state.debug_info.elevator_info.avg_wait_time_overall_per_spawn_rate = avg_wait_time_overall / self.spawn_rate;
        }

        // assert!(self.elevator.members.iter().all(|member| self.elevator.target_queue.contains(&member.to)), "{:?}", self.elevator);
        // assert!(self.elevator.target_queue.iter().all(|level| self.elevator.members.iter().any(|member| member.to == *level)), "{:?}", self.elevator);

        // 'dt'% chance of adding a new member
        // if self.rng.gen_bool(dt/20.0) {

        // if shared_state.pressed_keys.contains_key(&crossterm::event::KeyCode::Char('s')) {
        if Instant::now() > self.next_spawn {
            self.next_spawn = Instant::now() + std::time::Duration::from_secs_f64(1.0 / self.spawn_rate);

            let mut from = self.rng.gen_range(0..self.elevator.max_level);
            let mut to = self.rng.gen_range(0..self.elevator.max_level);
            while from == to || from == self.elevator.compute_current_level() {
                from = self.rng.gen_range(0..self.elevator.max_level);
                to = self.rng.gen_range(0..self.elevator.max_level);
            }
            self.elevator.add_waiting_member(ElevatorMember {
                from,
                to,
                start_time: Instant::now(),
            });
            // self.elevator.add_target_level(from);
            // self.elevator.add_target_level(to);
        }

        if shared_state.pressed_keys.contains_key(&crossterm::event::KeyCode::Char('c')) {
            // clear stats
            self.elevator.max_wait_time = 0.0;
            self.elevator.finished_waiting_times.clear();
            self.elevator.total_members = 0;
            self.elevator.members.clear();
            self.elevator.waiting_members.clear();
        }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let opacity_levels = [' ', '░', '▒', '▓', '█'];
        // compute door opacity
        let door_opacity = match self.elevator.state {
            ElevatorState::OpeningDoors { secs_until_opened } => {
                let opacity = (secs_until_opened / Elevator::DOOR_TIME * opacity_levels.len() as f64).floor() as usize;
                opacity_levels[opacity.min(4)]
            }
            ElevatorState::ClosingDoors { secs_until_closed } => {
                let opacity = (secs_until_closed / Elevator::DOOR_TIME * opacity_levels.len() as f64).floor() as usize;
                let opacity = opacity.min(4);
                opacity_levels[4 - opacity]
            }
            _ => '█',
        };

        let mut y = self.elevator.base_height as usize;

        // render bottom floor
        "█     ███████".render(&mut renderer, self.x, y + 1, depth_base);

        // elevator housing at x, 5 spaces, another housing (wall) at x+6
        let height_of_levels = Elevator::HEIGHT_OF_FLOOR.ceil() as usize;
        let mut curr_floor = 0;

        while y > height_of_levels {
            if curr_floor == self.elevator.max_level {
                break;
            }
            '█'.render(&mut renderer, self.x, y, depth_base);
            if self.elevator.current_level.is_some_and(|l| l == curr_floor) {
                door_opacity.render(&mut renderer, self.x + 6, y, depth_base);
            } else {
                '█'.render(&mut renderer, self.x + 6, y, depth_base);
            }
            '█'.render(&mut renderer, self.x, y-1, depth_base);
            '█'.render(&mut renderer, self.x + 6, y-1, depth_base);
            // draw the above floor
            "██████".render(&mut renderer, self.x + 7, y - 1, depth_base);
            // draw waiting members
            let mut member_str = String::new();
            for member in self.elevator.waiting_members.iter() {
                if member.from == curr_floor {
                    let c = if member.to > member.from {
                        '↑'
                    } else {
                        '↓'
                    };
                    member_str.push(c);
                }
            }
            member_str.render(&mut renderer, self.x + 7, y, depth_base+100);
            y -= height_of_levels;
            curr_floor += 1;
        }
        // draw elevator
        let elevator_depth = depth_base+1;

        let elevator_sprite = Sprite::new([
            ['█', '█', '█', '█', '█'],
            ['█', ' ', ' ', ' ', door_opacity],
            ['█', '█', '█', '█', '█'],
        ], 0, 0);


        let elevator_y = self.elevator.current_pos.floor() as usize;
        elevator_sprite.render(&mut renderer, self.x+1, elevator_y-1, elevator_depth);

        // draw members
        let member_depth = elevator_depth+1;
        let member_s = if self.elevator.members.is_empty() {
            format!("   ")
        } else {
            format!("{:3}", self.elevator.members.len())
        };
        member_s.render(&mut renderer, self.x + 2, elevator_y, member_depth);
    }
}


#[derive(Debug, Default)]
struct Elevator {
    state: ElevatorState,
    current_pos: f64,
    current_vel: f64,
    members: Vec<ElevatorMember>,
    waiting_members: Vec<ElevatorMember>,
    // invariant: must be non-empty (otherwise where is it moving to?) if state == Moving*
    target_queue: Vec<Level>,
    base_height: f64,
    // If in a stopped state (Stopped, Closing, Opening, etc), this is the current level
    current_level: Option<Level>,
    max_level: Level,
    finished_waiting_times: Vec<f64>,
    total_members: usize,
    max_wait_time: f64,
    exit_target: Option<Level>,
}

impl Elevator {
    // const HEIGHT_OF_FLOOR: f64 = 2.0;
    // const ACCELERATION: f64 = 3000.0;
    // const MAX_VELOCITY: f64 = 10000.0;
    //
    // const DOOR_TIME: f64 = 0.001;

    const HEIGHT_OF_FLOOR: f64 = 2.0;
    const ACCELERATION: f64 = 30.0;
    const MAX_VELOCITY: f64 = 100.0;

    const DOOR_TIME: f64 = 0.3;

    fn compute_current_level(&self) -> Level {
        if let Some(current_level) = self.current_level {
            return current_level;
        }
        ((self.base_height - self.current_pos) / Self::HEIGHT_OF_FLOOR) as Level
    }

    fn add_waiting_member(&mut self, member: ElevatorMember) {
        self.waiting_members.push(member);
        self.total_members += 1;
    }

    fn add_target_level(&mut self, level: Level) {
        if self.target_queue.contains(&level) {
            return;
        }
        if Some(level) == self.current_level {
            return;
        }

        if let Some(&first) = self.target_queue.first() {
            let current_level = self.compute_current_level();
            if level > self.compute_current_level() {
                // if we're going up, insert in order of ascending
                if first > current_level {
                    for i in 0..self.target_queue.len() {
                        if self.target_queue[i] < current_level {
                            // we've reached the end of the going up queue, insert it here
                            self.target_queue.insert(i, level);
                            return;
                        }
                        if self.target_queue[i] > level {
                            self.target_queue.insert(i, level);
                            return;
                        }
                    }
                    // didn't find it
                    self.target_queue.push(level);
                } else {
                    // we're going down rn, so insert it in the part of the queue that's ascending
                    for i in 0..self.target_queue.len() {
                        if self.target_queue[i] >= current_level && self.target_queue[i] > level {
                            self.target_queue.insert(i, level);
                            return;
                        }
                    }
                    // didn't find it
                    self.target_queue.push(level);
                }
            } else {
                // we want to go down.
                if first < current_level {
                    for i in 0..self.target_queue.len() {
                        if self.target_queue[i] > current_level {
                            // we've reached the end of the going down queue, insert it here
                            self.target_queue.insert(i, level);
                            return;
                        }
                        if self.target_queue[i] < level {
                            self.target_queue.insert(i, level);
                            return;
                        }
                    }
                    // didn't find it
                    self.target_queue.push(level);
                } else {
                    for i in 0..self.target_queue.len() {
                        if self.target_queue[i] <= current_level && self.target_queue[i] < level {
                            self.target_queue.insert(i, level);
                            return;
                        }
                    }
                    // didn't find it
                    self.target_queue.push(level);
                }
            }
        } else {
            self.target_queue.push(level);
        }

    }

    fn update(&mut self, dt: f64) {
        match self.state {
            ElevatorState::Stopped => {
                self.current_vel = 0.0;
                // TODO: decide to add new member?
                if let Some(&target_level) = self.target_queue.first() {
                    if self.compute_current_level() < target_level {
                        self.state = ElevatorState::MovingUp;
                        // Now that we're moving up, we can decide to stop at any floor that has a member
                        // waiting to go up.
                        for i in 0..self.waiting_members.len() {
                            if self.waiting_members[i].from > self.compute_current_level() && self.waiting_members[i].from < target_level && self.waiting_members[i].to > self.waiting_members[i].from {
                                self.add_target_level(self.waiting_members[i].from);
                            }
                        }
                    } else {
                        self.state = ElevatorState::MovingDown;
                        // Now that we're moving down, we can decide to stop at any floor that has a member
                        // waiting to go down.
                        for i in 0..self.waiting_members.len() {
                            if self.waiting_members[i].from < self.compute_current_level() && self.waiting_members[i].from > target_level && self.waiting_members[i].to < self.waiting_members[i].from {
                                self.add_target_level(self.waiting_members[i].from);
                            }
                        }
                    }
                } else {
                    // TODO: actually we need to be smarter here. if there are a whole bunch of people
                    // who want to go down, we should pick the topmost person.
                    // well. actually. right now it's first come first serve, which could also make sense.
                    if let Some(member) = self.waiting_members.first() {
                        self.add_target_level(member.from);
                    }
                }
            }
            ElevatorState::MovingUp => {
                self.current_level = None;
                let next_target = self.target_queue.first().unwrap();
                let target_y = self.base_height - (*next_target as f64) * Self::HEIGHT_OF_FLOOR;
                let decel_y = target_y + 0.5 * self.current_vel.powi(2) / Self::ACCELERATION;

                if self.current_pos <= decel_y {
                    // += because we're slowing down moving up, and moving up is negative vel
                    self.current_vel += Self::ACCELERATION * dt;
                } else {
                    self.current_vel -= Self::ACCELERATION * dt;
                    if self.current_vel.abs() > Self::MAX_VELOCITY {
                        self.current_vel = -Self::MAX_VELOCITY;
                    }
                }
                self.current_pos += self.current_vel * dt;
                if self.current_pos <= target_y {
                    self.current_pos = target_y;
                    // reached target.
                    self.current_level = Some(*next_target);
                    self.target_queue.remove(0);
                    self.state = ElevatorState::OpeningDoors { secs_until_opened: Self::DOOR_TIME };
                }
            }
            ElevatorState::MovingDown => {
                self.current_level = None;
                let next_target = self.target_queue.first().unwrap();
                let target_y = self.base_height - (*next_target as f64) * Self::HEIGHT_OF_FLOOR;
                let target_y = target_y + 0.9;
                let decel_y = target_y - 0.5 * self.current_vel.powi(2) / Self::ACCELERATION;

                if self.current_pos >= decel_y {
                    self.current_vel -= Self::ACCELERATION * dt;
                } else {
                    self.current_vel += Self::ACCELERATION * dt;
                    if self.current_vel.abs() > Self::MAX_VELOCITY {
                        self.current_vel = Self::MAX_VELOCITY;
                    }
                }
                self.current_pos += self.current_vel * dt;
                if self.current_pos >= target_y {
                    self.current_pos = target_y;
                    // reached target.
                    self.current_level = Some(*next_target);
                    self.target_queue.remove(0);
                    self.state = ElevatorState::OpeningDoors { secs_until_opened: Self::DOOR_TIME };
                }
            }
            ElevatorState::OpeningDoors { ref mut secs_until_opened } => {
                if *secs_until_opened <= 0.0 {
                    // remove any members that have reached their destination
                    let current_level = self.compute_current_level();
                    let mut did_any_leave = false;
                    self.members.retain(|&member| {
                        if member.to == current_level {
                            let wait_time_secs = member.start_time.elapsed().as_secs_f64();
                            self.finished_waiting_times.push(wait_time_secs);
                            self.max_wait_time = self.max_wait_time.max(wait_time_secs);
                            did_any_leave = true;
                            false
                        } else {
                            true
                        }
                    });
                    // collect all members that are on this floor and going in the same direction as our next movement
                    let mut check_idx = 0;
                    while !self.waiting_members.is_empty() && check_idx < self.waiting_members.len() {
                        if self.waiting_members[check_idx].from != current_level {
                            check_idx += 1;
                            continue;
                        }
                        let direction = self.target_queue.first().map(|target| {
                            if *target > current_level {
                                1
                            } else {
                                -1
                            }
                        }).unwrap_or(0);
                        if direction == 0 {
                            // we're not moving, so any member is fine.
                            let member_to_add = self.waiting_members.remove(check_idx);
                            self.members.push(member_to_add);
                            self.add_target_level(member_to_add.to);
                            // going up/down state has changed.
                            continue;
                        } else if direction == 1 {
                            // going up, only pick up members with a to level above current level
                            if self.waiting_members[check_idx].to > current_level {
                                let member_to_add = self.waiting_members.remove(check_idx);
                                self.members.push(member_to_add);
                                self.add_target_level(member_to_add.to);
                            } else {
                                check_idx += 1;
                                continue;
                            }
                        } else {
                            // going down
                            if self.waiting_members[check_idx].to < current_level {
                                let member_to_add = self.waiting_members.remove(check_idx);
                                self.members.push(member_to_add);
                                self.add_target_level(member_to_add.to);
                            } else {
                                check_idx += 1;
                                continue;
                            }
                        }
                    }
                    if did_any_leave {
                        if let Some(exit_target) = self.exit_target {
                            self.add_target_level(exit_target);
                        }
                    }
                    self.state = ElevatorState::ClosingDoors { secs_until_closed: Self::DOOR_TIME };
                } else {
                    *secs_until_opened -= dt;
                }
            }
            ElevatorState::ClosingDoors { ref mut secs_until_closed } => {
                if *secs_until_closed <= 0.0 {
                    self.state = ElevatorState::Stopped;
                } else {
                    *secs_until_closed -= dt;
                }
            }
        }
    }
}

type Level = u16;

#[derive(Debug, Default)]
enum ElevatorState {
    #[default]
    Stopped,
    // Moving towards target level
    MovingUp,
    MovingDown,
    OpeningDoors {
        secs_until_opened: f64,
    },
    ClosingDoors {
        secs_until_closed: f64,
    },
}

#[derive(Debug, Clone, Copy)]
struct ElevatorMember {
    from: Level,
    to: Level,
    start_time: Instant,
}

impl Default for ElevatorMember {
    fn default() -> Self {
        Self {
            from: 0,
            to: 0,
            start_time: Instant::now(),
        }
    }
}