//! Saving and loading, as JSON files in the platform data folder
//! (`~/.local/share/scorch` on Linux, `%APPDATA%\scorch` on Windows,
//! `~/Library/Application Support/scorch` on macOS):
//!   - `run.json`: the game in progress, so you can continue where you left off.
//!     Rewritten at the start of every turn, deleted when the game ends.
//!   - `profile.json`: settings and all-time stats.

use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};

/// Set by `--screen` (a developer aid): nothing is written or deleted.
static READ_ONLY: AtomicBool = AtomicBool::new(false);

pub fn set_read_only() {
    READ_ONLY.store(true, Ordering::Relaxed);
}

use macroquad::prelude::{Color, Vec2};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};

use crate::ai::AiProfile;
use crate::battle::Battle;
use crate::player::Player;
use crate::settings::Settings;
use crate::setup::Setup;
use crate::stats::Stats;

const RUN: &str = "run.json";
const PROFILE: &str = "profile.json";

/// Run being written: borrows the live game state.
#[derive(Serialize)]
pub struct RunOut<'a> {
    pub players: &'a [Player],
    pub setup: &'a Setup,
    pub round: u32,
    pub stage: StageOut<'a>,
}

#[derive(Serialize)]
pub enum StageOut<'a> {
    Battle(&'a Battle),
    Shop { shopper: usize },
}

/// Run being read back: owns everything.
#[derive(Deserialize)]
pub struct RunIn {
    pub players: Vec<Player>,
    #[serde(default)]
    pub setup: Setup,
    pub round: u32,
    pub stage: StageIn,
}

#[derive(Deserialize)]
pub enum StageIn {
    Battle(Box<Battle>),
    Shop { shopper: usize },
}

#[derive(Serialize, Deserialize)]
#[serde(default)]
pub struct Profile {
    pub stats: Stats,
    /// The last "New game" setup, offered again next time.
    pub setup: Setup,
    pub settings: Settings,
    pub custom: AiProfile,
    /// The start-up dedication was shown (it shows on the first run only).
    pub intro_seen: bool,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            stats: Stats::default(),
            setup: Setup::default(),
            settings: Settings::default(),
            custom: AiProfile::custom(),
            intro_seen: false,
        }
    }
}

pub fn run_exists() -> bool {
    path(RUN).is_some_and(|p| p.exists())
}

pub fn store_run(run: &RunOut) {
    store(RUN, run);
}

pub fn load_run() -> Option<RunIn> {
    load(RUN)
}

pub fn remove_run() {
    if READ_ONLY.load(Ordering::Relaxed) {
        return;
    }
    if let Some(p) = path(RUN) {
        let _ = fs::remove_file(p);
    }
}

pub fn store_profile(profile: &Profile) {
    store(PROFILE, profile);
}

pub fn load_profile() -> Profile {
    let mut profile: Profile = load(PROFILE).unwrap_or_default();
    profile.setup.sanitize();
    profile
}

fn path(name: &str) -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("scorch").join(name))
}

fn load<T: DeserializeOwned>(name: &str) -> Option<T> {
    let text = fs::read_to_string(path(name)?).ok()?;
    match serde_json::from_str(&text) {
        Ok(value) => Some(value),
        Err(e) => {
            eprintln!("Ignoring unreadable {name}: {e}");
            None
        }
    }
}

/// Writes to a temporary file first, then renames it over the old one,
/// so a crash mid-write never leaves a half-written save.
fn store<T: Serialize>(name: &str, value: &T) {
    let Some(path) = path(name) else {
        return;
    };
    if READ_ONLY.load(Ordering::Relaxed) {
        return;
    }
    let result = (|| -> std::io::Result<()> {
        if let Some(dir) = path.parent() {
            fs::create_dir_all(dir)?;
        }
        let tmp = path.with_extension("tmp");
        fs::write(&tmp, serde_json::to_string(value)?)?;
        fs::rename(tmp, &path)
    })();
    if let Err(e) = result {
        eprintln!("Could not save {name}: {e}");
    }
}

/// serde descriptions of macroquad types, used as `#[serde(with = "...")]`.
#[derive(Serialize, Deserialize)]
#[serde(remote = "Color")]
pub struct ColorDef {
    pub r: f32,
    pub g: f32,
    pub b: f32,
    pub a: f32,
}

#[derive(Serialize, Deserialize)]
#[serde(remote = "Vec2")]
pub struct Vec2Def {
    pub x: f32,
    pub y: f32,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::PRESETS;
    use macroquad::prelude::RED;

    #[test]
    fn run_survives_a_save_and_load() {
        let mut players = vec![
            Player::new("You", RED, None),
            Player::new("Bot", RED, Some(PRESETS[2])),
        ];
        players[0].money = 321;
        let setup = Setup::default();
        let battle = Battle::new(&players, 1, 3, 5, setup.rules());
        let json = serde_json::to_string(&RunOut {
            players: &players,
            setup: &setup,
            round: 3,
            stage: StageOut::Battle(&battle),
        })
        .unwrap();

        let run: RunIn = serde_json::from_str(&json).unwrap();
        assert_eq!(run.round, 3);
        assert_eq!(run.players[0].money, 321);
        assert_eq!(run.players[1].bot.unwrap().name, "Sniper");
        let StageIn::Battle(loaded) = run.stage else {
            panic!("expected a battle");
        };
        // Same battle state: re-serializing gives identical JSON.
        let again = serde_json::to_string(&StageOut::Battle(&loaded)).unwrap();
        let before = serde_json::to_string(&StageOut::Battle(&battle)).unwrap();
        assert_eq!(again, before);
    }

    #[test]
    fn old_or_partial_profile_still_loads() {
        let profile: Profile = serde_json::from_str(r#"{"stats":{"kills":7}}"#).unwrap();
        assert_eq!(profile.stats.kills, 7);
        assert_eq!(profile.setup.difficulty, 1);
        assert_eq!(profile.setup.rounds, 5);
    }

    #[test]
    fn old_sudden_death_turns_map_to_shots_each() {
        use crate::setup::SuddenDeath;
        let load = |v: &str| {
            let json = format!(r#"{{"setup":{{"sudden_death":"{v}","rounds":7}}}}"#);
            let p: Profile = serde_json::from_str(&json).unwrap();
            assert_eq!(p.setup.rounds, 7, "the rest of the setup survives");
            p.setup.sudden_death
        };
        assert_eq!(load("After15"), SuddenDeath::After8);
        assert_eq!(load("After25"), SuddenDeath::After12);
        assert_eq!(load("After40"), SuddenDeath::After16);
        assert_eq!(load("Off"), SuddenDeath::Off);
        assert_eq!(load("After16"), SuddenDeath::After16);
    }

    #[test]
    fn old_system_language_is_detected_again() {
        use crate::i18n::Language;
        let old: Profile =
            serde_json::from_str(r#"{"settings":{"language":"System","fullscreen":true}}"#)
                .unwrap();
        assert_eq!(old.settings.language, None);
        assert!(old.settings.fullscreen, "the rest of the settings survive");
        let chosen: Profile =
            serde_json::from_str(r#"{"settings":{"language":"Ukrainian"}}"#).unwrap();
        assert_eq!(chosen.settings.language, Some(Language::Ukrainian));
    }
}
