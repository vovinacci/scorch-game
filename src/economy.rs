//! Round economy, with catch-up rules so an early loss doesn't decide the game:
//!   - everyone gets a flat round pay,
//!   - every lost round in a row pays a growing loss bonus,
//!   - killing a tank pays a bounty for each round that tank has won,
//!   - last place shoots first next round.
//!
//! With the Interest setting on, money kept unspent earns a little more.

use serde::{Deserialize, Serialize};

use crate::choice::choice;
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
    /// On the money left unspent, with the Interest setting on (added last, with the round's other pay).
    #[serde(default)]
    pub interest: i32,
    pub damage: i32,
    pub kills: i32,
    pub bounty: i32,
    pub win: i32,
    pub round_pay: i32,
    pub loss_bonus: i32,
}

impl Income {
    pub fn lines(&self) -> [(&'static str, i32); 7] {
        [
            ("income-damage", self.damage),
            ("income-kills", self.kills),
            ("income-bounties", self.bounty),
            ("income-round-win", self.win),
            ("income-round-pay", self.round_pay),
            ("income-loss-bonus", self.loss_bonus),
            ("income-interest", self.interest),
        ]
    }

    pub fn total(&self) -> i32 {
        self.lines().iter().map(|(_, v)| v).sum()
    }
}

pub fn loss_bonus(streak: u32) -> i32 {
    LOSS_BONUS * streak.min(LOSS_BONUS_MAX_STREAK).cast_signed()
}

choice!(Interest { Off, Low, High });

#[expect(
    clippy::derivable_impls,
    reason = "choice! declares the enum, so it can't take #[derive(Default)]"
)]
impl Default for Interest {
    fn default() -> Self {
        Interest::Off
    }
}

impl Interest {
    /// Share of the money left unspent paid on top at the end of a round.
    pub fn rate(self) -> f32 {
        match self {
            Interest::Off => 0.0,
            Interest::Low => 0.05,
            Interest::High => 0.1,
        }
    }

    /// Interest on `money`, in whole dollars, and never more than `INTEREST_MAX`.
    pub fn on(self, money: i32) -> i32 {
        ((money.max(0) as f32 * self.rate()) as i32).min(INTEREST_MAX)
    }
}

choice!(Prices { Fixed, Market });

#[expect(
    clippy::derivable_impls,
    reason = "choice! declares the enum, so it can't take #[derive(Default)]"
)]
impl Default for Prices {
    fn default() -> Self {
        Prices::Fixed
    }
}

/// With Market prices, each weapon's and item's price follows what players buy: every pack bought in a round's
/// shopping raises it by `MARKET_UP`, a round when nobody buys any lowers it by `MARKET_DOWN`, between `MARKET_LOW`
/// and `MARKET_HIGH` times the list price. Selling a pack counts as one bought less. Saved with the game.
#[derive(Clone, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Market {
    on: bool,
    /// Per ware (`slot`): the share of the list price it costs now.
    factors: Vec<f32>,
    /// Per ware: packs bought this round, less packs sold.
    demand: Vec<i32>,
}

pub const MARKET_UP: f32 = 0.1;
pub const MARKET_DOWN: f32 = 0.05;
pub const MARKET_LOW: f32 = 0.6;
pub const MARKET_HIGH: f32 = 1.6;

impl Market {
    pub fn new(prices: Prices) -> Self {
        Self {
            on: prices == Prices::Market,
            ..Self::default()
        }
    }

    /// What a ware with this `list` price costs now: the list price, or on the market the nearest $5 to its share
    /// of it, $5 at least.
    pub fn price(&self, slot: usize, list: i32) -> i32 {
        let factor = self.factors.get(slot).copied().unwrap_or(1.0);
        if !self.on {
            return list;
        }
        ((list as f32 * factor / 5.0).round() as i32 * 5).max(5)
    }

    /// A pack of the ware in `slot` was bought (`packs` 1) or sold (-1).
    pub fn trade(&mut self, slot: usize, packs: i32) {
        if !self.on {
            return;
        }
        if self.demand.len() <= slot {
            self.demand.resize(slot + 1, 0);
        }
        self.demand[slot] += packs;
    }

    /// The round's shopping is over: prices move with what was bought, for `slots` wares.
    pub fn settle(&mut self, slots: usize) {
        if !self.on {
            return;
        }
        self.factors.resize(slots.max(self.factors.len()), 1.0);
        self.demand.resize(self.factors.len(), 0);
        for (factor, demand) in self.factors.iter_mut().zip(&mut self.demand) {
            let change = if *demand > 0 {
                MARKET_UP * *demand as f32
            } else {
                -MARKET_DOWN
            };
            *factor = (*factor + change).clamp(MARKET_LOW, MARKET_HIGH);
            *demand = 0;
        }
    }
}

/// The most interest pays in a round: saving is a plan, not a way to stop playing.
pub const INTEREST_MAX: i32 = 100;

/// Pays everyone at the end of a round. Damage, kills and bounties were already
/// recorded during the battle; `interest` is paid on the money they come back with.
pub fn end_round(players: &mut [Player], winner: Option<usize>, interest: Interest) {
    for (i, p) in players.iter_mut().enumerate() {
        p.income.interest = interest.on(p.money);
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
        p.earned += p.income.total();
    }
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
            end_round(&mut ps, Some(0), Interest::Off);
            let expected = LOSS_BONUS * round.min(LOSS_BONUS_MAX_STREAK).cast_signed();
            assert_eq!(ps[1].income.loss_bonus, expected);
            ps[1].income = Income::default();
        }
        assert_eq!(ps[0].wins, 6);
        assert_eq!(ps[0].loss_streak, 0);
        end_round(&mut ps, Some(1), Interest::Off);
        assert_eq!(ps[1].loss_streak, 0);
        assert_eq!(ps[1].income.win, WIN_PAY);
    }

    #[test]
    fn interest_is_paid_on_the_money_kept_up_to_a_limit() {
        let mut ps = players(3);
        ps[0].money = 300;
        ps[1].money = 5000;
        ps[2].money = -20;
        end_round(&mut ps, None, Interest::High);
        assert_eq!(
            [
                ps[0].income.interest,
                ps[1].income.interest,
                ps[2].income.interest
            ],
            [30, INTEREST_MAX, 0]
        );
        assert_eq!(ps[0].money, 300 + 30 + ROUND_PAY + LOSS_BONUS);
        let mut ps = players(1);
        ps[0].money = 300;
        end_round(&mut ps, None, Interest::Low);
        assert_eq!(ps[0].income.interest, 15);
        end_round(&mut ps, None, Interest::Off);
        assert_eq!(ps[0].income.interest, 0);
        // Everything paid counts as earned, whatever is spent later.
        let earned = ps[0].earned;
        ps[0].money = 0;
        assert_eq!(ps[0].earned, earned);
        assert!(earned >= 3 * ROUND_PAY);
    }

    #[test]
    fn market_prices_follow_what_is_bought() {
        let mut fixed = Market::new(Prices::Fixed);
        fixed.trade(0, 5);
        fixed.settle(3);
        assert_eq!(fixed.price(0, 100), 100, "fixed prices never move");
        let mut market = Market::new(Prices::Market);
        assert_eq!(market.price(0, 60), 60, "list prices to begin with");
        // Three packs of the first bought, one of the second bought and sold back, none of the third.
        market.trade(0, 3);
        market.trade(1, 1);
        market.trade(1, -1);
        market.settle(3);
        assert_eq!(market.price(0, 60), 80, "30% dearer, to the nearest $5");
        assert_eq!(market.price(1, 60), 55, "nobody kept any: cheaper");
        assert_eq!(market.price(2, 60), 55);
        // It stays within its limits however long the run.
        for _ in 0..40 {
            market.trade(0, 3);
            market.settle(3);
        }
        assert_eq!(market.price(0, 100), 160);
        assert_eq!(market.price(2, 100), 60);
        assert_eq!(market.price(2, 5), 5, "never free");
    }
}
