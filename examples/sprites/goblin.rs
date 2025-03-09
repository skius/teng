use crate::animationcontroller::AnimationController;
use crate::sprite::{get_animation, AnimationRepositoryKey};

#[derive(Default, Debug, Clone, Copy, Eq, PartialEq, Hash)]
pub enum GoblinState {
    #[default]
    Idle,
    Walk,
    Run,
    Hurt,
    Death,
}

#[derive(Debug)]
pub struct Goblin {
    animation_controller: AnimationController<GoblinState>,
    pos: (f64, f64),
    health: f64,
}

impl Goblin {
    pub fn new() -> Self {
        let mut animation_controller = AnimationController::default();
        animation_controller.register_animations_from_repository(vec![
            (GoblinState::Idle, AnimationRepositoryKey::GoblinIdle),
            (GoblinState::Walk, AnimationRepositoryKey::GoblinWalk),
            (GoblinState::Run, AnimationRepositoryKey::GoblinRun),
            (GoblinState::Hurt, AnimationRepositoryKey::GoblinHurt),
            (GoblinState::Death, AnimationRepositoryKey::GoblinDeath),
        ]);
        
        Self {
            animation_controller,
            pos: (0.0, 0.0),
            health: 100.0,
        }
    }

    pub fn update(&mut self) {
        if self.health <= 0.0 {
            self.animation_controller.set_animation(GoblinState::Death);
        } else if self.health < 50.0 {
            self.animation_controller.set_animation(GoblinState::Hurt);
        } else if self.health < 75.0 {
            self.animation_controller.set_animation(GoblinState::Run);
        } else {
            self.animation_controller.set_animation(GoblinState::Walk);
        }
    }

    pub fn get_pos(&self) -> (f64, f64) {
        self.pos
    }
    
    pub fn get_animation_controller(&mut self) -> &mut AnimationController<GoblinState> {
        &mut self.animation_controller
    }
    
}
