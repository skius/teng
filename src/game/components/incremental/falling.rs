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
    dt_budget: f64,
}

impl FallingSimulationComponent {
    const UPDATES_PER_SECOND: f64 = 100.0;
    const UPDATE_INTERVAL: f64 = 1.0 / Self::UPDATES_PER_SECOND;

    pub fn new() -> Self {
        Self {
            dt_budget: 0.0,
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
        let dt = (update_info.current_time - update_info.last_time).as_secs_f64();
        self.dt_budget += dt;
        

        while self.dt_budget >= Self::UPDATE_INTERVAL {
            self.update_simulation(shared_state);
            self.dt_budget -= Self::UPDATE_INTERVAL;
        }
    }

    fn render(&self, mut renderer: &mut dyn Renderer, shared_state: &SharedState, depth_base: i32) {
        let depth_base = i32::MAX - 99;
        let data = shared_state.extensions.get::<FallingSimulationData>().unwrap();
        format!("FallingSimulationComponent: {}", data.secs_passed).render(&mut renderer, 0, 0, depth_base);
    }
}