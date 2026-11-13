//! Developer aids: opening one screen directly (`--screen`), scripted menu keys (`--keys`) and screenshots (`--shot`).

use macroquad::prelude::*;

use super::{Game, Screen};
use crate::{crash, items, save, ui};

impl Game {
    /// Developer aid: `scorch --screen <name> [--shot file.png]` opens one
    /// screen directly with a fresh game, to check layouts (for example in
    /// every language); `--shot` saves a screenshot and quits. Saving is off. Names: intro, menu, about, setup, custom, settings, pause-settings, stats, crash, unreadable, arsenal, buried, watch,
    /// battle, pause, over, defence, log, shop, abandon, final.
    #[expect(clippy::too_many_lines, reason = "one arm for each developer screen")]
    pub(super) fn open_screen(&mut self, name: &str) -> Screen {
        self.has_saved_run = false;
        let game_screen = |game: &mut Game, paused: bool| match game.start() {
            Screen::Battle { battle, .. } => Screen::Battle { battle, paused },
            other => other,
        };
        match name {
            "intro" => Screen::Intro { time: 0.0 },
            "about" => Screen::About,
            "setup" => Screen::Setup,
            "custom" => Screen::CustomBot,
            "settings" => Screen::Settings {
                back: Box::new(Screen::Menu),
            },
            "pause-settings" => Screen::Settings {
                back: Box::new(game_screen(self, true)),
            },
            "stats" => Screen::Stats,
            "unreadable" => {
                self.save_problem = Some(sample_unreadable());
                Screen::Menu
            }
            "crash" => {
                let report = |t| {
                    std::path::PathBuf::from(format!("/home/you/.local/share/scorch/crash-{t}.log"))
                };
                self.crash = Some(crash::Pending {
                    paths: vec![report(1_793_470_061), report(1_793_460_000)],
                    text: String::new(),
                });
                Screen::Menu
            }
            "battle" => game_screen(self, false),
            "pause" => game_screen(self, true),
            // A battle where every tank carries some of every weapon.
            "arsenal" => {
                let screen = game_screen(self, false);
                for p in &mut self.players {
                    for (w, n) in p.ammo.iter_mut().enumerate().skip(1) {
                        *n = 1 + w as u32 % 3;
                    }
                }
                screen
            }
            // A battle where every tank has raised a shield (one of each kind).
            // A Dirt Bomb right on every tank.
            "buried" => match game_screen(self, false) {
                Screen::Battle { mut battle, .. } => {
                    battle.bury_all();
                    Screen::Battle {
                        battle,
                        paused: false,
                    }
                }
                other => other,
            },
            "defence" => match game_screen(self, false) {
                Screen::Battle { mut battle, .. } => {
                    for (i, p) in self.players.iter_mut().enumerate() {
                        // A shield of each kind, and batteries for the hint on the player's turn.
                        p.items[1 + i % 3] = 1;
                        p.items[items::BATTERY] = 2;
                    }
                    battle.raise_shields(&mut self.players);
                    Screen::Battle {
                        battle,
                        paused: false,
                    }
                }
                other => other,
            },
            // You flee: `watch` lets the bots play the round out, with sound (a battle that runs by itself); `over`
            // skips to the bot that wins it.
            "watch" | "over" => match game_screen(self, false) {
                Screen::Battle { mut battle, .. } => {
                    battle.flee(&mut self.players);
                    if name == "over" {
                        battle.skip_to_end(&mut self.players);
                    }
                    Screen::Battle {
                        battle,
                        paused: false,
                    }
                }
                other => other,
            },
            "log" => {
                self.log_open = true;
                game_screen(self, true)
            }
            "shop" => {
                game_screen(self, false);
                Screen::Shop { shopper: 0 }
            }
            "abandon" => Screen::ConfirmAbandon {
                back: Box::new(game_screen(self, true)),
            },
            "new-game" => {
                self.has_saved_run = true;
                Screen::ConfirmNewGame
            }
            // A game the player lost 2 rounds to 3, so a ladder game shows its note.
            "final" => {
                game_screen(self, false);
                self.players[0].wins = 2;
                self.players[1].wins = 3;
                self.end_game();
                Screen::Final
            }
            _ => Screen::Menu,
        }
    }
}

/// For `--screen unreadable`: the notice about a save file that couldn't be read.
pub(super) fn sample_unreadable() -> save::Unreadable {
    save::Unreadable {
        file: save::File::Profile,
        problem: save::Problem::Damaged,
        kept: Some("/home/you/.local/share/scorch/profile.unreadable.json".into()),
    }
}

/// A menu key by its `--keys` name.
pub(super) fn menu_key(name: &str) -> Option<KeyCode> {
    match name {
        "up" => Some(KeyCode::Up),
        "down" => Some(KeyCode::Down),
        "left" => Some(KeyCode::Left),
        "right" => Some(KeyCode::Right),
        "enter" => Some(KeyCode::Enter),
        _ => None,
    }
}

/// The 16:9 game area of a screenshot, without the bands around it, so shots look the same in any
/// window.
pub(super) fn game_area(screen: &Image) -> Image {
    let (w, h) = (f32::from(screen.width), f32::from(screen.height));
    let scale = (w / ui::WIDTH).min(h / ui::HEIGHT);
    let (gw, gh) = ((ui::WIDTH * scale).floor(), (ui::HEIGHT * scale).floor());
    screen.sub_image(Rect::new(
        ((w - gw) / 2.0).floor(),
        ((h - gh) / 2.0).floor(),
        gw,
        gh,
    ))
}
