//! Saving and loading, as JSON files in the platform data folder
//! (`~/.local/share/scorch` on Linux, `%APPDATA%\scorch` on Windows,
//! `~/Library/Application Support/scorch` on macOS):
//!   - `run.json`: the game in progress, so you can continue where you left off.
//!     Rewritten at the start of every turn, deleted when the game ends.
//!   - `profile.json`: settings and all-time stats.
//!
//! Each file carries the `FORMAT` it was written in. A file that can't be read (damaged, or written by a newer
//! version of the game) is moved aside as `<name>.unreadable.json`, so the next save doesn't overwrite it, and the
//! player is told.

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
use crate::ladder::Ladder;
use crate::player::Player;
use crate::settings::Settings;
use crate::setup::Setup;
use crate::stats::Stats;

const RUN: &str = "run.json";
const PROFILE: &str = "profile.json";

/// Version of the save files, written into each as `"version"`. Raise it with a change an older version of the game
/// can't read; new fields with `#[serde(default)]` don't need it. Files without it (before 0.7) count as 0.
const FORMAT: u64 = 1;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum File {
    Profile,
    Run,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Problem {
    /// Not what the game wrote: cut short, edited by hand, from a broken disk.
    Damaged,
    /// Written by a newer version of the game.
    Newer,
}

/// A save file that couldn't be used.
#[derive(Clone, Debug)]
pub struct Unreadable {
    pub file: File,
    pub problem: Problem,
    /// Where it was moved to (`None` if it couldn't be moved: it then stays where it was).
    pub kept: Option<PathBuf>,
}

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
    /// Where the player stands on the ladder.
    pub ladder: Ladder,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            stats: Stats::default(),
            setup: Setup::default(),
            settings: Settings::default(),
            custom: AiProfile::custom(),
            intro_seen: false,
            ladder: Ladder::default(),
        }
    }
}

pub fn run_exists() -> bool {
    path(RUN).is_some_and(|p| p.exists())
}

pub fn store_run(run: &RunOut<'_>) {
    store(RUN, run);
}

/// The saved game; `Err(None)` if there is none, `Err(Some(..))` if it couldn't be read.
pub fn load_run() -> Result<RunIn, Option<Unreadable>> {
    load(RUN).map_err(|problem| problem.map(|problem| unreadable(File::Run, RUN, problem)))
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

/// The profile, or a fresh one; with what went wrong if there was a file that couldn't be read.
pub fn load_profile() -> (Profile, Option<Unreadable>) {
    let (mut profile, problem) = match load::<Profile>(PROFILE) {
        Ok(profile) => (profile, None),
        Err(problem) => (Profile::default(), problem),
    };
    profile.setup.sanitize();
    let problem = problem.map(|problem| unreadable(File::Profile, PROFILE, problem));
    (profile, problem)
}

/// `value` as the text of a save file: its JSON, with the format version added.
fn stamped<T: Serialize>(value: &T) -> serde_json::Result<String> {
    let mut json = serde_json::to_value(value)?;
    if let Some(fields) = json.as_object_mut() {
        fields.insert("version".to_string(), FORMAT.into());
    }
    serde_json::to_string(&json)
}

/// Reads the text of a save file.
fn parse<T: DeserializeOwned>(text: &str) -> Result<T, Problem> {
    let json: serde_json::Value = serde_json::from_str(text).map_err(|_| Problem::Damaged)?;
    let version = json.get("version").and_then(serde_json::Value::as_u64);
    if version.unwrap_or(0) > FORMAT {
        return Err(Problem::Newer);
    }
    serde_json::from_value(json).map_err(|e| {
        eprintln!("Unreadable save file: {e}");
        Problem::Damaged
    })
}

/// Moves the unreadable file `name` aside, so the next save doesn't overwrite it.
fn unreadable(file: File, name: &str, problem: Problem) -> Unreadable {
    let kept = path(name).and_then(|from| {
        let to = from.with_extension("unreadable.json");
        let moved = !READ_ONLY.load(Ordering::Relaxed) && fs::rename(&from, &to).is_ok();
        moved.then_some(to)
    });
    Unreadable {
        file,
        problem,
        kept,
    }
}

fn path(name: &str) -> Option<PathBuf> {
    dirs::data_dir().map(|d| d.join("scorch").join(name))
}

/// The file `name`; `Err(None)` if there is none.
fn load<T: DeserializeOwned>(name: &str) -> Result<T, Option<Problem>> {
    let text = path(name)
        .and_then(|path| fs::read_to_string(path).ok())
        .ok_or(None)?;
    parse(&text).map_err(Some)
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
        fs::write(&tmp, stamped(value)?)?;
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
    fn a_save_file_carries_its_version() {
        let text = stamped(&Profile::default()).unwrap();
        let json: serde_json::Value = serde_json::from_str(&text).unwrap();
        assert_eq!(json["version"], FORMAT);
        assert!(parse::<Profile>(&text).is_ok());
    }

    #[test]
    fn older_files_load_and_newer_or_damaged_ones_are_told_apart() {
        // Before 0.7 files had no version.
        let old = r#"{"intro_seen":true}"#;
        assert!(parse::<Profile>(old).is_ok_and(|p| p.intro_seen));
        let newer = format!(r#"{{"intro_seen":true,"version":{}}}"#, FORMAT + 1);
        assert_eq!(parse::<Profile>(&newer).err(), Some(Problem::Newer));
        for damaged in ["", "{\"intro_seen\":tr", r#"{"intro_seen":"yes"}"#, "42"] {
            assert_eq!(
                parse::<Profile>(damaged).err(),
                Some(Problem::Damaged),
                "{damaged}"
            );
        }
    }

    #[test]
    fn a_save_from_before_an_item_existed_still_loads() {
        let mut player = Player::new("You", RED, None);
        player.items = [2, 1, 0, 1, 3];
        player.items_on = [true, false, true, true, true];
        let mut saved = serde_json::to_value(&player).unwrap();
        // Four items then: the fifth starts at none, switched on.
        saved["items"].as_array_mut().unwrap().truncate(4);
        saved["items_on"].as_array_mut().unwrap().truncate(4);
        let loaded: Player = serde_json::from_value(saved).unwrap();
        assert_eq!(loaded.items, [2, 1, 0, 1, 0]);
        assert_eq!(loaded.items_on, [true, false, true, true, true]);
    }

    #[test]
    fn run_survives_a_save_and_load() {
        let mut players = vec![
            Player::new("You", RED, None),
            Player::new("Bot", RED, Some(PRESETS[2])),
        ];
        players[0].money = 321;
        let setup = Setup::default();
        let battle = Battle::new(&players, vec![1, 0], 3, 5, setup.rules());
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
    fn ammo_saved_before_new_weapons_still_loads() {
        let mut json = serde_json::to_value(Player::new("You", RED, None)).unwrap();
        json["ammo"] = serde_json::json!([0, 4, 1, 0, 2]);
        let player: Player = serde_json::from_value(json).unwrap();
        assert_eq!(player.ammo[1], 4);
        assert_eq!(player.ammo[4], 2);
        assert!(
            player.ammo[5..].iter().all(|&n| n == 0),
            "new weapons start empty"
        );
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
        assert_eq!(load("After24"), SuddenDeath::After24);
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
