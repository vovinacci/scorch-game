//! Bot skill profiles and the custom profile editor.
//!
//! Aim model (the editor screen explains the same thing to the player):
//!   1. The bot simulates thousands of shots and finds the perfect one.
//!   2. First shot at a new target: it adds an error of 40-100% of E, random sign.
//!   3. After every shot: error = error * L + wobble, wobble random within +-WOBBLE * E.
//!
//! So the typical error on shot n is about 0.7 * E * L^(n-1).

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use serde::{Deserialize, Serialize};

use crate::i18n::t;
use crate::ui;

/// Fresh random noise added to every shot, as a fraction of the first-shot error E.
pub const WOBBLE: f32 = 0.12;
/// Roughly what counts as "on target" for the hit estimate on the editor screen.
const HIT_ANGLE: f32 = 2.0;
const HIT_POWER: f32 = 30.0;

#[derive(Clone, Copy, Serialize)]
pub struct AiProfile {
    pub name: &'static str,
    /// E for angle: max angle error (degrees) of the first shot at a new target.
    pub first_angle_error: f32,
    /// E for power: max power error of the first shot at a new target.
    pub first_power_error: f32,
    /// L: fraction of the error kept after each shot. Lower = learns faster.
    pub learning: f32,
}

pub const PRESETS: [AiProfile; 3] = [
    AiProfile {
        name: "Moron",
        first_angle_error: 25.0,
        first_power_error: 300.0,
        learning: 0.8,
    },
    AiProfile {
        name: "Shooter",
        first_angle_error: 12.0,
        first_power_error: 180.0,
        learning: 0.5,
    },
    AiProfile {
        name: "Sniper",
        first_angle_error: 6.0,
        first_power_error: 90.0,
        learning: 0.3,
    },
];

impl AiProfile {
    pub fn custom() -> Self {
        Self {
            name: "Custom",
            ..PRESETS[1]
        }
    }

    /// (angle, power) error of the first shot at a new target.
    pub fn first_error(&self) -> (f32, f32) {
        (rough(self.first_angle_error), rough(self.first_power_error))
    }

    /// Error of the next shot, given the error of the last one.
    pub fn next_error(&self, (angle, power): (f32, f32)) -> (f32, f32) {
        (
            angle * self.learning + wobble(self.first_angle_error),
            power * self.learning + wobble(self.first_power_error),
        )
    }

    /// Typical error on shot `n` (1-based), ignoring wobble.
    /// 0.7 is the average of "40% to 100% of E".
    pub fn typical_error(&self, n: u32) -> (f32, f32) {
        let k = 0.7 * self.learning.powi(n as i32 - 1);
        (self.first_angle_error * k, self.first_power_error * k)
    }

    /// Long-run spread caused by wobble alone (standard deviation).
    /// Uniform noise +-w has std w/sqrt(3); repeated "x * L + noise" settles at
    /// std / sqrt(1 - L^2).
    pub fn wobble_floor(&self) -> (f32, f32) {
        let k = WOBBLE / (3.0 * (1.0 - self.learning * self.learning)).sqrt();
        (self.first_angle_error * k, self.first_power_error * k)
    }

    /// First shot whose typical error is small enough to hit, if any within 20 shots.
    pub fn expected_hit_shot(&self) -> Option<u32> {
        (1..=20).find(|&n| {
            let (a, p) = self.typical_error(n);
            a < HIT_ANGLE && p < HIT_POWER
        })
    }

    pub fn summary(&self) -> String {
        let hit = match self.expected_hit_shot() {
            Some(n) => t!("bot-hits-by", n = n),
            None => t!("bot-rarely-hits"),
        };
        t!(
            "bot-summary",
            angle = self.first_angle_error.round(),
            power = self.first_power_error.round(),
            keep = (self.learning * 100.0).round(),
            hit = hit
        )
    }
}

/// Title screen difficulty choices. The first four build a bot lineup from a
/// threat budget; Custom gives every bot the custom profile.
pub const DIFFICULTIES: [&str; 5] = ["Easy", "Normal", "Hard", "Brutal", "Custom"];
pub const CUSTOM: usize = 4;

/// Difficulty name on screen, in the current language.
pub fn difficulty_text(difficulty: usize) -> String {
    t!(&format!(
        "difficulty-{}",
        DIFFICULTIES[difficulty].to_lowercase()
    ))
}

/// Bot skill (preset) name on screen, in the current language.
pub fn skill_text(name: &str) -> String {
    t!(&format!("skill-{}", name.to_lowercase()))
}

/// Threat budget per difficulty: (points for one bot, points per extra bot).
/// Each bot costs its preset's threat: Moron 1, Shooter 2, Sniper 3.
/// Extra bots add less than the first one, so more opponents is harder
/// but not overwhelming.
const BUDGETS: [(f32, f32); 4] = [(1.0, 0.5), (2.0, 1.0), (3.0, 1.5), (3.0, 3.0)];

/// Threat points to spend on `bots` opponents. Every bot costs at least 1
/// (a Moron) and at most 3 (a Sniper).
pub fn threat_budget(difficulty: usize, bots: usize) -> usize {
    let (first, extra) = BUDGETS[difficulty];
    let points = (first + extra * (bots as f32 - 1.0)).round() as usize;
    points.clamp(bots, 3 * bots)
}

/// Builds the opponents: everyone starts as a Moron, then the remaining budget
/// upgrades random bots one step at a time. Same budget, different mix each game.
pub fn lineup(difficulty: usize, bots: usize, custom: AiProfile) -> Vec<AiProfile> {
    if difficulty == CUSTOM {
        return vec![custom; bots];
    }
    let mut levels = vec![0; bots];
    let mut spare = threat_budget(difficulty, bots) - bots;
    while spare > 0 {
        let i = gen_range(0, bots);
        if levels[i] < PRESETS.len() - 1 {
            levels[i] += 1;
            spare -= 1;
        }
    }
    levels.sort_unstable_by(|a, b| b.cmp(a));
    levels.into_iter().map(|l| PRESETS[l]).collect()
}

/// Ladder step after a finished game: one level up after a win, one down after
/// a loss, within Easy..Brutal. Custom is never changed.
pub fn ladder_step(difficulty: usize, won: bool) -> usize {
    match difficulty {
        CUSTOM => CUSTOM,
        d if won => (d + 1).min(CUSTOM - 1),
        d => d.saturating_sub(1),
    }
}

/// One-line explanation for the title screen.
pub fn describe(difficulty: usize, custom: &AiProfile) -> String {
    if difficulty == CUSTOM {
        return t!("difficulty-custom-note", summary = custom.summary());
    }
    let points: Vec<String> = (1..=3)
        .map(|n| {
            t!(
                "difficulty-points",
                points = threat_budget(difficulty, n),
                bots = n
            )
        })
        .collect();
    t!("difficulty-note", points = points.join(", "))
}

/// Loading form of `AiProfile`: names are `&'static str`, so a saved name is
/// mapped back to a known preset name (anything else becomes "Custom").
#[derive(Deserialize)]
struct SavedProfile {
    name: String,
    first_angle_error: f32,
    first_power_error: f32,
    learning: f32,
}

// Written by hand: a derived impl would require the input to live for 'static.
impl<'de> Deserialize<'de> for AiProfile {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        SavedProfile::deserialize(d).map(Self::from)
    }
}

impl From<SavedProfile> for AiProfile {
    fn from(saved: SavedProfile) -> Self {
        let name = PRESETS
            .iter()
            .map(|p| p.name)
            .find(|&n| n == saved.name)
            .unwrap_or("Custom");
        Self {
            name,
            first_angle_error: saved.first_angle_error,
            first_power_error: saved.first_power_error,
            learning: saved.learning,
        }
    }
}

/// Random value between 40% and 100% of `max`, either sign: first shots never land dead-on.
fn rough(max: f32) -> f32 {
    let e = gen_range(max * 0.4, max);
    if ui::chance(0.5) { e } else { -e }
}

fn wobble(max: f32) -> f32 {
    gen_range(-WOBBLE * max, WOBBLE * max)
}

/// Custom profile editor. Returns `true` when the player is done.
pub fn editor(profile: &mut AiProfile) -> bool {
    ui::text_centered(&t!("custom-title"), ui::WIDTH / 2.0, 60.0, 48.0, ORANGE);

    // Left column: sliders and explanation.
    let x = 70.0;
    let w = 540.0;
    let sliders = [
        (
            t!("custom-angle-error"),
            &mut profile.first_angle_error,
            0.0,
            40.0,
        ),
        (
            t!("custom-power-error"),
            &mut profile.first_power_error,
            0.0,
            400.0,
        ),
        (t!("custom-learning"), &mut profile.learning, 0.05, 0.95),
    ];
    let mut y = 110.0;
    for (label, value, min, max) in sliders {
        ui::text_at(label, x, y, 22.0, LIGHTGRAY);
        let shown = if max <= 1.0 {
            format!("{:.2}", *value)
        } else {
            format!("{:.0}", *value)
        };
        ui::text_at(&shown, x + w - 50.0, y, 22.0, WHITE);
        *value = ui::slider(Rect::new(x, y + 10.0, w, 22.0), *value, min, max);
        y += 70.0;
    }

    ui::text_fit(&t!("custom-start-from"), x, y + 8.0, 110.0, 22.0, LIGHTGRAY);
    let mut bx = x + 120.0;
    for preset in PRESETS {
        if ui::button(
            Rect::new(bx, y - 16.0, 130.0, 36.0),
            &skill_text(preset.name),
            true,
        ) {
            *profile = AiProfile {
                name: "Custom",
                ..preset
            };
        }
        bx += 140.0;
    }
    ui::text_wrapped(&t!("custom-explanation"), x, y + 56.0, w, 18.0, WHITE);

    // Right column: what these numbers mean in practice.
    let rx = 680.0;
    let rw = 530.0;
    ui::panel(Rect::new(rx, 90.0, rw, 520.0));
    ui::text_at(t!("custom-typical-miss"), rx + 20.0, 125.0, 24.0, GOLD);
    let max_bar = 40.0f32.max(profile.first_angle_error * 0.7);
    let mut y = 165.0;
    for n in 1..=8 {
        let (a, p) = profile.typical_error(n);
        let on_target = a < HIT_ANGLE && p < HIT_POWER;
        let color = if on_target { GREEN } else { ORANGE };
        ui::text_at(t!("custom-shot", n = n), rx + 20.0, y, 22.0, LIGHTGRAY);
        draw_rectangle(rx + 140.0, y - 15.0, 180.0 * (a / max_bar), 16.0, color);
        let miss = t!("custom-miss", angle = format!("{a:.1}"), power = p.round());
        ui::text_at(miss, rx + 340.0, y, 22.0, WHITE);
        y += 34.0;
    }
    let (fa, fp) = profile.wobble_floor();
    ui::text_at(
        t!(
            "custom-wobble",
            angle = format!("{fa:.1}"),
            power = fp.round()
        ),
        rx + 20.0,
        y + 10.0,
        22.0,
        LIGHTGRAY,
    );
    let verdict = match profile.expected_hit_shot() {
        Some(1) => t!("custom-verdict-first"),
        Some(n) => t!("custom-verdict-by", n = n),
        None => t!("custom-verdict-never"),
    };
    ui::text_at(&verdict, rx + 20.0, y + 45.0, 24.0, GOLD);
    ui::text_at(
        t!("custom-on-target", angle = HIT_ANGLE, power = HIT_POWER),
        rx + 20.0,
        y + 72.0,
        18.0,
        GRAY,
    );

    ui::button(
        // Under the right panel: the explanation on the left runs long.
        Rect::new(rx + rw - 240.0, 640.0, 240.0, 56.0),
        &t!("done"),
        true,
    ) || is_key_pressed(KeyCode::Escape)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn lineup_spends_exactly_the_budget() {
        for difficulty in 0..CUSTOM {
            for bots in 1..=3 {
                let team = lineup(difficulty, bots, AiProfile::custom());
                let spent: usize = team
                    .iter()
                    .map(|p| PRESETS.iter().position(|q| q.name == p.name).unwrap() + 1)
                    .sum();
                assert_eq!(team.len(), bots);
                assert_eq!(spent, threat_budget(difficulty, bots));
            }
        }
        // Normal: one Shooter alone, a softer mix when outnumbered.
        assert_eq!(threat_budget(1, 1), 2);
        assert_eq!(threat_budget(1, 3), 4);
        // Brutal: all Snipers.
        assert_eq!(threat_budget(3, 3), 9);
    }

    #[test]
    fn ladder_moves_one_step_and_stays_in_range() {
        assert_eq!(ladder_step(1, true), 2);
        assert_eq!(ladder_step(1, false), 0);
        assert_eq!(ladder_step(0, false), 0);
        assert_eq!(ladder_step(CUSTOM - 1, true), CUSTOM - 1);
        assert_eq!(ladder_step(CUSTOM, true), CUSTOM);
    }

    #[test]
    fn presets_get_harder() {
        let shots: Vec<u32> = PRESETS
            .iter()
            .map(|p| p.expected_hit_shot().unwrap_or(99))
            .collect();
        assert!(shots[0] > shots[1] && shots[1] > shots[2], "{shots:?}");
    }

    #[test]
    fn error_halves_with_learning_half() {
        let p = PRESETS[1];
        let (a1, _) = p.typical_error(1);
        let (a2, _) = p.typical_error(2);
        assert!((a2 - a1 * 0.5).abs() < 1e-4);
    }
}
