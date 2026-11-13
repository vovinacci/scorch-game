// No console window behind the game in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Scorch: a Scorched Earth style artillery game.
//!
//! The game is a state machine. `Game::screen` says which screen is active;
//! every frame `Game::step` draws it, handles input, and returns the next screen.
//! How to play is in `docs/GUIDE.md`; exact numbers and internals in `docs/DEVELOPMENT.md`.

use macroquad::prelude::*;

mod ai;
mod battle;
mod choice;
mod crash;
mod economy;
mod game;
mod i18n;
mod icons;
mod items;
mod ladder;
mod logo;
mod player;
mod rng;
mod save;
mod settings;
mod setup;
mod shop;
mod sound;
mod stats;
mod terrain;
mod ui;
mod update;
mod weapons;
mod world;

/// The command line value after `flag`, as in `--screen shop`.
fn arg_after(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == flag)?;
    args.get(at + 1).cloned()
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Scorch".to_string(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        icon: Some(icon()),
        ..Default::default()
    }
}

/// The window, taskbar and Dock icon. Without it the OS shows miniquad's own logo, even over the
/// icon of a packaged app.
fn icon() -> miniquad::conf::Icon {
    fn rgba<const N: usize>(png: &[u8]) -> [u8; N] {
        let image = Image::from_file_with_format(png, Some(ImageFormat::Png)).expect("icon PNG");
        image.bytes.try_into().expect("icon size")
    }
    miniquad::conf::Icon {
        small: rgba(include_bytes!("../assets/icon/scorch-16.png")),
        medium: rgba(include_bytes!("../assets/icon/scorch-32.png")),
        big: rgba(include_bytes!("../assets/icon/scorch-64.png")),
    }
}

/// `--crash-test`: a deliberate panic, to try the crash report.
fn crash_test() -> ! {
    panic!("crash test requested with --crash-test");
}

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("{}", crash::version_line());
        return;
    }
    println!("{}", crash::version_line());
    crash::install();
    if std::env::args().any(|a| a == "--update") {
        std::process::exit(update::command_line());
    }
    if std::env::args().any(|a| a == "--crash-test") {
        crash_test();
    }
    // A screenshot that can't be saved would only fail (as a crash) after the window is up: say so now.
    if let Some(path) = arg_after("--shot")
        && let Some(problem) = shot_problem(std::path::Path::new(&path))
    {
        eprintln!("--shot {path}: {problem}");
        std::process::exit(2);
    }
    macroquad::Window::from_config(window_conf(), game::run());
}

/// Why a screenshot can't be saved to `path`, if it can't: the name must end in `.png` and its folder must exist.
fn shot_problem(path: &std::path::Path) -> Option<&'static str> {
    let png = path
        .extension()
        .is_some_and(|e| e.eq_ignore_ascii_case("png"));
    let folder = path.parent().filter(|p| !p.as_os_str().is_empty());
    if !png {
        Some("the file needs a name that ends in .png, like shot.png")
    } else if folder.is_some_and(|folder| !folder.is_dir()) {
        Some("the folder doesn't exist")
    } else {
        None
    }
}

/// The guides must name every weapon and setting value: these tests fail when they drift apart.
#[cfg(test)]
mod guide_tests {
    use crate::choice::Choice;
    use crate::i18n::{self, Language};
    use crate::world::{GravityLevel, Sky, TerrainKind, Walls, WindLevel};

    const GUIDE: &str = include_str!("../docs/GUIDE.md");

    const GUIDE_UK: &str = include_str!("../docs/GUIDE.uk.md");

    /// Every name in on-screen text, in the language set by `language`.
    fn names(language: Language) -> Vec<String> {
        i18n::set_language(language);
        let mut names: Vec<String> = Vec::new();
        names.extend(
            crate::weapons::WEAPONS
                .iter()
                .map(crate::weapons::Weapon::text),
        );
        names.extend(crate::items::ITEMS.iter().map(crate::items::Item::text));
        names.extend((0..crate::ai::DIFFICULTIES.len()).map(crate::ai::difficulty_text));
        names.extend(
            crate::ai::PRESETS
                .iter()
                .map(|p| crate::ai::skill_text(p.name)),
        );
        names.extend(texts::<TerrainKind>());
        names.extend(texts::<WindLevel>());
        names.extend(texts::<GravityLevel>());
        names.extend(texts::<Walls>());
        names.extend(texts::<Sky>());
        names.extend(texts::<crate::setup::TurnOrder>());
        names.extend(texts::<crate::setup::AimHelp>());
        i18n::set_language(Language::English);
        names
    }

    fn texts<T: Choice>() -> Vec<String> {
        T::ALL.iter().map(|v| v.text()).collect()
    }

    fn assert_in(guide: &str, file: &str, names: Vec<String>) {
        for name in names {
            assert!(guide.contains(&name), "{file} does not mention {name:?}");
        }
    }

    #[test]
    fn guide_lists_every_weapon_and_setting_value() {
        assert_in(GUIDE, "docs/GUIDE.md", names(Language::English));
    }

    #[test]
    fn ukrainian_guide_lists_every_weapon_and_setting_value() {
        assert_in(GUIDE_UK, "docs/GUIDE.uk.md", names(Language::Ukrainian));
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn a_screenshot_needs_a_png_name_in_a_folder_that_exists() {
        use std::path::Path;
        let problem = |path: &str| super::shot_problem(Path::new(path));
        assert_eq!(problem("shot.png"), None);
        assert_eq!(problem("src/shot.PNG"), None);
        // A file called just ".png" has no extension: the image format can't be told from it.
        assert!(problem("src/.png").is_some());
        assert!(problem("shot.jpg").is_some());
        assert!(problem("shot").is_some());
        assert!(problem("no-such-folder/shot.png").is_some());
    }

    #[test]
    fn icon_loads() {
        super::icon();
    }
}
