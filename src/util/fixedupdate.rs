//! Fixed update utilities.
//! 
//! A [`Component`]'s `update` is called once every frame. If you need logic that
//! runs at a fixed rate, you can use a [`FixedUpdateRunner`] in your component.
//! See its documentation for more information.
//!
//! [`Component`]: crate::components::Component 

/// A simple fixed update runner that accumulates time and runs fixed update at a fixed rate.
///
/// # Example
/// ```
/// use teng::util::fixedupdate::FixedUpdateRunner;
///
/// let mut runner = FixedUpdateRunner::new_from_rate_per_second(60.0);
/// let dt = 0.1; // from game loop
/// runner.fuel(dt);
/// while runner.has_gas() {
///    runner.consume();
///    // run fragment of code that should run at fixed rate
/// }
/// ```
pub struct FixedUpdateRunner {
    dt_accumulator: f64,
    fixed_dt: f64,
}

impl FixedUpdateRunner {
    /// Create a new fixed update runner that consumes `fixed_dt` amount of time every fixed update.
    pub fn new(fixed_dt: f64) -> Self {
        Self {
            dt_accumulator: 0.0,
            fixed_dt,
        }
    }

    /// Create a new fixed update runner that consumes `1.0 / rate` amount of time every fixed update.
    pub fn new_from_rate_per_second(rate: f64) -> Self {
        Self {
            dt_accumulator: 0.0,
            fixed_dt: 1.0 / rate,
        }
    }

    /// Add time to the accumulator. Call this with the delta time from the game loop.
    pub fn fuel(&mut self, dt: f64)  {
        self.dt_accumulator += dt;
    }

    /// Returns true if there is enough time in the accumulator to run a fixed update.
    pub fn has_gas(&self) -> bool {
        self.dt_accumulator >= self.fixed_dt
    }

    /// Consume the time in the accumulator. Call this when you run a fixed update.
    pub fn consume(&mut self) {
        self.dt_accumulator -= self.fixed_dt;
    }
    
    /// Available ticks to consume.
    pub fn available_ticks(&self) -> u64 {
        (self.dt_accumulator / self.fixed_dt).floor() as u64
    }
}

#[cfg(test)]
mod tests {
    use std::io::{repeat, Read};
    use std::iter::repeat_n;
    use super::*;

    #[test]
    fn test_fixed_update_runner() {
        let mut runner = FixedUpdateRunner::new_from_rate_per_second(60.0);
        
        // 120 fps of rendering, and a huge lag spike at the end (1s dt)
        let mut real_frames = vec![1.0/120.0; 120];
        real_frames.push(1.0);
        // the frames at which fixed update was called
        let mut fixed_update_calls = vec![];
        for (i, dt) in real_frames.iter().enumerate() {
            runner.fuel(*dt);
            while runner.has_gas() {
                runner.consume();
                fixed_update_calls.push(i);
            }
        }
        
        // 60 fixed updates, every second frame
        let mut expected_calls = (0..60).map(|x| x * 2 + 1).collect::<Vec<_>>();
        // and 59 frames of lag at the end (not 60 due to rounding
        expected_calls.extend(repeat_n(120, 59));
        assert_eq!(fixed_update_calls, expected_calls);
        assert_eq!(runner.available_ticks(), 0);
        runner.fuel(0.5);
        assert_eq!(runner.available_ticks(), (0.5 / (1.0 / 60.0)) as u64);
    }
}