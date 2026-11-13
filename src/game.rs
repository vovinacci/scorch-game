//! The game around the battles: the screens and how one leads to the next, starting and ending games and rounds,
//! saving, and the main loop.

use macroquad::prelude::*;

use crate::ai::CUSTOM;
use crate::battle::Battle;
use crate::choice::Choice;
use crate::economy::Income;
use crate::i18n::Language;
use crate::i18n::t;
use crate::ladder::{Ladder, Move, Outcome};
use crate::player::Player;
use crate::save::Profile;
use crate::save::RunOut;
use crate::save::StageIn;
use crate::save::StageOut;
use crate::setup::Preview;
use crate::setup::Setup;
use crate::setup::SetupAction;
use crate::shop::ShopAction;
use crate::stats::RoundStats;
use crate::world::Sky;
use crate::{
    ai, arg_after, battle, crash, economy, i18n, logo, rng, save, settings, setup, shop, sound,
    stats, ui, update,
};

mod dev;
mod menus;

use dev::{game_area, menu_key};
use menus::{about, confirm, intro, pause_menu};

const COLORS: [Color; 4] = [
    Color::new(0.95, 0.35, 0.3, 1.0),
    Color::new(0.35, 0.6, 1.0, 1.0),
    Color::new(0.4, 0.9, 0.4, 1.0),
    Color::new(0.95, 0.85, 0.3, 1.0),
];

const BOT_NAMES: [&str; 3] = ["Alpha", "Bravo", "Charlie"];

#[expect(
    clippy::large_enum_variant,
    reason = "one Screen exists at a time, so the big Battle variant costs nothing"
)]
enum Screen {
    /// The dedication, shown briefly at start-up.
    Intro {
        time: f32,
    },
    Menu,
    About,
    Setup,
    CustomBot,
    /// `back` is where Back returns to: the main menu, or the paused battle.
    Settings {
        back: Box<Screen>,
    },
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
    /// "Your saved game will be abandoned" before New game, when there is one.
    ConfirmNewGame,
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
    Settings,
}

struct Game {
    players: Vec<Player>,
    /// Setup of the game in progress (a copy of the profile's at the start).
    setup: Setup,
    round: u32,
    /// Settings, last setup and all-time stats, saved to disk.
    profile: Profile,
    has_saved_run: bool,
    /// Shown on the final screen after a ladder game: what it did to the player's rung.
    ladder_note: Option<String>,
    preview: Preview,
    /// Crash report from a previous run, offered on the main menu.
    crash: Option<crash::Pending>,
    /// A save file that couldn't be read (and was moved aside), told on the main menu.
    save_problem: Option<save::Unreadable>,
    /// Shown on the main menu after the crash report was copied.
    crash_note: Option<String>,
    /// Full battle log opened from the pause menu.
    log_open: bool,
    updater: update::Updater,
    /// Loaded once and never dropped (the audio thread outlives the game loop).
    sounds: &'static sound::Sounds,
    screen: Screen,
}

impl Game {
    fn start(&mut self) -> Screen {
        self.start_with(self.profile.setup)
    }

    fn start_with(&mut self, setup: Setup) -> Screen {
        // A new game replaces the saved one: that one counts as abandoned (and lost, if it was a ladder game).
        if self.has_saved_run {
            if let Ok(run) = save::load_run() {
                self.setup = run.setup;
            }
            self.abandon();
        }
        self.setup = setup;
        // On the ladder the rung sets the opponents.
        let lineup = if self.setup.on_ladder() {
            self.profile.ladder.opponents()
        } else {
            ai::lineup(self.setup.difficulty, self.setup.bots, self.profile.custom)
        };
        self.setup.bots = lineup.len();
        let humans = self.setup.humans;
        self.players.clear();
        for (i, &color) in COLORS.iter().enumerate().take(humans) {
            let name = if humans == 1 {
                "You".to_string()
            } else {
                format!("Player {}", i + 1)
            };
            self.players.push(Player::new(&name, color, None));
        }
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
        let order = setup::turn_order(self.setup.turn_order, &self.players, self.round);
        let mut battle = Battle::new(
            &self.players,
            order,
            self.round,
            self.setup.rounds,
            self.setup.rules(),
        );
        battle.raise_shields(&mut self.players);
        Screen::Battle {
            battle,
            paused: false,
        }
    }

    fn save_run(&mut self, stage: StageOut<'_>) {
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
            Screen::ConfirmAbandon { back } | Screen::Settings { back } => self.save_screen(back),
            _ => {}
        }
    }

    fn load_run(&mut self) -> Screen {
        let run = match save::load_run() {
            Ok(run) => run,
            Err(problem) => {
                self.has_saved_run = false;
                self.save_problem = problem.or(self.save_problem.take());
                return Screen::Menu;
            }
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
        // Walking away from a ladder game is a loss.
        if self.setup.counts_for_ladder() {
            self.profile.ladder.record(Outcome::Lost);
        }
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

    /// Records a finished ladder game and says what it did to the player's rung, for the final screen. `None` when
    /// the game wasn't on the ladder.
    fn ladder_result(&mut self) -> Option<String> {
        if !self.setup.on_ladder() {
            return None;
        }
        if !self.setup.counts_for_ladder() {
            return Some(t!("ladder-not-counted"));
        }
        let wins = |bot: bool| {
            let side = self.players.iter().filter(|p| p.is_bot() == bot);
            side.map(|p| p.wins).max().unwrap_or(0)
        };
        let outcome = Outcome::of(wins(false), wins(true));
        let ladder = &mut self.profile.ladder;
        let step = ladder.record(outcome);
        let rung = ladder.rung();
        Some(match step {
            Move::Up => t!("ladder-up", rung = rung, of = Ladder::RUNGS),
            Move::Champion => t!("ladder-champion"),
            Move::Stay => t!("ladder-shared", rung = rung),
            Move::Lost { left: 1 } => t!("ladder-lost-warned", rung = rung),
            Move::Lost { .. } => t!("ladder-lost", rung = rung),
            Move::Down => t!("ladder-down", rung = rung, of = Ladder::RUNGS),
        })
    }

    fn end_game(&mut self) {
        let best = self.players.iter().map(|p| p.wins).max().unwrap_or(0);
        for p in self.players.iter().filter(|p| !p.is_bot()) {
            self.profile.stats.add_game(p.wins == best, p.wins);
        }
        self.ladder_note = self.ladder_result();
        save::store_profile(&self.profile);
        save::remove_run();
        self.has_saved_run = false;
    }

    #[expect(
        clippy::too_many_lines,
        reason = "the screen state machine: one arm per screen"
    )]
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
                if about(&self.updater) {
                    Screen::Menu
                } else {
                    Screen::About
                }
            }

            Screen::Setup => {
                match setup::screen(
                    &mut self.profile.setup,
                    &self.profile.custom,
                    &self.profile.ladder,
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

            Screen::Settings { back } => {
                // Over a paused battle, keep the battle in view behind the panels.
                if let Screen::Battle { battle, .. } = back.as_ref() {
                    battle.draw_backdrop(&self.players, &self.profile.settings);
                    ui::dim();
                }
                if settings::screen(&mut self.profile.settings) {
                    save::store_profile(&self.profile);
                    *back
                } else {
                    Screen::Settings { back }
                }
            }

            Screen::Stats => {
                if stats::screen(&self.profile.stats, &self.profile.ladder) {
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
                match pause_menu(
                    your_turn,
                    battle.can_save(),
                    &self.setup,
                    progress,
                    battle.worlds(),
                ) {
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
                    Some(PauseAction::Settings) => Screen::Settings {
                        back: Box::new(Screen::Battle {
                            battle,
                            paused: true,
                        }),
                    },
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
                let volume = self.profile.settings.sound.level();
                for sfx in battle.take_sounds() {
                    self.sounds.play(sfx, volume);
                }
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
                        // Not hundreds of explosions at once.
                        battle.take_sounds();
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
                let shoots_first =
                    setup::sure_to_start(self.setup.turn_order, &self.players, shopper);
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
                match confirm("abandon-title", "abandon-text", "abandon") {
                    None => Screen::ConfirmAbandon { back },
                    Some(true) => self.abandon(),
                    Some(false) => *back,
                }
            }

            // The save goes when the new game starts (`start_with`), so backing out of the setup keeps it.
            Screen::ConfirmNewGame => {
                // Only the title behind the dialog: the menu's buttons must not take clicks meant for it.
                logo::title(ui::WIDTH / 2.0, 172.0);
                match confirm("new-game-title", "new-game-text", "menu-new-game") {
                    None => Screen::ConfirmNewGame,
                    Some(true) => Screen::Setup,
                    Some(false) => Screen::Menu,
                }
            }

            Screen::Final => self.final_standings(),

            Screen::Exit => Screen::Exit,
        }
    }
}

pub async fn run() {
    rng::seed((miniquad::date::now() * 1000.0) as u64);
    // Closing the window should save first, so handle it ourselves.
    prevent_quit();

    // `--screen` is a developer aid: nothing may be written, from the very start.
    let screen = arg_after("--screen");
    if screen.is_some() {
        save::set_read_only();
    }
    let (mut profile, save_problem) = save::load_profile();
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
        updater: update::Updater::new(),
        sounds: Box::leak(Box::new(sound::Sounds::load().await)),
        crash: crash::pending(),
        crash_note: None,
        save_problem,
        ladder_note: None,
        preview: Preview::default(),
        screen: first_screen,
    };
    if let Some(name) = screen {
        // A screen may make its texts while it opens: they must be in the player's language already.
        i18n::set_language(game.profile.settings.language.unwrap_or(Language::English));
        game.screen = game.open_screen(&name);
    }
    // With `--shot <file.png>`, save a screenshot after a moment and quit.
    let shot = arg_after("--shot");
    if shot.is_some() {
        ui::ignore_pointer();
    }
    if game.profile.settings.check_updates == Some(true) {
        game.updater.check();
    }
    // With `--keys down,right,enter`, press those menu keys one after another before the screenshot.
    let keys: Vec<KeyCode> = arg_after("--keys")
        .map(|keys| keys.split(',').filter_map(menu_key).collect())
        .unwrap_or_default();
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
        // One scripted key every 4 frames from frame 20 on: the screen is up by then and each key gets its own frame.
        let scripted = (frame >= 20 && frame.is_multiple_of(4))
            .then(|| keys.get((frame as usize - 20) / 4).copied())
            .flatten();
        ui::script_key(scripted);
        ui::begin_frame(!in_play);

        game.sounds
            .keep_awake(game.profile.settings.sound.level() > 0.0);

        // Take the screen out, run it, put the next one back.
        let screen = std::mem::replace(&mut game.screen, Screen::Menu);
        let before = std::mem::discriminant(&screen);
        let screen = game.step(screen);
        if ui::take_click() {
            game.sounds
                .play(sound::Sfx::Click, game.profile.settings.sound.level());
        }
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
            game_area(&get_screen_data()).export_png(path);
            break;
        }
        next_frame().await;
    }
}
