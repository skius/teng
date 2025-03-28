use std::any::Any;
use crate::gpu::instancewriter::InstanceWriter;
use crate::gpu::sprite::{AnimationKey, TextureAnimationAtlas};

/// Represents a specific frame in a single iteration of an animation loop.
///
/// It can be in the range of [0, frame_count * repeat_factor).
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
struct AnimationFrame(usize);

impl From<usize> for AnimationFrame {
    fn from(frame: usize) -> Self {
        Self(frame)
    }
}

/// A counter of the number of frames that have been rendered.
///
/// When the animation loops around, this counter keeps increasing.
/// This allows for making sure every trigger is triggered, even at low frame rate.
#[derive(Debug, Default, Clone, Copy, PartialEq, Eq, Ord, PartialOrd)]
struct FinishedFrames(usize);

impl From<usize> for FinishedFrames {
    fn from(frame: usize) -> Self {
        Self(frame)
    }
}

impl FinishedFrames {
    fn from_time_and_frame_duration(time_since_start_secs: f32, frame_duration_secs: f32) -> Self {
        Self((time_since_start_secs / frame_duration_secs) as usize)
    }

    /// Excludes other. `frame_count` is the duration of a single iteration of the animation.
    fn animation_frames_since(&self, other: Self, frame_count: usize, kind: AnimationKind) -> impl Iterator<Item = AnimationFrame> {
        let start = other.0 + 1; // Excluding the other frame
        let end = self.0;
        let mut curr = start;
        std::iter::from_fn(move || {
            if curr > end {
                return None;
            }
            if let AnimationKind::Once = kind {
                if curr >= frame_count {
                    return None;
                }
            }
            let frame = AnimationFrame(curr % frame_count);
            curr += 1;
            Some(frame)
        })
    }
}

// struct TriggerData {
//     data: Box<dyn Fn() -> Box<dyn Any>>,
// }
//
// impl TriggerData {
//     fn new<T: Clone + 'static>(data: T) -> Self {
//         Self {
//             data: Box::new(move || Box::new(data.clone()) as Box<dyn Any>),
//         }
//     }
//
//     fn get(&self) -> Box<dyn Any> {
//         (self.data)()
//     }
// }

enum AnimationState {
    // animation is done and can be discarded
    Done,
    // animation is still running
    Running,
}

impl AnimationState {
    fn with_triggers<TD>(self, triggers: Vec<TD>) -> AnimationResult<TD> {
        AnimationResult {
            state: self,
            triggers,
        }
    }

    fn to_result<TD>(self) -> AnimationResult<TD> {
        AnimationResult {
            state: self,
            triggers: Vec::new(),
        }
    }
}

pub struct AnimationResult<TriggerData> {
    pub state: AnimationState,
    // these triggers were issued
    pub triggers: Vec<TriggerData>,
}

impl<TriggerData> AnimationResult<TriggerData> {
    pub fn is_done(&self) -> bool {
        match self.state {
            AnimationState::Done => true,
            AnimationState::Running => false,
        }
    }
}

struct AnimationTrigger<TriggerData> {
    trigger_frame: AnimationFrame,
    data_on_trigger: TriggerData,
}

#[derive(Clone, Copy, Eq, PartialEq)]
pub enum AnimationKind {
    // play once, then stop
    Once,
    // loop forever
    Loop,
}

pub struct Animation<TriggerData> {
    key: AnimationKey<'static>,
    // ignoring the repeat factor
    single_iter_frame_count: usize,
    repeat_factor: usize,
    // TODO: should this be an option for the case where nothing has been triggered? Currently triggers at frame 0 cannot be triggered, since they're initialized to that.
    last_rendered_frame: FinishedFrames,
    frame_duration_secs: f32,
    has_issued_trigger: bool,
    kind: AnimationKind,
    triggers: Vec<AnimationTrigger<TriggerData>>,
}

impl<TriggerData: Clone> Animation<TriggerData> {
    // TODO: turn TextureAnimationAtlas into an Rc and store, or maybe even a global?
    pub fn new(atlas: &TextureAnimationAtlas, key: AnimationKey<'static>, frame_duration_secs: f32) -> Self {
        Self {
            key,
            single_iter_frame_count: atlas.frame_count(key),
            repeat_factor: 1,
            last_rendered_frame: FinishedFrames(0),
            frame_duration_secs,
            has_issued_trigger: false,
            kind: AnimationKind::Loop,
            triggers: Vec::new(),
        }
    }

    pub fn with_kind(mut self, kind: AnimationKind) -> Self {
        self.kind = kind;
        self
    }

    pub fn with_trigger(mut self, trigger_frame: impl Into<AnimationFrame>, data_on_trigger: TriggerData) -> Self {
        self.triggers.push(AnimationTrigger {
            trigger_frame: trigger_frame.into(),
            data_on_trigger,
        });
        self
    }

    pub fn with_repeat(mut self, repeat_factor: usize) -> Self {
        self.repeat_factor = repeat_factor;
        self
    }

    fn frame_count(&self) -> usize {
        self.single_iter_frame_count * self.repeat_factor
    }

    fn get_triggered_triggers(&self, new_frame: FinishedFrames) -> Vec<TriggerData> {
        let last_frame = self.last_rendered_frame;
        let mut triggered_triggers = Vec::new();

        // TODO: exponential time, but realistically we're running this very rarely.
        for anim_frame in new_frame.animation_frames_since(last_frame, self.frame_count(), self.kind) {
            for trigger in &self.triggers {
                if anim_frame == trigger.trigger_frame {
                    triggered_triggers.push(trigger.data_on_trigger.clone());
                }
            }
        }


        triggered_triggers
    }

    fn is_once(&self) -> bool {
        match self.kind {
            AnimationKind::Once => true,
            AnimationKind::Loop => false,
        }
    }

    pub fn update(&mut self, time_since_start_secs: f32) -> AnimationResult<TriggerData> {
        let frame = FinishedFrames::from_time_and_frame_duration(time_since_start_secs, self.frame_duration_secs);
        if frame <= self.last_rendered_frame {
            // Nothing's changed
            return AnimationState::Running.to_result();
        }
        let triggered_triggers = self.get_triggered_triggers(frame);

        self.last_rendered_frame = frame;

        if frame.0 >= self.frame_count() && self.is_once() {
            return AnimationState::Done.with_triggers(triggered_triggers);
        }

        AnimationState::Running.with_triggers(triggered_triggers)
    }

    pub fn render(&self, atlas: &TextureAnimationAtlas, pos: glam::Vec2, layer: u32, instance_writer: &mut InstanceWriter) {
        let frame = self.last_rendered_frame;
        let frame = frame.0 % self.single_iter_frame_count;
        for (idx, sprite) in atlas.get_sprites_frame(self.key, frame).enumerate() {
            let z = (layer+1) as f32 - (idx+1) as f32 * 0.01;

            instance_writer.write(sprite.to_instance([pos.x, pos.y, z]));
        }
    }
}