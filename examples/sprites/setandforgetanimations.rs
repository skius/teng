use crate::sprite::{AnimationKind, AnimationResult, CombinedAnimations};
use std::time::Instant;
use teng::rendering::render::HalfBlockDisplayRender;

pub struct SetAndForgetAnimation {
    position: (i64, i64),
    start_time: Instant,
    animation: CombinedAnimations,
}

#[derive(Default)]
pub struct SetAndForgetAnimations {
    animations: Vec<SetAndForgetAnimation>,
}

impl SetAndForgetAnimations {
    pub fn add(&mut self, position: (i64, i64), mut animation: CombinedAnimations) {
        animation.set_kind(AnimationKind::OneShot {
            trigger_frame: None,
        });
        self.animations.push(SetAndForgetAnimation {
            position,
            start_time: Instant::now(),
            animation,
        });
    }

    pub fn render_to_hbd(&mut self, hbd: &mut HalfBlockDisplayRender, current_time: Instant) {
        self.animations.retain_mut(|animation| {
            let time_passed = current_time
                .duration_since(animation.start_time)
                .as_secs_f32();
            if let Some(AnimationResult::Done) = animation.animation.render_to_hbd(
                animation.position.0,
                animation.position.1,
                hbd,
                time_passed,
            ) {
                // Remove the animation if it's done
                false
            } else {
                true
            }
        });
    }
}
