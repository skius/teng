use std::any::Any;
use crate::gpu::instancewriter::InstanceWriter;
use crate::gpu::sprite::{AnimationKey, TextureAnimationAtlas};

struct TriggerData {
    data: Box<dyn Fn() -> Box<dyn Any>>,
}

impl TriggerData {
    fn new<T: Clone + 'static>(data: T) -> Self {
        Self {
            data: Box::new(move || Box::new(data.clone()) as Box<dyn Any>),
        }
    }

    fn get(&self) -> Box<dyn Any> {
        (self.data)()
    }
}

enum AnimationResult {
    // animation is done and can be discarded
    Done,
    // animation is still running
    Running,
    // trigger was issued
    Trigger(Box<dyn Any>),
}

struct AnimationTrigger {
    trigger_frame: usize,
    did_trigger_this_loop: bool,
    data_on_trigger: TriggerData,
}

enum AnimationKind {
    // play once, then stop
    Once,
    // loop forever
    Loop,
}

pub struct Animation {
    key: AnimationKey<'static>,
    // ignoring the repeat factor
    frame_count: usize,
    repeat_factor: usize,
    last_rendered_frame: usize,
    frame_duration_secs: f32,
    has_issued_trigger: bool,
    kind: AnimationKind,
    triggers: Vec<AnimationTrigger>,
}

impl Animation {
    pub fn new(atlas: &TextureAnimationAtlas, key: AnimationKey<'static>, frame_duration_secs: f32) -> Self {
        Self {
            key,
            frame_count: atlas.frame_count(key),
            repeat_factor: 1,
            last_rendered_frame: 0,
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

    pub fn with_trigger(mut self, trigger_frame: usize, data_on_trigger: TriggerData) -> Self {
        self.triggers.push(AnimationTrigger {
            trigger_frame,
            did_trigger_this_loop: false,
            data_on_trigger,
        });
        self
    }

    pub fn with_repeat(mut self, repeat_factor: usize) -> Self {
        self.repeat_factor = repeat_factor;
        self
    }

    pub fn render(&mut self, atlas: &TextureAnimationAtlas, pos: glam::Vec2, time_since_start_secs: f32, instance_writer: &mut InstanceWriter) -> AnimationResult {

        // TODO: fill this

        AnimationResult::Running
    }
}