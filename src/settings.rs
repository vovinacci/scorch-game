//! Player settings (display and controls), separate from per-game setup.

use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::choice::choice;
use crate::i18n::{Language, t};
use crate::ui;

choice!(HealthDisplay {
    Bar,
    BarAndNumber,
    Number,
});

choice!(GameSpeed {
    Normal,
    Fast,
    Faster,
});

choice!(BotSpeed { X2, X4, X8 });

impl GameSpeed {
    pub fn factor(self) -> f32 {
        match self {
            GameSpeed::Normal => 1.0,
            GameSpeed::Fast => 1.5,
            GameSpeed::Faster => 2.0,
        }
    }
}

impl BotSpeed {
    pub fn factor(self) -> f32 {
        match self {
            BotSpeed::X2 => 2.0,
            BotSpeed::X4 => 4.0,
            BotSpeed::X8 => 8.0,
        }
    }
}

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Settings {
    pub health_display: HealthDisplay,
    /// Show the faint path of your previous shot while aiming.
    pub shot_path: bool,
    pub fullscreen: bool,
    /// Recent hits and kills, top left during battle.
    pub battle_log: bool,
    /// `None` until the first run detects it from the OS.
    #[serde(deserialize_with = "crate::i18n::lenient")]
    pub language: Option<Language>,
    /// Speed of everything in battle: shells, bots, falling.
    pub game_speed: GameSpeed,
    /// Extra speed-up when only bots are left in a round.
    pub bot_speed: BotSpeed,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            health_display: HealthDisplay::Bar,
            shot_path: true,
            fullscreen: false,
            battle_log: true,
            language: None,
            game_speed: GameSpeed::Normal,
            bot_speed: BotSpeed::X4,
        }
    }
}

/// Translation keys: `controls-<name>-key` and `controls-<name>`.
pub const CONTROLS: [&str; 11] = [
    "aim",
    "power",
    "fine",
    "weapon",
    "fire",
    "log",
    "pause",
    "skip",
    "menu-move",
    "menu-change",
    "menu-choose",
];

/// Draws the settings screen. Returns `true` when the player goes back.
pub fn screen(settings: &mut Settings) -> bool {
    ui::text_centered(&t!("settings-title"), ui::WIDTH / 2.0, 70.0, 48.0, ORANGE);

    let left = Rect::new(80.0, 110.0, 520.0, 490.0);
    ui::panel(left);
    ui::text_at(
        t!("settings-display"),
        left.x + 24.0,
        left.y + 40.0,
        28.0,
        GOLD,
    );
    let row = |i: f32| Rect::new(left.x + 24.0, left.y + 60.0 + i * 50.0, left.w - 48.0, 42.0);
    // A globe and the label in both languages, so the row is easy to find
    // even after switching to a language you can't read.
    let mut language = settings.language.unwrap_or(Language::English);
    let r = row(0.0);
    ui::choice(
        r,
        &format!("    {}", t!("settings-language")),
        &mut language,
    );
    ui::globe(vec2(r.x + 24.0, r.y + r.h / 2.0), 9.0, LIGHTGRAY);
    settings.language = Some(language);
    ui::choice(
        row(1.0),
        &t!("settings-health"),
        &mut settings.health_display,
    );
    ui::toggle(row(2.0), &t!("settings-shot-path"), &mut settings.shot_path);
    ui::toggle(
        row(3.0),
        &t!("settings-battle-log"),
        &mut settings.battle_log,
    );
    // Fullscreen is applied by the main loop (see `run`).
    ui::toggle(
        row(4.0),
        &t!("settings-fullscreen"),
        &mut settings.fullscreen,
    );
    ui::text_at(
        t!("settings-gameplay"),
        left.x + 24.0,
        left.y + 60.0 + 5.0 * 50.0 + 30.0,
        28.0,
        GOLD,
    );
    ui::choice(
        row(6.0),
        &t!("settings-game-speed"),
        &mut settings.game_speed,
    );
    ui::choice(row(7.0), &t!("settings-bot-speed"), &mut settings.bot_speed);

    let right = Rect::new(680.0, 110.0, 520.0, 490.0);
    ui::panel(right);
    ui::text_at(
        t!("settings-controls"),
        right.x + 24.0,
        right.y + 40.0,
        28.0,
        GOLD,
    );
    let mut y = right.y + 84.0;
    for name in CONTROLS {
        let key = t!(&format!("controls-{name}-key"));
        ui::text_fit(&key, right.x + 24.0, y, 190.0, 20.0, SKYBLUE);
        let action = t!(&format!("controls-{name}"));
        ui::text_fit(&action, right.x + 230.0, y, right.w - 254.0, 20.0, WHITE);
        y += 36.0;
    }

    ui::button(
        Rect::new(ui::WIDTH / 2.0 - 120.0, 620.0, 240.0, 52.0),
        &t!("back"),
        true,
    ) || is_key_pressed(KeyCode::Escape)
}
