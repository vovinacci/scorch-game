//! The ladder: ten rungs of opponents, from one Moron to a Cyborg, a Sniper and a Spoiler. A single player climbs by winning games
//! outright, drops a rung after two losses in a row on it, and is champion after beating the top rung.

use serde::{Deserialize, Serialize};

use crate::ai::{AiProfile, PRESETS};

/// The bots to beat on each rung (indices into `ai::PRESETS`). The order comes from `balance_ladder`, which plays
/// whole games against the line-ups a rung could be made of: the share of games won outright falls rung by rung,
/// from 100% to 15% for a Sniper-like player and from 95% to about 6% for a Shooter-like one. Every bot type is met on
/// the way. More bots is not always harder, since they also fight each other.
const RUNGS: [&[usize]; 10] = [
    &[MORON],
    &[MORON, MORON],
    &[TOSSER],
    &[SHOOTER],
    &[CHOOSER],
    &[SHOOTER, MORON, MORON],
    &[POOLSHARK, SHOOTER, MORON],
    &[SPOILER, SHOOTER, MORON],
    &[SNIPER, SPOILER, POOLSHARK],
    &[CYBORG, SNIPER, SPOILER],
];
const MORON: usize = 0;
const SHOOTER: usize = 1;
const SNIPER: usize = 2;
const SPOILER: usize = 3;
const CYBORG: usize = 4;
const TOSSER: usize = 5;
const POOLSHARK: usize = 6;
const CHOOSER: usize = 7;

/// Losses in a row on a rung that take the player down one.
const LOSSES_TO_DROP: u8 = 2;

/// How a ladder game ended for the player.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Outcome {
    /// Most round wins, alone.
    Won,
    /// Most round wins, shared with a bot.
    Shared,
    /// Anything else, and abandoning the game.
    Lost,
}

impl Outcome {
    /// From the round wins of the player and of the best bot.
    pub fn of(player: u32, best_bot: u32) -> Self {
        match player.cmp(&best_bot) {
            std::cmp::Ordering::Greater => Outcome::Won,
            std::cmp::Ordering::Equal => Outcome::Shared,
            std::cmp::Ordering::Less => Outcome::Lost,
        }
    }
}

/// What a game did to the player's place on the ladder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Move {
    Up,
    /// Won on the top rung.
    Champion,
    /// A shared first place: nothing changes.
    Stay,
    /// Lost, with `left` more losses in a row before going down (0 on the bottom rung: there is no further down).
    Lost {
        left: u8,
    },
    Down,
}

/// Where the player is on the ladder. Kept in the profile.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(default)]
pub struct Ladder {
    /// Current rung, counted from 0.
    rung: usize,
    /// Losses in a row on this rung.
    losses: u8,
    /// Highest rung reached, counted from 0.
    best: usize,
    /// Games won on the top rung.
    pub champion: u32,
}

impl Ladder {
    pub const RUNGS: usize = RUNGS.len();

    /// The current rung as players count it, from 1.
    pub fn rung(&self) -> usize {
        self.at() + 1
    }

    /// The highest rung reached, from 1.
    pub fn best(&self) -> usize {
        self.best.min(Self::RUNGS - 1) + 1
    }

    /// Losses in a row left on this rung before going down (0 on the bottom rung).
    pub fn losses_left(&self) -> u8 {
        if self.at() == 0 {
            0
        } else {
            LOSSES_TO_DROP.saturating_sub(self.losses)
        }
    }

    /// The bots to beat on the current rung, strongest first.
    pub fn opponents(&self) -> Vec<AiProfile> {
        RUNGS[self.at()].iter().map(|&p| PRESETS[p]).collect()
    }

    /// Takes the result of a ladder game and says what it did.
    pub fn record(&mut self, outcome: Outcome) -> Move {
        let rung = self.at();
        match outcome {
            Outcome::Won if rung + 1 == Self::RUNGS => {
                self.losses = 0;
                self.champion += 1;
                Move::Champion
            }
            Outcome::Won => {
                self.rung = rung + 1;
                self.losses = 0;
                self.best = self.best.max(self.rung);
                Move::Up
            }
            Outcome::Shared => Move::Stay,
            Outcome::Lost if rung == 0 => Move::Lost { left: 0 },
            Outcome::Lost if self.losses + 1 >= LOSSES_TO_DROP => {
                self.rung = rung - 1;
                self.losses = 0;
                Move::Down
            }
            Outcome::Lost => {
                self.losses += 1;
                Move::Lost {
                    left: LOSSES_TO_DROP - self.losses,
                }
            }
        }
    }

    /// The current rung from 0, valid whatever a hand-edited profile says.
    fn at(&self) -> usize {
        self.rung.min(Self::RUNGS - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_most_round_wins_alone_is_a_win() {
        assert_eq!(Outcome::of(3, 2), Outcome::Won);
        assert_eq!(Outcome::of(2, 2), Outcome::Shared);
        assert_eq!(Outcome::of(1, 3), Outcome::Lost);
        assert_eq!(Outcome::of(0, 0), Outcome::Shared);
    }

    #[test]
    fn a_win_goes_up_and_two_losses_in_a_row_go_down() {
        let mut ladder = Ladder::default();
        assert_eq!((ladder.rung(), ladder.best()), (1, 1));
        assert_eq!(ladder.record(Outcome::Won), Move::Up);
        assert_eq!(ladder.record(Outcome::Won), Move::Up);
        assert_eq!((ladder.rung(), ladder.best()), (3, 3));
        assert_eq!(ladder.record(Outcome::Lost), Move::Lost { left: 1 });
        assert_eq!(ladder.losses_left(), 1);
        assert_eq!(ladder.record(Outcome::Lost), Move::Down);
        assert_eq!(
            (ladder.rung(), ladder.best()),
            (2, 3),
            "the best rung stays"
        );
        assert_eq!(ladder.losses_left(), 2, "a fresh start on the lower rung");
    }

    #[test]
    fn a_win_or_a_shared_first_place_between_losses() {
        let mut ladder = Ladder::default();
        ladder.record(Outcome::Won);
        ladder.record(Outcome::Lost);
        // A shared first place changes nothing: the loss still stands.
        assert_eq!(ladder.record(Outcome::Shared), Move::Stay);
        assert_eq!(ladder.losses_left(), 1);
        // A win wipes it out.
        ladder.record(Outcome::Won);
        assert_eq!(ladder.record(Outcome::Lost), Move::Lost { left: 1 });
    }

    #[test]
    fn the_bottom_rung_has_no_way_down_and_the_top_makes_a_champion() {
        let mut ladder = Ladder::default();
        for _ in 0..3 {
            assert_eq!(ladder.record(Outcome::Lost), Move::Lost { left: 0 });
        }
        assert_eq!(ladder.rung(), 1);
        for _ in 1..Ladder::RUNGS {
            assert_eq!(ladder.record(Outcome::Won), Move::Up);
        }
        assert_eq!(ladder.rung(), Ladder::RUNGS);
        assert_eq!(ladder.record(Outcome::Won), Move::Champion);
        assert_eq!((ladder.rung(), ladder.champion), (Ladder::RUNGS, 1));
    }

    #[test]
    fn rungs_go_from_one_moron_to_a_cyborg_with_company() {
        let names = |ladder: &Ladder| -> Vec<&str> {
            ladder.opponents().iter().map(|bot| bot.name).collect()
        };
        let mut ladder = Ladder::default();
        assert_eq!(names(&ladder), ["Moron"]);
        for _ in 1..Ladder::RUNGS {
            ladder.record(Outcome::Won);
        }
        assert_eq!(names(&ladder), ["Cyborg", "Sniper", "Spoiler"]);
        // The names behind the indices are the ones meant, and every type has a rung.
        let met: std::collections::BTreeSet<&str> = RUNGS
            .iter()
            .flat_map(|rung| rung.iter().map(|&p| PRESETS[p].name))
            .collect();
        assert_eq!(met.len(), PRESETS.len());
        assert_eq!(PRESETS[CHOOSER].name, "Chooser");
        assert_eq!(PRESETS[POOLSHARK].name, "Poolshark");
        assert_eq!(PRESETS[TOSSER].name, "Tosser");
        // A profile edited by hand can't point past the top.
        ladder.rung = 99;
        assert_eq!(ladder.rung(), Ladder::RUNGS);
        assert_eq!(names(&ladder).len(), 3);
    }
}
