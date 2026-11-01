// No console window behind the game in Windows release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

//! Scorch: a Scorched Earth style artillery game.
//!
//! The game is a state machine. `Game::screen` says which screen is active;
//! every frame `Game::step` draws it, handles input, and returns the next screen.
//! How to play is in `docs/GUIDE.md`; exact numbers and internals in `docs/DEVELOPMENT.md`.

use macroquad::prelude::*;
use macroquad::rand::gen_range;

mod ai;
mod battle;
mod choice;
mod crash;
mod economy;
mod i18n;
mod logo;
mod player;
mod save;
mod settings;
mod setup;
mod shop;
mod stats;
mod terrain;
mod ui;
mod weapons;
mod world;

use ai::CUSTOM;
use battle::Battle;
use choice::Choice;
use economy::Income;
use i18n::{Language, t};
use player::Player;
use save::{Profile, RunOut, StageIn, StageOut};
use setup::{Preview, Setup, SetupAction};
use shop::ShopAction;
use stats::RoundStats;
use world::Sky;

const COLORS: [Color; 4] = [
    Color::new(0.95, 0.35, 0.3, 1.0),
    Color::new(0.35, 0.6, 1.0, 1.0),
    Color::new(0.4, 0.9, 0.4, 1.0),
    Color::new(0.95, 0.85, 0.3, 1.0),
];
const BOT_NAMES: [&str; 3] = ["Alpha", "Bravo", "Charlie"];

// One Screen exists at a time, so the big Battle variant costs nothing.
#[allow(clippy::large_enum_variant)]
enum Screen {
    /// The dedication, shown briefly at start-up.
    Intro {
        time: f32,
    },
    Menu,
    About,
    Setup,
    CustomBot,
    Settings,
    Stats,
    Battle {
        battle: Battle,
        paused: bool,
    },
    Shop {
        shopper: usize,
    },
    /// "Are you sure?" before abandoning; `back` is where Cancel returns to.
    ConfirmAbandon {
        back: Box<Screen>,
    },
    Final,
    /// Leave the main loop and close the game.
    Exit,
}

enum PauseAction {
    Resume,
    Flee,
    SelfDestruct,
    SaveToMenu,
    SaveAndQuit,
    Abandon,
    BattleLog,
}

struct Game {
    players: Vec<Player>,
    /// Setup of the game in progress (a copy of the profile's at the start).
    setup: Setup,
    round: u32,
    /// Settings, last setup and all-time stats, saved to disk.
    profile: Profile,
    has_saved_run: bool,
    /// Shown on the final screen when ladder mode changed the difficulty.
    ladder_note: Option<String>,
    preview: Preview,
    /// Crash report from a previous run, offered on the main menu.
    crash: Option<crash::Pending>,
    /// Shown on the main menu after the crash report was copied.
    crash_note: Option<String>,
    /// Full battle log opened from the pause menu.
    log_open: bool,
    screen: Screen,
}

impl Game {
    fn start(&mut self) -> Screen {
        self.start_with(self.profile.setup)
    }

    fn start_with(&mut self, setup: Setup) -> Screen {
        // A new game replaces the saved one: that one counts as abandoned.
        if self.has_saved_run {
            self.abandon();
        }
        self.setup = setup;
        let (humans, bots) = (self.setup.humans, self.setup.bots);
        self.players.clear();
        for (i, &color) in COLORS.iter().enumerate().take(humans) {
            let name = if humans == 1 {
                "You".to_string()
            } else {
                format!("Player {}", i + 1)
            };
            self.players.push(Player::new(&name, color, None));
        }
        let lineup = ai::lineup(self.setup.difficulty, bots, self.profile.custom);
        for (i, (name, profile)) in BOT_NAMES.iter().zip(lineup).enumerate() {
            let name = format!("{name} [{}]", profile.name);
            self.players
                .push(Player::new(&name, COLORS[humans + i], Some(profile)));
        }
        self.round = 1;
        self.battle()
    }

    fn battle(&mut self) -> Screen {
        for p in &mut self.players {
            p.income = Income::default();
            p.round_stats = RoundStats::default();
        }
        // First round: random starter. Later: last place shoots first.
        let first = if self.round == 1 {
            gen_range(0, self.players.len())
        } else {
            economy::underdog(&self.players)
        };
        Screen::Battle {
            battle: Battle::new(
                &self.players,
                first,
                self.round,
                self.setup.rounds,
                self.setup.rules(),
            ),
            paused: false,
        }
    }

    fn save_run(&mut self, stage: StageOut) {
        save::store_run(&RunOut {
            players: &self.players,
            setup: &self.setup,
            round: self.round,
            stage,
        });
        self.has_saved_run = true;
    }

    /// Between shots the battle is saved as is; mid-shot, the autosave
    /// from the start of this turn is kept instead.
    fn save_battle(&mut self, battle: &Battle) {
        if battle.can_save() {
            self.save_run(StageOut::Battle(battle));
        }
    }

    /// Saves whatever is on screen, if it can be saved. Used when the window is closed.
    fn save_screen(&mut self, screen: &Screen) {
        match screen {
            Screen::Battle { battle, .. } => self.save_battle(battle),
            &Screen::Shop { shopper } => self.save_run(StageOut::Shop { shopper }),
            Screen::ConfirmAbandon { back } => self.save_screen(back),
            _ => {}
        }
    }

    fn load_run(&mut self) -> Screen {
        let Some(run) = save::load_run() else {
            self.has_saved_run = false;
            return Screen::Menu;
        };
        self.players = run.players;
        self.setup = run.setup;
        self.round = run.round;
        match run.stage {
            StageIn::Battle(battle) => Screen::Battle {
                battle: *battle,
                paused: false,
            },
            StageIn::Shop { shopper } => Screen::Shop { shopper },
        }
    }

    /// Ends the game in progress for good: deletes the save and records it in stats.
    /// Rounds already played stay in the stats.
    fn abandon(&mut self) -> Screen {
        save::remove_run();
        self.has_saved_run = false;
        self.profile.stats.games_abandoned += 1;
        save::store_profile(&self.profile);
        Screen::Menu
    }

    fn enter_shop(&mut self, shopper: usize) -> Screen {
        self.save_run(StageOut::Shop { shopper });
        Screen::Shop { shopper }
    }

    /// Next human who still has to shop, starting from `from`.
    fn next_shopper(&self, from: usize) -> Option<usize> {
        (from..self.players.len()).find(|&i| !self.players[i].is_bot())
    }

    fn end_round(&mut self, winner: Option<usize>) -> Screen {
        economy::end_round(&mut self.players, winner);
        for (i, p) in self.players.iter().enumerate() {
            if !p.is_bot() {
                self.profile
                    .stats
                    .add_round(&p.round_stats, winner == Some(i), p.income.total());
            }
        }

        if self.round == self.setup.rounds {
            self.end_game();
            return Screen::Final;
        }
        save::store_profile(&self.profile);

        self.round += 1;
        for p in self.players.iter_mut().filter(|p| p.is_bot()) {
            shop::ai_shop(p);
        }
        match self.next_shopper(0) {
            Some(shopper) => self.enter_shop(shopper),
            None => self.battle(),
        }
    }

    fn end_game(&mut self) {
        let best = self.players.iter().map(|p| p.wins).max().unwrap_or(0);
        for p in self.players.iter().filter(|p| !p.is_bot()) {
            self.profile.stats.add_game(p.wins == best, p.wins);
        }
        self.ladder_note = None;
        if self.setup.ladder {
            let won = self.players.iter().any(|p| !p.is_bot() && p.wins == best);
            let before = self.profile.setup.difficulty;
            self.profile.setup.difficulty = ai::ladder_step(before, won);
            if self.profile.setup.difficulty != before {
                let level = ai::difficulty_text(self.profile.setup.difficulty);
                self.ladder_note = Some(if won {
                    t!("ladder-up", level = level)
                } else {
                    t!("ladder-down", level = level)
                });
            }
        }
        save::store_profile(&self.profile);
        save::remove_run();
        self.has_saved_run = false;
    }

    fn step(&mut self, screen: Screen) -> Screen {
        match screen {
            Screen::Intro { time } => {
                let next = intro(time);
                if matches!(next, Screen::Menu) {
                    self.profile.intro_seen = true;
                    save::store_profile(&self.profile);
                }
                next
            }
            Screen::Menu => self.main_menu(),
            Screen::About => {
                if about() {
                    Screen::Menu
                } else {
                    Screen::About
                }
            }

            Screen::Setup => {
                match setup::screen(
                    &mut self.profile.setup,
                    &self.profile.custom,
                    &mut self.preview,
                ) {
                    SetupAction::Stay => Screen::Setup,
                    SetupAction::Randomize => {
                        self.profile.setup.randomize();
                        Screen::Setup
                    }
                    SetupAction::EditCustom => Screen::CustomBot,
                    SetupAction::Back => {
                        save::store_profile(&self.profile);
                        Screen::Menu
                    }
                    SetupAction::Start => {
                        save::store_profile(&self.profile);
                        self.start()
                    }
                }
            }

            Screen::CustomBot => {
                if ai::editor(&mut self.profile.custom) {
                    self.profile.setup.difficulty = CUSTOM;
                    save::store_profile(&self.profile);
                    Screen::Setup
                } else {
                    Screen::CustomBot
                }
            }

            Screen::Settings => {
                if settings::screen(&mut self.profile.settings) {
                    save::store_profile(&self.profile);
                    Screen::Menu
                } else {
                    Screen::Settings
                }
            }

            Screen::Stats => {
                if stats::screen(&self.profile.stats) {
                    Screen::Menu
                } else {
                    Screen::Stats
                }
            }

            Screen::Battle {
                mut battle,
                paused: true,
            } => {
                battle.draw(&self.players, &self.profile.settings);
                if self.log_open {
                    battle::draw_history(&battle);
                    let back = Rect::new(540.0, 596.0, 200.0, 44.0);
                    if ui::button(back, &t!("back"), true) || is_key_pressed(KeyCode::Escape) {
                        self.log_open = false;
                    }
                    return Screen::Battle {
                        battle,
                        paused: true,
                    };
                }
                let your_turn = battle.is_human_turn(&self.players);
                let progress = battle.progress();
                match pause_menu(your_turn, battle.can_save(), &self.setup, progress) {
                    None => Screen::Battle {
                        battle,
                        paused: true,
                    },
                    Some(PauseAction::Resume) => Screen::Battle {
                        battle,
                        paused: false,
                    },
                    Some(PauseAction::Flee) => {
                        battle.flee(&mut self.players);
                        Screen::Battle {
                            battle,
                            paused: false,
                        }
                    }
                    Some(PauseAction::SelfDestruct) => {
                        battle.self_destruct(&mut self.players);
                        Screen::Battle {
                            battle,
                            paused: false,
                        }
                    }
                    Some(PauseAction::SaveToMenu) => {
                        self.save_battle(&battle);
                        Screen::Menu
                    }
                    Some(PauseAction::SaveAndQuit) => {
                        self.save_battle(&battle);
                        Screen::Exit
                    }
                    Some(PauseAction::BattleLog) => {
                        self.log_open = true;
                        Screen::Battle {
                            battle,
                            paused: true,
                        }
                    }
                    Some(PauseAction::Abandon) => Screen::ConfirmAbandon {
                        back: Box::new(Screen::Battle {
                            battle,
                            paused: true,
                        }),
                    },
                }
            }

            Screen::Battle {
                mut battle,
                paused: false,
            } => {
                let result = battle.frame(&mut self.players, &self.profile.settings);
                if battle.take_checkpoint() {
                    self.save_run(StageOut::Battle(&battle));
                }
                if let Some(end) = result {
                    return self.end_round(end.winner);
                }
                if battle.only_bots_left(&self.players) {
                    let label = t!(
                        "bots-only-speed",
                        speed = self.profile.settings.bot_speed.text()
                    );
                    ui::with_shadow(|| ui::text_at(label, 20.0, ui::HEIGHT - 20.0, 20.0, GOLD));
                    let skip = Rect::new(ui::WIDTH - 280.0, ui::HEIGHT - 60.0, 260.0, 44.0);
                    if ui::button(skip, &t!("skip-to-end"), true) || is_key_pressed(KeyCode::Enter)
                    {
                        battle.skip_to_end(&mut self.players);
                    }
                }
                if is_key_down(KeyCode::L) {
                    battle::draw_history(&battle);
                }
                Screen::Battle {
                    battle,
                    paused: is_key_pressed(KeyCode::Escape),
                }
            }

            Screen::Shop { shopper } => {
                let shoots_first = economy::underdog(&self.players) == shopper;
                match shop::frame(&mut self.players[shopper], shoots_first) {
                    ShopAction::Stay => Screen::Shop { shopper },
                    ShopAction::SaveAndExit => {
                        self.save_run(StageOut::Shop { shopper });
                        Screen::Menu
                    }
                    ShopAction::Abandon => Screen::ConfirmAbandon {
                        back: Box::new(Screen::Shop { shopper }),
                    },
                    ShopAction::Done => match self.next_shopper(shopper + 1) {
                        Some(next) => self.enter_shop(next),
                        None => self.battle(),
                    },
                }
            }

            Screen::ConfirmAbandon { back } => {
                if let Screen::Battle { battle, .. } = back.as_ref() {
                    battle.draw(&self.players, &self.profile.settings);
                }
                match confirm_abandon() {
                    None => Screen::ConfirmAbandon { back },
                    Some(true) => self.abandon(),
                    Some(false) => *back,
                }
            }

            Screen::Final => self.final_standings(),

            Screen::Exit => Screen::Exit,
        }
    }

    fn main_menu(&mut self) -> Screen {
        let cx = ui::WIDTH / 2.0;
        logo::title(cx, 172.0);
        logo::divider(cx, 268.0, 430.0);
        ui::text_centered(&t!("tagline"), cx, 226.0, 24.0, LIGHTGRAY);

        let mut y = 300.0;
        let mut item = |key: &str| {
            let r = Rect::new(cx - 180.0, y, 360.0, 52.0);
            y += 60.0;
            ui::button(r, &t!(key), true)
        };
        let next = if self.has_saved_run && item("menu-continue") {
            self.load_run()
        } else if item("menu-new-game") {
            Screen::Setup
        } else if item("menu-statistics") {
            Screen::Stats
        } else if item("menu-settings") {
            Screen::Settings
        } else if item("menu-about") {
            Screen::About
        } else if item("menu-quit") {
            Screen::Exit
        } else {
            Screen::Menu
        };

        self.crash_notice(cx);

        ui::text_centered(&t!("tribute"), cx, ui::HEIGHT - 14.0, 18.0, GRAY);
        let version = format!("v{}", crash::VERSION);
        let w = ui::measure(&version, 16.0).width;
        ui::text_at(
            &version,
            ui::WIDTH - 12.0 - w,
            ui::HEIGHT - 14.0,
            16.0,
            GRAY,
        );
        next
    }

    /// "The game crashed" (once or several times) with a button to copy the reports for sending.
    fn crash_notice(&mut self, cx: f32) {
        if let Some(note) = self.crash_note.clone() {
            ui::text_centered(&note, cx, 40.0, 18.0, SKYBLUE);
        }
        let Some(pending) = &self.crash else {
            return;
        };
        let area = Rect::new(cx - 420.0, 10.0, 840.0, 76.0);
        ui::panel(area);
        ui::text_fit(
            &t!("crash-last-time", n = pending.count()),
            area.x + 16.0,
            area.y + 28.0,
            area.w - 340.0,
            18.0,
            ORANGE,
        );
        ui::text_fit(
            &pending.location().display().to_string(),
            area.x + 16.0,
            area.y + 56.0,
            area.w - 340.0,
            16.0,
            LIGHTGRAY,
        );
        let copy = ui::button(
            Rect::new(area.x + area.w - 314.0, area.y + 19.0, 170.0, 38.0),
            &t!("crash-copy", n = pending.count()),
            true,
        );
        let dismiss = ui::button(
            Rect::new(area.x + area.w - 136.0, area.y + 19.0, 120.0, 38.0),
            &t!("crash-dismiss"),
            true,
        );
        if copy {
            miniquad::window::clipboard_set(&pending.text);
            self.crash_note = Some(t!("crash-copied", n = pending.count()));
        }
        if copy || dismiss {
            crash::mark_seen(pending);
            self.crash = None;
        }
    }

    fn final_standings(&mut self) -> Screen {
        let area = Rect::new(290.0, 110.0, 700.0, 500.0);
        ui::panel(area);
        ui::text_centered(
            &t!("final-title"),
            area.center().x,
            area.y + 60.0,
            42.0,
            GOLD,
        );
        let mut order: Vec<&Player> = self.players.iter().collect();
        order.sort_by_key(|p| std::cmp::Reverse((p.wins, p.kills)));
        let mut y = area.y + 130.0;
        for (rank, p) in order.iter().enumerate() {
            let name = format!("{}. {}", rank + 1, p.label());
            ui::text_fit(&name, area.x + 60.0, y, 300.0, 28.0, p.color);
            let score = format!(
                "{}   {}",
                t!("unit-wins", n = p.wins),
                t!("unit-kills", n = p.kills)
            );
            ui::text_fit(&score, area.x + 380.0, y, 280.0, 28.0, p.color);
            y += 44.0;
        }
        if let Some(note) = &self.ladder_note {
            ui::text_centered(note, area.center().x, area.y + 370.0, 24.0, SKYBLUE);
        }
        let half = Rect::new(area.x + 110.0, area.y + 410.0, 230.0, 56.0);
        if ui::button(half, &t!("final-play-again"), true) {
            // Same setup as the game just played.
            self.start_with(self.setup)
        } else if ui::button(
            Rect::new(half.x + 250.0, half.y, half.w, half.h),
            &t!("final-main-menu"),
            true,
        ) {
            Screen::Menu
        } else {
            Screen::Final
        }
    }
}

/// Pause overlay drawn over the frozen battle. Returns the chosen action, if any.
fn pause_menu(
    your_turn: bool,
    can_save: bool,
    setup: &Setup,
    progress: (u32, u32),
) -> Option<PauseAction> {
    draw_rectangle(
        0.0,
        0.0,
        ui::WIDTH,
        ui::HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.5),
    );
    let area = Rect::new(220.0, 60.0, 400.0, 600.0);
    ui::panel(area);
    ui::text_centered(
        &t!("pause-title"),
        area.center().x,
        area.y + 55.0,
        40.0,
        GOLD,
    );

    // What this game was set up with.
    let info = Rect::new(660.0, 60.0, 400.0, 600.0);
    ui::panel(info);
    ui::text_centered(
        &t!("pause-this-game"),
        info.center().x,
        info.y + 55.0,
        32.0,
        GOLD,
    );
    setup.draw_summary(
        Rect::new(info.x + 30.0, info.y + 100.0, info.w - 60.0, info.h),
        34.0,
        Some(progress),
    );

    let button = |i: f32| Rect::new(area.x + 40.0, area.y + 80.0 + i * 60.0, area.w - 80.0, 52.0);
    let action = if ui::button(button(0.0), &t!("pause-resume"), true)
        || is_key_pressed(KeyCode::Escape)
    {
        Some(PauseAction::Resume)
    } else if ui::button(button(1.0), &t!("pause-flee"), your_turn) {
        Some(PauseAction::Flee)
    } else if ui::button(button(2.0), &t!("pause-self-destruct"), your_turn) {
        Some(PauseAction::SelfDestruct)
    } else if ui::button(button(3.0), &t!("pause-battle-log"), true) || is_key_pressed(KeyCode::L) {
        Some(PauseAction::BattleLog)
    } else if ui::button(button(4.0), &t!("save-exit-menu"), true) {
        Some(PauseAction::SaveToMenu)
    } else if ui::button(button(5.0), &t!("pause-save-quit"), true) {
        Some(PauseAction::SaveAndQuit)
    } else if ui::button(button(6.0), &t!("abandon-game"), true) {
        Some(PauseAction::Abandon)
    } else {
        None
    };

    // Explain greyed-out buttons instead of hiding them.
    let note = if !your_turn {
        t!("pause-note-not-aiming")
    } else if can_save {
        t!("pause-note-actions")
    } else {
        t!("pause-note-mid-shot")
    };
    ui::text_wrapped(
        &note,
        area.x + 30.0,
        area.y + area.h - 52.0,
        area.w - 60.0,
        17.0,
        GRAY,
    );
    action
}

/// The command line value after `flag`, as in `--screen shop`.
fn arg_after(flag: &str) -> Option<String> {
    let args: Vec<String> = std::env::args().collect();
    let at = args.iter().position(|a| a == flag)?;
    args.get(at + 1).cloned()
}

impl Game {
    /// Developer aid: `scorch --screen <name> [--shot file.png]` opens one
    /// screen directly with a fresh game, to check layouts (for example in
    /// every language); `--shot` saves a screenshot and quits. Saving is off. Names: intro, menu, about, setup, custom, settings, stats, crash,
    /// battle, pause, over, log, shop, abandon, final.
    fn open_screen(&mut self, name: &str) -> Screen {
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
            "settings" => Screen::Settings,
            "stats" => Screen::Stats,
            "crash" => {
                let report = |t| {
                    std::path::PathBuf::from(format!("/home/you/.local/share/scorch/crash-{t}.log"))
                };
                self.crash = Some(crash::Pending {
                    paths: vec![report(1793470061), report(1793460000)],
                    text: String::new(),
                });
                Screen::Menu
            }
            "battle" => game_screen(self, false),
            "pause" => game_screen(self, true),
            // A bot just won the round: you flee, the bots finish it.
            "over" => match game_screen(self, false) {
                Screen::Battle { mut battle, .. } => {
                    battle.flee(&mut self.players);
                    battle.skip_to_end(&mut self.players);
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
            "final" => {
                game_screen(self, false);
                Screen::Final
            }
            _ => Screen::Menu,
        }
    }
}

const INTRO_TIME: f32 = 3.5;

/// First-run dedication: fades in and out, any key or click skips it.
fn intro(time: f32) -> Screen {
    // The first frames after start-up can take long; don't let them eat the intro.
    let time = time + get_frame_time().min(0.05);
    let skip = get_last_key_pressed().is_some() || is_mouse_button_pressed(MouseButton::Left);
    if time >= INTRO_TIME || (skip && time > 0.2) {
        return Screen::Menu;
    }
    let fade = (time / 0.6).min((INTRO_TIME - time) / 0.6).clamp(0.0, 1.0);
    dedication(ui::HEIGHT / 2.0 - 10.0, fade);
    Screen::Intro { time }
}

/// "In memory of ...", years, and why, centred on `y` (the years line).
fn dedication(y: f32, alpha: f32) {
    let cx = ui::WIDTH / 2.0;
    let color = |c: Color| Color { a: alpha, ..c };
    ui::text_centered(&t!("dedication-1"), cx, y - 38.0, 30.0, color(WHITE));
    ui::text_centered(&t!("dedication-years"), cx, y, 22.0, color(GRAY));
    ui::text_centered(&t!("dedication-2"), cx, y + 42.0, 24.0, color(LIGHTGRAY));
}

/// About: dedication, the original game, credits, version. Returns true on Back.
fn about() -> bool {
    let cx = ui::WIDTH / 2.0;
    let area = Rect::new(190.0, 50.0, 900.0, 560.0);
    ui::panel(area);
    ui::text_centered(&t!("about-title"), cx, area.y + 60.0, 42.0, GOLD);
    dedication(area.y + 180.0, 1.0);
    logo::divider(cx, area.y + 262.0, 430.0);
    let mut y = area.y + 320.0;
    for key in ["about-made-with", "tribute", "about-font"] {
        y = ui::text_wrapped(&t!(key), area.x + 60.0, y, area.w - 120.0, 20.0, LIGHTGRAY) + 14.0;
    }
    ui::text_centered(
        &format!("{} {}", t!("about-version"), crash::VERSION),
        cx,
        area.y + area.h - 30.0,
        18.0,
        GRAY,
    );
    ui::button(Rect::new(cx - 120.0, 630.0, 240.0, 52.0), &t!("back"), true)
        || is_key_pressed(KeyCode::Escape)
}

/// Confirmation dialog. `Some(true)` = abandon, `Some(false)` = cancel.
fn confirm_abandon() -> Option<bool> {
    draw_rectangle(
        0.0,
        0.0,
        ui::WIDTH,
        ui::HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.6),
    );
    let area = Rect::new(390.0, 220.0, 500.0, 260.0);
    ui::panel(area);
    ui::text_centered(
        &t!("abandon-title"),
        area.center().x,
        area.y + 55.0,
        36.0,
        RED,
    );
    ui::text_wrapped(
        &t!("abandon-text"),
        area.x + 40.0,
        area.y + 100.0,
        area.w - 80.0,
        22.0,
        WHITE,
    );
    let half = (area.w - 100.0) / 2.0;
    if ui::button(
        Rect::new(area.x + 40.0, area.y + 180.0, half, 52.0),
        &t!("abandon"),
        true,
    ) {
        Some(true)
    } else if ui::button(
        Rect::new(area.x + 60.0 + half, area.y + 180.0, half, 52.0),
        &t!("cancel"),
        true,
    ) || is_key_pressed(KeyCode::Escape)
    {
        Some(false)
    } else {
        None
    }
}

fn window_conf() -> Conf {
    Conf {
        window_title: "Scorch".to_string(),
        window_width: 1280,
        window_height: 720,
        window_resizable: true,
        ..Default::default()
    }
}

fn main() {
    if std::env::args().any(|a| a == "--version" || a == "-V") {
        println!("{}", crash::version_line());
        return;
    }
    println!("{}", crash::version_line());
    crash::install();
    if std::env::args().any(|a| a == "--crash-test") {
        panic!("crash test requested with --crash-test");
    }
    macroquad::Window::from_config(window_conf(), run());
}

async fn run() {
    macroquad::rand::srand((miniquad::date::now() * 1000.0) as u64);
    // Closing the window should save first, so handle it ourselves.
    prevent_quit();

    let mut profile = save::load_profile();
    // First run (or an old "System" setting): pick the OS language once and save it.
    if profile.settings.language.is_none() {
        profile.settings.language = Some(i18n::detect());
        save::store_profile(&profile);
    }
    let first_screen = if profile.intro_seen {
        Screen::Menu
    } else {
        Screen::Intro { time: 0.0 }
    };
    let mut game = Game {
        players: Vec::new(),
        setup: profile.setup,
        round: 1,
        profile,
        has_saved_run: save::run_exists(),
        log_open: false,
        crash: crash::pending(),
        crash_note: None,
        ladder_note: None,
        preview: Preview::default(),
        screen: first_screen,
    };
    if let Some(name) = arg_after("--screen") {
        save::set_read_only();
        game.screen = game.open_screen(&name);
    }
    // With `--shot <file.png>`, save a screenshot after a moment and quit.
    let shot = arg_after("--shot");
    let mut frame: u32 = 0;
    let mut fullscreen = false;
    loop {
        // Window managers (Sway at least) ignore fullscreen requests made too
        // early or while a mouse button is held, so apply the setting here,
        // once the window is up and the click is over.
        frame += 1;
        let wanted = game.profile.settings.fullscreen;
        if frame > 30 && wanted != fullscreen && !is_mouse_button_down(MouseButton::Left) {
            set_fullscreen(wanted);
            fullscreen = wanted;
        }
        i18n::set_language(game.profile.settings.language.unwrap_or(Language::English));
        clear_background(BLACK);
        set_camera(&ui::camera());
        Sky::Night.draw();

        // Arrow keys aim the tank during play, so menu keyboard focus is off then.
        let in_play = matches!(game.screen, Screen::Battle { paused: false, .. });
        ui::begin_frame(!in_play);

        // Take the screen out, run it, put the next one back.
        let screen = std::mem::replace(&mut game.screen, Screen::Menu);
        let before = std::mem::discriminant(&screen);
        let screen = game.step(screen);
        if std::mem::discriminant(&screen) != before {
            ui::reset_focus();
        }
        if is_quit_requested() {
            game.save_screen(&screen);
            break;
        }
        if matches!(screen, Screen::Exit) {
            break;
        }
        game.screen = screen;
        if let Some(path) = &shot
            && frame == 90
        {
            get_screen_data().export_png(path);
            break;
        }
        next_frame().await;
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
        names.extend(crate::weapons::WEAPONS.iter().map(|w| w.text()));
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
