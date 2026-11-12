//! Bot skill profiles and the custom profile editor.
//!
//! Aim model (the editor screen explains the same thing to the player):
//!   1. The bot simulates thousands of shots and finds the perfect one.
//!   2. First shot at a new target: it adds an error of 40-100% of E, random sign.
//!   3. After every shot: error = error * L + wobble, wobble random within +-WOBBLE * E.
//!
//! So the typical error on shot n is about 0.7 * E * L^(n-1).

use crate::rng::gen_range;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::i18n::t;
use crate::ui;

/// Fresh random noise added to every shot, as a fraction of the first-shot error E.
pub const WOBBLE: f32 = 0.12;
/// Roughly what counts as "on target" for the hit estimate on the editor screen.
const HIT_ANGLE: f32 = 2.0;
const HIT_POWER: f32 = 30.0;

/// Whom a bot shoots at.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Targets {
    /// Revenge, then the same target again, then the best scored enemy (`Battle::choose_target`).
    #[default]
    Usual,
    /// Whoever has won the most rounds, whatever else is going on.
    Leader,
}

/// Which shots a bot looks for.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum Shots {
    /// The one that lands closest, at any angle.
    #[default]
    Any,
    /// High lobs only, while one of them gets near the target.
    Lobs,
    /// Off a wall where the walls bounce or wrap and such a shot lands about as close.
    Walls,
}

#[derive(Clone, Copy, Serialize)]
pub struct AiProfile {
    pub name: &'static str,
    /// E for angle: max angle error (degrees) of the first shot at a new target.
    pub first_angle_error: f32,
    /// E for power: max power error of the first shot at a new target.
    pub first_power_error: f32,
    /// L: fraction of the error kept after each shot. Lower = learns faster.
    pub learning: f32,
    pub targets: Targets,
    pub shots: Shots,
}

/// The bot types. The first three keep their places: the ladder's rungs and older code count on them.
pub const PRESETS: [AiProfile; 8] = [
    AiProfile {
        name: "Moron",
        first_angle_error: 25.0,
        first_power_error: 300.0,
        learning: 0.8,
        targets: Targets::Usual,
        shots: Shots::Any,
    },
    AiProfile {
        name: "Shooter",
        first_angle_error: 12.0,
        first_power_error: 180.0,
        learning: 0.5,
        targets: Targets::Usual,
        shots: Shots::Any,
    },
    AiProfile {
        name: "Sniper",
        first_angle_error: 6.0,
        first_power_error: 90.0,
        learning: 0.3,
        targets: Targets::Usual,
        shots: Shots::Any,
    },
    // Between a Shooter and a Sniper in aim, but always after whoever is winning.
    AiProfile {
        name: "Spoiler",
        first_angle_error: 9.0,
        first_power_error: 135.0,
        learning: 0.4,
        targets: Targets::Leader,
        shots: Shots::Any,
    },
    // Close to perfect: its first shot nearly hits.
    AiProfile {
        name: "Cyborg",
        first_angle_error: 3.0,
        first_power_error: 45.0,
        learning: 0.2,
        targets: Targets::Usual,
        shots: Shots::Any,
    },
    // A Shooter that only lobs its shells high.
    AiProfile {
        name: "Tosser",
        first_angle_error: 12.0,
        first_power_error: 180.0,
        learning: 0.5,
        targets: Targets::Usual,
        shots: Shots::Lobs,
    },
    // Aims like a Spoiler, and plays the walls where they bounce or wrap.
    AiProfile {
        name: "Poolshark",
        first_angle_error: 9.0,
        first_power_error: 135.0,
        learning: 0.4,
        targets: Targets::Usual,
        shots: Shots::Walls,
    },
    // Plays each round as one of the others, picked at random (`AiProfile::for_round`); its own numbers are never
    // used in a battle.
    AiProfile {
        name: CHOOSER,
        first_angle_error: 12.0,
        first_power_error: 180.0,
        learning: 0.5,
        targets: Targets::Usual,
        shots: Shots::Any,
    },
];

const CHOOSER: &str = "Chooser";

/// What each type costs in a line-up, in half threat points, in the order of `PRESETS`: Moron 1, Tosser 1.5,
/// Shooter 2, Poolshark and Chooser 2.5, Sniper and Spoiler 3, Cyborg 4. Measured with `balance_types`.
const THREAT: [usize; PRESETS.len()] = [2, 4, 6, 6, 8, 3, 5, 5];

impl AiProfile {
    /// The type a bot plays a round as: itself, or for a Chooser any of the others.
    pub fn for_round(self) -> Self {
        if self.name != CHOOSER {
            return self;
        }
        let others: Vec<&AiProfile> = PRESETS.iter().filter(|p| p.name != CHOOSER).collect();
        *others[gen_range(0, others.len())]
    }

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
        let k = 0.7 * self.learning.powi(n.cast_signed() - 1);
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

/// Threat points per difficulty for one, two and three bots (see `THREAT` for what a bot costs). Extra bots add
/// less than the first one, since they also fight each other. Measured with `balance_difficulty`.
const BUDGETS: [[usize; 3]; 4] = [[1, 2, 3], [2, 3, 4], [3, 4, 6], [4, 6, 9]];

/// Threat points to spend on `bots` opponents.
pub fn threat_budget(difficulty: usize, bots: usize) -> usize {
    let row = BUDGETS[difficulty];
    row[bots.clamp(1, row.len()) - 1]
}

/// Builds the opponents: everyone starts as a Moron, then random bots move up to the next dearer cost while the
/// budget lasts, and each bot becomes a type of its cost (a random one where several cost the same). Same budget,
/// different mix each game.
pub fn lineup(difficulty: usize, bots: usize, custom: AiProfile) -> Vec<AiProfile> {
    if difficulty == CUSTOM {
        return vec![custom; bots];
    }
    let mut costs: Vec<usize> = THREAT.to_vec();
    costs.sort_unstable();
    costs.dedup();
    // Each bot's place in `costs`.
    let mut steps = vec![0; bots];
    let mut spare = 2 * threat_budget(difficulty, bots) - costs[0] * bots;
    let rise = |step: usize| costs.get(step + 1).map(|dearer| dearer - costs[step]);
    loop {
        let open: Vec<usize> = (0..bots)
            .filter(|&i| rise(steps[i]).is_some_and(|by| by <= spare))
            .collect();
        if open.is_empty() {
            break;
        }
        let i = open[gen_range(0, open.len())];
        spare -= rise(steps[i]).unwrap_or(0);
        steps[i] += 1;
    }
    steps.sort_unstable_by(|a, b| b.cmp(a));
    steps
        .into_iter()
        .map(|step| of_threat(costs[step], bots == 1))
        .collect()
}

/// A bot type that costs `cost` half threat points. A `lone` bot is never one that differs only in whom it shoots
/// at: with one enemy a Spoiler is just a worse shot than a Sniper for the same points.
fn of_threat(cost: usize, lone: bool) -> AiProfile {
    let fits: Vec<usize> = (0..PRESETS.len())
        .filter(|&i| THREAT[i] == cost && !(lone && PRESETS[i].targets != Targets::Usual))
        .collect();
    PRESETS[fits[gen_range(0, fits.len())]]
}

/// One-line explanation for the title screen.
pub fn describe(difficulty: usize, bots: usize, custom: &AiProfile) -> String {
    if difficulty == CUSTOM {
        return t!("difficulty-custom-note", summary = custom.summary());
    }
    t!(
        "difficulty-note",
        bots = bots,
        points = threat_budget(difficulty, bots)
    )
}

/// Loading form of `AiProfile`: names are `&'static str`, so a saved name is
/// mapped back to a known preset name (anything else becomes "Custom").
#[derive(Deserialize)]
struct SavedProfile {
    name: String,
    first_angle_error: f32,
    first_power_error: f32,
    learning: f32,
    #[serde(default)]
    targets: Targets,
    #[serde(default)]
    shots: Shots,
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
            targets: saved.targets,
            shots: saved.shots,
        }
    }
}

/// Random value between 40% and 100% of `max`, either sign: first shots never land dead-on.
fn rough(max: f32) -> f32 {
    let e = gen_range(max * 0.4, max);
    if crate::rng::chance(0.5) { e } else { -e }
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
    // The types whose aim differs; the others share one of these and differ in what they do with it.
    for preset in [0, 1, 2, 4].map(|i| &PRESETS[i]) {
        if ui::button(
            Rect::new(bx, y - 16.0, 98.0, 36.0),
            &skill_text(preset.name),
            true,
        ) {
            *profile = AiProfile {
                name: "Custom",
                ..*preset
            };
        }
        bx += 107.0;
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
        let (angle, power) = profile.typical_error(n);
        let on_target = angle < HIT_ANGLE && power < HIT_POWER;
        let color = if on_target { GREEN } else { ORANGE };
        ui::text_at(t!("custom-shot", n = n), rx + 20.0, y, 22.0, LIGHTGRAY);
        draw_rectangle(rx + 140.0, y - 15.0, 180.0 * (angle / max_bar), 16.0, color);
        let miss = t!(
            "custom-miss",
            angle = format!("{angle:.1}"),
            power = power.round()
        );
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
                    .map(|p| THREAT[PRESETS.iter().position(|q| q.name == p.name).unwrap()])
                    .sum();
                assert_eq!(team.len(), bots);
                assert_eq!(spent, 2 * threat_budget(difficulty, bots));
            }
        }
        // Brutal: a Cyborg alone, as much as three Snipers for three.
        assert_eq!(lineup(3, 1, AiProfile::custom())[0].name, "Cyborg");
        assert_eq!(threat_budget(3, 3), 9);
    }

    #[test]
    fn presets_get_harder() {
        let shots: Vec<u32> = PRESETS
            .iter()
            .map(|p| p.expected_hit_shot().unwrap_or(99))
            .collect();
        assert!(shots[0] > shots[1] && shots[1] > shots[2], "{shots:?}");
        // A Spoiler sits between a Shooter and a Sniper; a Cyborg's first miss is about half a Sniper's.
        let first = |i: usize| PRESETS[i].typical_error(1).0;
        assert!(first(1) > first(3) && first(3) > first(2), "Spoiler");
        assert!(first(4) < first(2) * 0.6, "Cyborg");
    }

    #[test]
    fn lineups_use_every_type() {
        let mut seen = std::collections::BTreeSet::new();
        for round in 0..600 {
            let team = lineup(round % CUSTOM, 1 + round % 3, AiProfile::custom());
            seen.extend(team.iter().map(|p| p.name));
        }
        assert_eq!(seen.len(), PRESETS.len(), "{seen:?}");
        // Easy stays all Morons, and a lone Normal bot is a Shooter.
        assert!(
            lineup(0, 3, AiProfile::custom())
                .iter()
                .all(|p| p.name == "Moron")
        );
        assert_eq!(lineup(1, 1, AiProfile::custom())[0].name, "Shooter");
        // A lone bot is never a Spoiler: with one enemy there is no leader to pick.
        for _ in 0..50 {
            assert_eq!(lineup(2, 1, AiProfile::custom())[0].name, "Sniper");
        }
    }

    #[test]
    fn a_chooser_plays_a_round_as_any_other_type() {
        let chooser = *PRESETS.last().unwrap();
        assert_eq!(chooser.name, CHOOSER);
        let roles: std::collections::BTreeSet<&str> =
            (0..300).map(|_| chooser.for_round().name).collect();
        assert_eq!(roles.len(), PRESETS.len() - 1, "{roles:?}");
        assert!(!roles.contains(CHOOSER));
        // Everyone else is what it is.
        assert_eq!(PRESETS[2].for_round().name, "Sniper");
    }

    #[test]
    fn older_saves_load_without_a_target_rule() {
        let old =
            r#"{"name":"Sniper","first_angle_error":6.0,"first_power_error":90.0,"learning":0.3}"#;
        let bot: AiProfile = serde_json::from_str(old).unwrap();
        assert_eq!((bot.name, bot.targets), ("Sniper", Targets::Usual));
        let spoiler: AiProfile =
            serde_json::from_str(&serde_json::to_string(&PRESETS[3]).unwrap()).unwrap();
        assert_eq!(
            (spoiler.name, spoiler.targets),
            ("Spoiler", Targets::Leader)
        );
    }

    #[test]
    fn error_halves_with_learning_half() {
        let p = PRESETS[1];
        let (a1, _) = p.typical_error(1);
        let (a2, _) = p.typical_error(2);
        assert!((a2 - a1 * 0.5).abs() < 1e-4);
    }
}
