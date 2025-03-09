/// This type can be used to implement consumable triggers.
#[derive(Debug, Default, Eq, PartialEq)]
pub enum Trigger {
    #[default]
    Empty,
    Ready,
}

impl Trigger {
    /// Consume the trigger, returning `true` if it was `Ready`.
    pub fn consume(&mut self) -> bool {
        match self {
            Trigger::Empty => false,
            Trigger::Ready => {
                *self = Trigger::Empty;
                true
            }
        }
    }
    
    /// Set the trigger to `Ready`.
    pub fn set(&mut self) {
        *self = Trigger::Ready;
    }
}