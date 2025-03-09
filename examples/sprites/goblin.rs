use teng::components::Component;
use teng::{SharedState, UpdateInfo};
use crate::animationcontroller::AnimationController;
use crate::GameState;
use crate::sprite::{AnimationRepositoryKey, get_animation};

// TODO: Hurtboxes,
// TODO: Goblins

pub struct GoblinComponent;

impl Component<GameState> for GoblinComponent {
    fn update(&mut self, update_info: UpdateInfo, shared_state: &mut SharedState<GameState>) {
        for goblin in &mut shared_state.custom.goblins {
            
        }
    }
}

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
    pub fn new_at(pos: (f64, f64)) -> Self {
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
            pos,
            health: 100.0,
        }
    }

    pub fn get_pos(&self) -> (f64, f64) {
        self.pos
    }

    pub fn get_animation_controller(&mut self) -> &mut AnimationController<GoblinState> {
        &mut self.animation_controller
    }
}
