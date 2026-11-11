//! The screens around the battle: main menu and its notices, pause menu, final standings, intro, about, and the
//! confirmation dialog.

use macroquad::prelude::*;

use super::{Game, PauseAction, Screen};
use crate::i18n::t;
use crate::player::Player;
use crate::setup::Setup;
use crate::{crash, logo, save, ui, update, world};

impl Game {
    pub(super) fn main_menu(&mut self) -> Screen {
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
            if self.has_saved_run {
                Screen::ConfirmNewGame
            } else {
                Screen::Setup
            }
        } else if item("menu-statistics") {
            Screen::Stats
        } else if item("menu-settings") {
            Screen::Settings {
                back: Box::new(Screen::Menu),
            }
        } else if item("menu-about") {
            Screen::About
        } else if item("menu-quit") {
            Screen::Exit
        } else {
            Screen::Menu
        };

        // One notice at the top at a time: a lost save first, then crash reports.
        let y = if self.save_notice(cx) {
            116.0
        } else if self.crash_notice(cx) {
            96.0
        } else {
            10.0
        };
        self.update_notice(cx, y);

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
    /// A save file couldn't be read: what was lost and where the old file is kept. True while it shows.
    pub(super) fn save_notice(&mut self, cx: f32) -> bool {
        let Some(problem) = &self.save_problem else {
            return false;
        };
        let text = match (problem.file, problem.problem) {
            (save::File::Profile, save::Problem::Damaged) => t!("save-profile-damaged"),
            (save::File::Profile, save::Problem::Newer) => t!("save-profile-newer"),
            (save::File::Run, save::Problem::Damaged) => t!("save-run-damaged"),
            (save::File::Run, save::Problem::Newer) => t!("save-run-newer"),
        };
        let kept = match &problem.kept {
            Some(path) => t!("save-kept", path = path.display().to_string()),
            None => String::new(),
        };
        // Taller than the crash notice: the path gets a line of its own, the whole width.
        let area = Rect::new(cx - 420.0, 10.0, 840.0, 96.0);
        ui::panel(area);
        let (x, w) = (area.x + 16.0, area.w - 32.0);
        ui::text_fit(&text, x, area.y + 34.0, w - 136.0, 18.0, ORANGE);
        ui::text_fit(&kept, x, area.y + 80.0, w, 16.0, LIGHTGRAY);
        if ui::button(
            Rect::new(area.x + area.w - 136.0, area.y + 19.0, 120.0, 38.0),
            &t!("crash-dismiss"),
            true,
        ) {
            self.save_problem = None;
        }
        true
    }

    /// Crash reports from earlier runs, to copy and send. True while the notice shows.
    pub(super) fn crash_notice(&mut self, cx: f32) -> bool {
        if let Some(note) = self.crash_note.clone() {
            ui::text_centered(&note, cx, 40.0, 18.0, SKYBLUE);
        }
        let Some(pending) = &self.crash else {
            return false;
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
        true
    }

    /// The update question (first time), an available update, its progress, or a failed install.
    pub(super) fn update_notice(&mut self, cx: f32, y: f32) {
        if !update::enabled() {
            return;
        }
        let ask = self.profile.settings.check_updates.is_none();
        let (text, buttons): (String, Vec<String>) = if ask {
            (t!("update-ask"), vec![t!("update-yes"), t!("update-no")])
        } else {
            match self.updater.status() {
                update::Status::Available { version } => (
                    t!("update-available", version = version),
                    vec![t!("update-install"), t!("update-later")],
                ),
                update::Status::Downloading { version } => {
                    (t!("update-downloading", version = version), vec![])
                }
                update::Status::Installed { version } => (
                    t!("update-installed", version = version),
                    vec![t!("update-restart")],
                ),
                update::Status::InstallFailed(error) => (
                    t!("update-failed", error = error),
                    vec![t!("crash-dismiss")],
                ),
                _ => return,
            }
        };
        let area = Rect::new(cx - 420.0, y, 840.0, 76.0);
        ui::panel(area);
        let text_w = area.w - 32.0 - 154.0 * buttons.len() as f32;
        // One line sits in the middle of the panel; two lines start higher.
        let one_line = ui::measure(&text, 18.0).width <= text_w;
        let text_y = area.y + if one_line { 44.0 } else { 30.0 };
        ui::text_wrapped(&text, area.x + 16.0, text_y, text_w, 18.0, SKYBLUE);
        let mut pressed = None;
        for (i, label) in buttons.iter().enumerate().rev() {
            let x = area.x + area.w - 16.0 - 146.0 * (buttons.len() - i) as f32 + 8.0;
            if ui::button(Rect::new(x, area.y + 19.0, 138.0, 38.0), label, true) {
                pressed = Some(i);
            }
        }
        let Some(pressed) = pressed else {
            return;
        };
        if ask {
            let yes = pressed == 0;
            self.profile.settings.check_updates = Some(yes);
            save::store_profile(&self.profile);
            if yes {
                self.updater.check();
            }
            return;
        }
        match self.updater.status() {
            update::Status::Available { .. } if pressed == 0 => self.updater.install(),
            update::Status::Installed { .. } => update::restart(),
            _ => self.updater.dismiss(),
        }
    }

    pub(super) fn final_standings(&mut self) -> Screen {
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
pub(super) fn pause_menu(
    your_turn: bool,
    can_save: bool,
    setup: &Setup,
    progress: (u32, u32),
    round: (world::World, world::World),
) -> Option<PauseAction> {
    ui::dim();
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
        32.0,
        Some(progress),
        Some(round),
    );

    let button = |i: f32| Rect::new(area.x + 40.0, area.y + 80.0 + i * 55.0, area.w - 80.0, 48.0);
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
    } else if ui::button(button(4.0), &t!("menu-settings"), true) {
        Some(PauseAction::Settings)
    } else if ui::button(button(5.0), &t!("save-exit-menu"), true) {
        Some(PauseAction::SaveToMenu)
    } else if ui::button(button(6.0), &t!("pause-save-quit"), true) {
        Some(PauseAction::SaveAndQuit)
    } else if ui::button(button(7.0), &t!("abandon-game"), true) {
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

const INTRO_TIME: f32 = 3.5;

/// First-run dedication: fades in and out, any key or click skips it.
pub(super) fn intro(time: f32) -> Screen {
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
pub(super) fn dedication(y: f32, alpha: f32) {
    let cx = ui::WIDTH / 2.0;
    let color = |c: Color| Color { a: alpha, ..c };
    ui::text_centered(&t!("dedication-1"), cx, y - 38.0, 30.0, color(WHITE));
    ui::text_centered(&t!("dedication-years"), cx, y, 22.0, color(GRAY));
    ui::text_centered(&t!("dedication-2"), cx, y + 42.0, 24.0, color(LIGHTGRAY));
}

/// About: dedication, the original game, credits, version and a manual update check. Returns true on Back.
pub(super) fn about(updater: &update::Updater) -> bool {
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
    if update::enabled() {
        let check = Rect::new(cx - 160.0, area.y + area.h - 116.0, 320.0, 44.0);
        if ui::button(check, &t!("about-check-updates"), true) {
            updater.check();
        }
        let status = match updater.status() {
            update::Status::Checking => t!("update-checking"),
            update::Status::UpToDate => t!("update-up-to-date"),
            update::Status::CheckFailed(_) => t!("update-check-failed"),
            update::Status::Available { version } => t!("update-available", version = version),
            _ => String::new(),
        };
        ui::text_centered(&status, cx, area.y + area.h - 52.0, 18.0, SKYBLUE);
    }
    ui::button(Rect::new(cx - 120.0, 630.0, 240.0, 52.0), &t!("back"), true)
        || is_key_pressed(KeyCode::Escape)
}

/// Confirmation dialog with a `title`, a `text` and a `yes` button (translation keys).
/// `Some(true)` = yes, `Some(false)` = cancel, `None` = not decided yet.
pub(super) fn confirm(title: &str, text: &str, yes: &str) -> Option<bool> {
    draw_rectangle(
        0.0,
        0.0,
        ui::WIDTH,
        ui::HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.6),
    );
    let area = Rect::new(390.0, 220.0, 500.0, 260.0);
    ui::panel(area);
    ui::text_centered(&t!(title), area.center().x, area.y + 55.0, 36.0, RED);
    ui::text_wrapped(
        &t!(text),
        area.x + 40.0,
        area.y + 100.0,
        area.w - 80.0,
        22.0,
        WHITE,
    );
    let half = (area.w - 100.0) / 2.0;
    if ui::button(
        Rect::new(area.x + 40.0, area.y + 180.0, half, 52.0),
        &t!(yes),
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
