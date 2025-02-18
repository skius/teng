//! Responsible for deriving seeds for the game from a passed seed.

use std::hash::{DefaultHasher, Hash, Hasher};
use std::sync::OnceLock;

/// The global seed to use for the game.
/// Can be initialized by the user or a default value.
static SEED: OnceLock<u64> = OnceLock::new();

/// Set the global seed. You can only set the seed once.
pub fn set_seed(seed: u64) {
    SEED.set(seed).unwrap();
}

/// Get the global seed.
pub fn get_seed() -> u64 {
    *SEED.get().unwrap()
}

/// Get the global seed, if it has been set.
pub fn get_seed_opt() -> Option<u64> {
    SEED.get().copied()
}

macro_rules! seed_impl {
    ($fn_name:ident, $typ:ty) => {
        /// Derive a deterministic seed for a given purpose from the global seed.
        /// Per global seed and purpose, the returned seed will always be the same.
        #[allow(unused)]
        pub fn $fn_name(purpose: &str) -> $typ {
            let mut hasher = DefaultHasher::new();
            SEED.get().unwrap().hash(&mut hasher);
            purpose.hash(&mut hasher);
            hasher.finish() as $typ
        }
    };
}

seed_impl!(get_u64_seed_for, u64);
seed_impl!(get_usize_seed_for, usize);
seed_impl!(get_i64_seed_for, i64);
seed_impl!(get_i32_seed_for, i32);
seed_impl!(get_i16_seed_for, i16);
seed_impl!(get_i8_seed_for, i8);
seed_impl!(get_u32_seed_for, u32);
seed_impl!(get_u16_seed_for, u16);
seed_impl!(get_u8_seed_for, u8);
