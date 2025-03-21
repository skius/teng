use std::collections::HashMap;
use std::hash::Hash;
use std::time::Instant;
use crate::gpu::animation::{Animation, AnimationResult};
use crate::gpu::instancewriter::InstanceWriter;
use crate::gpu::sprite::TextureAnimationAtlas;

pub trait AnimationStateMachine {
    /// Data returned by a trigger.
    type TriggerData: Clone;

    type State: Hash + Eq + Clone + Copy;

    fn get_animation(&self, state: &Self::State) -> Animation<Self::TriggerData>;

    fn next_state(&self, current_state: &Self::State, result: &AnimationResult<Self::TriggerData>) -> Self::State;
    
    fn get_atlas(&self) -> &TextureAnimationAtlas;
}

struct AnimationMap<ASM: AnimationStateMachine> {
    animations: HashMap<ASM::State, Animation<ASM::TriggerData>>,
    current_state: ASM::State,
}

impl<ASM: AnimationStateMachine> AnimationMap<ASM> {
    fn get(&self, state: &ASM::State) -> &Animation<ASM::TriggerData> {
        self.animations.get(state).unwrap()
    }

    fn get_mut(&mut self, state: &ASM::State) -> &mut Animation<ASM::TriggerData> {
        self.animations.get_mut(state).unwrap()
    }

    fn current_animation(&self) -> &Animation<ASM::TriggerData> {
        self.get(&self.current_state)
    }

    fn current_animation_mut(&mut self) -> &mut Animation<ASM::TriggerData> {
        self.animations.get_mut(&self.current_state).unwrap()
    }

}

/// Every entity that runs animations has this.
pub struct AnimationController<ASM: AnimationStateMachine> {
    animation_map: AnimationMap<ASM>,
    time_started: Instant,
    asm: ASM,
}

impl<ASM: AnimationStateMachine> AnimationController<ASM> {
    pub fn new(asm: ASM, initial_state: ASM::State) -> Self {
        let initial_animation = asm.get_animation(&initial_state);
        Self {
            animation_map: AnimationMap {
                animations: HashMap::from([(initial_state, initial_animation)]),
                current_state: initial_state,
            },
            time_started: Instant::now(),
            asm,
        }
    }

    pub fn current_animation(&self) -> &Animation<ASM::TriggerData> {
        self.animation_map.current_animation()
    }

    pub fn current_animation_mut(&mut self) -> &mut Animation<ASM::TriggerData> {
        self.animation_map.current_animation_mut()
    }

    pub fn current_state(&self) -> &ASM::State {
        &self.animation_map.current_state
    }

    // pub fn register_animation(&mut self, state: ASM::State, animation: Animation<ASM::TriggerData>) {
    //     self.animation_map.animations.insert(state, animation);
    // }
    
    pub fn set_animation(&mut self, state: ASM::State) {
        self.switch_to(state);
    }

    pub fn update(&mut self) -> AnimationResult<ASM::TriggerData> {
        let current_animation = self.animation_map.current_animation_mut();
        let result = current_animation.update(self.time_started.elapsed().as_secs_f32());
        if result.is_done() {
            let next_state = self.asm.next_state(&self.animation_map.current_state, &result);
            self.switch_to(next_state);
        }
        result
    }
    
    pub fn render(&self, pos: glam::Vec2, layer: u32, instance_writer: &mut InstanceWriter) {
        let current_animation = self.current_animation();
        let atlas = self.asm.get_atlas();
        current_animation.render(atlas, pos, layer, instance_writer);
    }
    
    
    fn switch_to(&mut self, state: ASM::State) {
        self.animation_map.current_state = state;
        self.animation_map.animations.insert(state, self.asm.get_animation(&state));
        self.time_started = Instant::now();
    }
}