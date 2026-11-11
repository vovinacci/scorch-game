//! Random numbers for the game: wind, terrain, bots' errors, bomblets.
//!
//! Every thread has its own generator. The game seeds its one from the clock at start-up, so no two games are alike.
//! A test runs on a thread of its own with the generator in its starting state, so it sees the same numbers every
//! time: a test that fails will fail again, and tests running side by side can't change each other's numbers (as
//! they did with the one generator all threads shared).

use macroquad::rand::{RandGenerator, RandomRange};

thread_local! {
    static GENERATOR: RandGenerator = const { RandGenerator::new() };
}

/// Starts this thread's numbers afresh from `seed`.
pub fn seed(seed: u64) {
    GENERATOR.with(|generator| generator.srand(seed));
}

/// A number from `low` up to, not including, `high`.
pub fn gen_range<T: RandomRange>(low: T, high: T) -> T {
    GENERATOR.with(|generator| generator.gen_range(low, high))
}

/// True with probability `p` (0 never, 1 always).
pub fn chance(p: f32) -> bool {
    gen_range(0.0, 1.0) < p
}

#[cfg(test)]
mod tests {
    use super::*;

    fn numbers() -> Vec<u32> {
        (0..5).map(|_| gen_range(0, 1_000_000)).collect()
    }

    #[test]
    fn a_seed_gives_the_same_numbers_again() {
        seed(7);
        let first = numbers();
        assert_ne!(first, numbers(), "they move on");
        seed(7);
        assert_eq!(first, numbers());
    }

    #[test]
    fn every_thread_starts_the_same_and_keeps_to_itself() {
        let here = numbers();
        // Another thread starts from the beginning, however far this one has gone.
        let there = std::thread::spawn(numbers).join().unwrap();
        assert_eq!(here, there);
    }
}
