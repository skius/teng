use std::time::{Duration, Instant};
use crate::game::{Component, Render, Renderer, SetupInfo, SharedState, UpdateInfo};

struct FallingSimulationData {
    secs_passed: f64,
}

impl FallingSimulationData {
    fn new() -> Self {
        Self {
            secs_passed: 0.0,
        }
    }
}

pub struct FallingSimulationComponent {
    next_update: Instant,
    last_update: Instant,
}

impl FallingSimulationComponent {
    const UPDATES_PER_SECOND: f64 = 100.0;
    const UPDATE_INTERVAL: f64 = 1.0 / Self::UPDATES_PER_SECOND;

    pub fn new() -> Self {
        Self {
            next_update: Instant::now(),
            last_update: Instant::now(),
        }
    }

    fn update_simulation(&mut self, shared_state: &mut SharedState) {
        let data = shared_state.extensions.get_mut::<FallingSimulationData>().unwrap();
        data.secs_passed += Self::UPDATE_INTERVAL;
    }
}

impl Component for FallingSimulationComponent {
    fn setup(&mut self, setup_info: &SetupInfo, shared_state: &mut SharedState) {
        shared_state.extensions.insert(FallingSimulationData::new());
    }

    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState) {
        if Instant::now() < self.next_update {
            return;
        }

        // note: use 'self.last_update' as dt computation, because that is the delta since we last
        // processed.
        let mut remaining_dt = (update_info.current_time - self.last_update).as_secs_f64();
        // dt is the amount of time that has passed, and is our budget for updating the simulation
        // in case the fps is locked to eg 30, then our budget is doubled.

        while remaining_dt >= Self::UPDATE_INTERVAL {
            self.update_simulation(shared_state);
            remaining_dt -= Self::UPDATE_INTERVAL;
            // use the leftover remaining_dt (assuming we're in last iteration) to move last_update a bit farther back and give us more time
            self.last_update = Instant::now() - Duration::from_secs_f64(remaining_dt);
        }

        self.next_update += Duration::from_secs_f64(Self::UPDATE_INTERVAL);
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX - 99;
        let data = shared_state.extensions.get::<FallingSimulationData>().unwrap();
        format!("FallingSimulationComponent: {}", data.secs_passed).render(&mut renderer, 0, 0, depth_base);
    }
}