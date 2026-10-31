//! Round economy, with catch-up rules so an early loss doesn't decide the game:
//!   - everyone gets a flat round pay,
//!   - every lost round in a row pays a growing loss bonus,
//!   - killing a tank pays a bounty for each round that tank has won,
//!   - last place shoots first next round.

use serde::{Deserialize, Serialize};

use crate::player::Player;

/// $ per point of damage dealt to other tanks.
pub const DAMAGE_PAY: f32 = 1.0;
pub const KILL_PAY: i32 = 50;
/// Extra $ for a kill, per round the victim has won.
pub const BOUNTY_PER_WIN: i32 = 50;
pub const WIN_PAY: i32 = 100;
pub const ROUND_PAY: i32 = 75;
/// Loss bonus per lost round in a row, up to `LOSS_BONUS_MAX_STREAK` rounds.
pub const LOSS_BONUS: i32 = 40;
pub const LOSS_BONUS_MAX_STREAK: u32 = 4;

/// Money earned in the current round, kept per source so the shop can show it.
#[derive(Default, Clone, Copy, Serialize, Deserialize)]
pub struct Income {
    pub damage: i32,
    pub kills: i32,
    pub bounty: i32,
    pub win: i32,
    pub round_pay: i32,
    pub loss_bonus: i32,
}

impl Income {
    pub fn lines(&self) -> [(&'static str, i32); 6] {
        [
            ("income-damage", self.damage),
            ("income-kills", self.kills),
            ("income-bounties", self.bounty),
            ("income-round-win", self.win),
            ("income-round-pay", self.round_pay),
            ("income-loss-bonus", self.loss_bonus),
        ]
    }

    pub fn total(&self) -> i32 {
        self.lines().iter().map(|(_, v)| v).sum()
    }
}

pub fn loss_bonus(streak: u32) -> i32 {
    LOSS_BONUS * streak.min(LOSS_BONUS_MAX_STREAK) as i32
}

/// Pays everyone at the end of a round. Damage, kills and bounties were already
/// recorded during the battle.
pub fn end_round(players: &mut [Player], winner: Option<usize>) {
    for (i, p) in players.iter_mut().enumerate() {
        if winner == Some(i) {
            p.wins += 1;
            p.loss_streak = 0;
            p.income.win = WIN_PAY;
        } else {
            p.loss_streak += 1;
            p.income.loss_bonus = loss_bonus(p.loss_streak);
        }
        p.income.round_pay = ROUND_PAY;
        p.money += p.income.total();
    }
}

/// Last place shoots first: fewest wins, then fewest kills.
pub fn underdog(players: &[Player]) -> usize {
    (0..players.len())
        .min_by_key(|&i| (players[i].wins, players[i].kills))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests {
    use super::*;
    use macroquad::prelude::WHITE;

    fn players(n: usize) -> Vec<Player> {
        (0..n)
            .map(|i| Player::new(&format!("P{i}"), WHITE, None))
            .collect()
    }

    #[test]
    fn loser_gets_growing_capped_bonus_and_winner_resets() {
        let mut ps = players(2);
        for round in 1..=6 {
            end_round(&mut ps, Some(0));
            let expected = LOSS_BONUS * round.min(LOSS_BONUS_MAX_STREAK) as i32;
            assert_eq!(ps[1].income.loss_bonus, expected);
            ps[1].income = Income::default();
        }
        assert_eq!(ps[0].wins, 6);
        assert_eq!(ps[0].loss_streak, 0);
        end_round(&mut ps, Some(1));
        assert_eq!(ps[1].loss_streak, 0);
        assert_eq!(ps[1].income.win, WIN_PAY);
    }

    #[test]
    fn last_place_shoots_first() {
        let mut ps = players(3);
        ps[0].wins = 2;
        ps[1].wins = 1;
        ps[2].wins = 1;
        ps[2].kills = 3;
        assert_eq!(underdog(&ps), 1);
    }
}
