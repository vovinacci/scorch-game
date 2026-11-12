//! All-time statistics for human players, kept in the profile file.

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::i18n::t;
use crate::ladder::Ladder;
use crate::ui;

/// What one player did in the current round. Added to `Stats` when the round ends.
#[derive(Default, Clone, Copy, Serialize, Deserialize)]
pub struct RoundStats {
    pub shots: u32,
    pub damage: f32,
    pub best_hit: f32,
    pub kills: u32,
    pub died: bool,
    pub fled: bool,
}

#[derive(Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Stats {
    pub games: u32,
    pub games_won: u32,
    pub games_abandoned: u32,
    pub best_game_wins: u32,
    pub rounds: u32,
    pub rounds_won: u32,
    pub kills: u32,
    pub deaths: u32,
    pub flees: u32,
    pub shots: u32,
    pub damage: f64,
    pub best_hit: f32,
    pub money_earned: i64,
}

impl Stats {
    pub fn add_round(&mut self, round: &RoundStats, won: bool, earned: i32) {
        self.rounds += 1;
        self.rounds_won += u32::from(won);
        self.kills += round.kills;
        self.deaths += u32::from(round.died);
        self.flees += u32::from(round.fled);
        self.shots += round.shots;
        self.damage += f64::from(round.damage);
        self.best_hit = self.best_hit.max(round.best_hit);
        self.money_earned += i64::from(earned);
    }

    pub fn add_game(&mut self, won: bool, round_wins: u32) {
        self.games += 1;
        self.games_won += u32::from(won);
        self.best_game_wins = self.best_game_wins.max(round_wins);
    }

    /// (translation key, value) per line of the stats screen.
    pub fn rows(&self) -> Vec<(&'static str, String)> {
        let percent = |part: u32, whole: u32| {
            if whole == 0 {
                "-".to_string()
            } else {
                format!("{:.0}%", 100.0 * part as f32 / whole as f32)
            }
        };
        let per_shot = if self.shots == 0 {
            "-".to_string()
        } else {
            format!("{:.1}", self.damage / f64::from(self.shots))
        };
        vec![
            ("stats-games-played", self.games.to_string()),
            (
                "stats-games-won",
                format!(
                    "{} ({})",
                    self.games_won,
                    percent(self.games_won, self.games)
                ),
            ),
            ("stats-games-abandoned", self.games_abandoned.to_string()),
            ("stats-best-game", self.best_game_wins.to_string()),
            ("stats-rounds-played", self.rounds.to_string()),
            (
                "stats-rounds-won",
                format!(
                    "{} ({})",
                    self.rounds_won,
                    percent(self.rounds_won, self.rounds)
                ),
            ),
            ("stats-kills", self.kills.to_string()),
            ("stats-deaths", self.deaths.to_string()),
            ("stats-rounds-fled", self.flees.to_string()),
            ("stats-shots", self.shots.to_string()),
            ("stats-damage", format!("{:.0}", self.damage)),
            ("stats-damage-per-shot", per_shot),
            ("stats-best-hit", format!("{:.0}", self.best_hit)),
            ("stats-money", format!("${}", self.money_earned)),
        ]
    }
}

/// Draws the all-time stats screen. Returns `true` when the player goes back.
pub fn screen(stats: &Stats, ladder: &Ladder) -> bool {
    let area = Rect::new(340.0, 30.0, 600.0, 660.0);
    ui::panel(area);
    ui::text_centered(
        &t!("stats-title"),
        area.center().x,
        area.y + 55.0,
        42.0,
        GOLD,
    );
    let mut y = area.y + 105.0;
    let mut rows = stats.rows();
    rows.push((
        "stats-ladder",
        t!(
            "stats-ladder-rung",
            rung = ladder.best(),
            of = Ladder::RUNGS
        ),
    ));
    if ladder.champion > 0 {
        rows.push(("stats-ladder-champion", ladder.champion.to_string()));
    }
    for (key, value) in rows {
        ui::text_fit(&t!(key), area.x + 50.0, y, area.w - 230.0, 24.0, LIGHTGRAY);
        let w = ui::measure(&value, 24.0).width;
        ui::text_at(&value, area.x + area.w - 50.0 - w, y, 24.0, WHITE);
        y += 30.0;
    }
    ui::button(
        Rect::new(area.center().x - 120.0, area.y + area.h - 76.0, 240.0, 52.0),
        &t!("back"),
        true,
    ) || is_key_pressed(KeyCode::Escape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rounds_and_games_add_up() {
        let mut stats = Stats::default();
        let round = RoundStats {
            shots: 3,
            damage: 80.0,
            best_hit: 50.0,
            kills: 1,
            died: true,
            fled: false,
        };
        stats.add_round(&round, false, 120);
        stats.add_round(
            &RoundStats {
                best_hit: 30.0,
                ..round
            },
            true,
            200,
        );
        stats.add_game(true, 1);
        assert_eq!(
            (stats.rounds, stats.rounds_won, stats.kills, stats.deaths),
            (2, 1, 2, 2)
        );
        assert_eq!(stats.shots, 6);
        assert_eq!(stats.best_hit, 50.0);
        assert_eq!(stats.money_earned, 320);
        assert_eq!(
            (stats.games, stats.games_won, stats.best_game_wins),
            (1, 1, 1)
        );
    }
}
