//! A player persists across rounds: money, ammo and score. Their tank does not.

use macroquad::prelude::Color;
use serde::{Deserialize, Serialize};

use crate::ai::{self, AiProfile};
use crate::economy::Income;
use crate::i18n::t;
use crate::save::ColorDef;
use crate::stats::RoundStats;
use crate::weapons::WEAPON_COUNT;

#[derive(Serialize, Deserialize)]
pub struct Player {
    pub name: String,
    #[serde(with = "ColorDef")]
    pub color: Color,
    /// `Some` for bots: how well they aim.
    pub bot: Option<AiProfile>,
    pub money: i32,
    pub ammo: [u32; WEAPON_COUNT],
    pub wins: u32,
    pub kills: u32,
    pub loss_streak: u32,
    /// Earned in the current (or just finished) round.
    pub income: Income,
    /// What happened in the current round, for all-time stats.
    #[serde(default)]
    pub round_stats: RoundStats,
}

impl Player {
    pub fn new(name: &str, color: Color, bot: Option<AiProfile>) -> Self {
        let mut ammo = [0; WEAPON_COUNT];
        ammo[1] = 2; // a couple of Missiles to start with
        Self {
            name: name.to_string(),
            color,
            bot,
            money: 100,
            ammo,
            wins: 0,
            kills: 0,
            loss_streak: 0,
            income: Income::default(),
            round_stats: RoundStats::default(),
        }
    }

    /// Name on screen, in the current language. `name` itself stays English
    /// ("You", "Player 2", "Alpha [Moron]") so saves don't depend on the language.
    pub fn label(&self) -> String {
        if self.is_you() {
            return t!("player-you");
        }
        if let Some(n) = self.name.strip_prefix("Player ") {
            return t!("player-n", n = n);
        }
        match self.name.split_once(" [") {
            Some((bot, skill)) => {
                let skill = skill.trim_end_matches(']');
                format!(
                    "{} [{}]",
                    t!(&format!("bot-{}", bot.to_lowercase())),
                    ai::skill_text(skill)
                )
            }
            None => self.name.clone(),
        }
    }

    /// The only human in a solo game is called "You", which needs its own grammar.
    pub fn is_you(&self) -> bool {
        self.name == "You"
    }

    pub fn is_bot(&self) -> bool {
        self.bot.is_some()
    }

    pub fn has(&self, weapon: usize) -> bool {
        weapon == 0 || self.ammo[weapon] > 0
    }

    pub fn ammo_label(&self, weapon: usize) -> String {
        if weapon == 0 {
            t!("ammo-unlimited")
        } else {
            self.ammo[weapon].to_string()
        }
    }
}
