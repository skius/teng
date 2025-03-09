use crate::sprite::{get_animation, Animation, AnimationRepositoryKey, AnimationResult, CombinedAnimations};
use std::collections::HashMap;
use std::hash::Hash;
use std::time::Instant;
use teng::rendering::render::HalfBlockDisplayRender;

pub enum KeyedAnimationResult<K> {
    Triggered(K),
    Finished(K),
}

#[derive(Debug)]
pub struct AnimationController<K: Hash + Eq> {
    animation_map: HashMap<K, CombinedAnimations>,
    current_animation: K,
    time_started: Instant,
}

impl<K: Hash + Eq + Default> Default for AnimationController<K> {
    fn default() -> Self {
        Self {
            animation_map: HashMap::new(),
            current_animation: Default::default(),
            time_started: Instant::now(),
        }
    }
}

impl<K: Hash + Eq + Copy> AnimationController<K> {
    fn current_animation(&self) -> &CombinedAnimations {
        self.animation_map.get(&self.current_animation).unwrap()
    }

    fn current_animation_mut(&mut self) -> &mut CombinedAnimations {
        self.animation_map.get_mut(&self.current_animation).unwrap()
    }

    pub fn set_flipped_x(&mut self, flipped_x: bool) {
        for (_, animation) in self.animation_map.iter_mut() {
            animation.set_flipped_x(flipped_x);
        }
    }

    pub fn is_currently_oneshot(&self) -> bool {
        self.current_animation().is_oneshot()
    }

    pub fn current_state(&self) -> K {
        self.current_animation
    }

    pub fn register_animation(&mut self, key: K, animation: CombinedAnimations) {
        self.animation_map.insert(key, animation);
    }
    
    pub fn register_animation_bulk(&mut self, animations: Vec<(K, AnimationRepositoryKey)>) {
        for (key, animation_key) in animations {
            self.register_animation(key, get_animation(animation_key));
        }
    }

    /// Sets the current animation to be played, but does not override a currently playing one-shot animation, except if the one-shot is finished.
    pub fn set_animation(&mut self, key: K) {
        if !self.current_animation().is_oneshot() || self.current_animation().is_finished() {
            self.current_animation = key;
            self.current_animation_mut().reset();
        }
    }

    /// Forces an animation to be played. Ignores any currently playing one-shot animations. Useful to start a new one-shot animation.
    /// Resets any playing animation to the beginning.
    pub fn set_animation_override(&mut self, key: K) {
        self.current_animation = key;
        self.current_animation_mut().reset();
        self.time_started = Instant::now();
    }

    pub fn render_to_hbd(
        &mut self,
        x: i64,
        y: i64,
        hbd: &mut HalfBlockDisplayRender,
        current_time: Instant,
    ) -> Option<KeyedAnimationResult<K>> {
        let current_animation = self.animation_map.get_mut(&self.current_animation).unwrap();
        let time_passed = current_time.duration_since(self.time_started).as_secs_f32();
        current_animation
            .render_to_hbd(x, y, hbd, time_passed)
            .map(|anim_res| match anim_res {
                AnimationResult::Done => KeyedAnimationResult::Finished(self.current_animation),
                AnimationResult::Trigger => KeyedAnimationResult::Triggered(self.current_animation),
            })
    }
}
