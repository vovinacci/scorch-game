//! The "New game" setup screen: players, opponents, match rules and world.
//! The last setup is remembered in the profile and copied into each game.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use serde::{Deserialize, Serialize};

use crate::ai::{self, AiProfile, CUSTOM, DIFFICULTIES};
use crate::choice::{Choice, choice};
use crate::i18n::t;
use crate::terrain::{HEIGHT, Terrain, WIDTH};
use crate::ui;
use crate::world::{TerrainKind, World};

choice!(SuddenDeath {
    Off,
    After15,
    After25,
    After40,
});

impl SuddenDeath {
    pub fn turn(self) -> Option<u32> {
        match self {
            SuddenDeath::Off => None,
            SuddenDeath::After15 => Some(15),
            SuddenDeath::After25 => Some(25),
            SuddenDeath::After40 => Some(40),
        }
    }
}

const ROUND_CHOICES: [u32; 4] = [3, 5, 7, 10];
/// Four tank colors, so at most four players.
pub const MAX_PLAYERS: usize = 4;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct Setup {
    pub humans: usize,
    pub bots: usize,
    /// Index into `ai::DIFFICULTIES`.
    pub difficulty: usize,
    /// Difficulty steps up after a won game, down after a lost one.
    pub ladder: bool,
    pub rounds: u32,
    /// Max shot power = health x 10, like the original game.
    pub power_limit: bool,
    pub sudden_death: SuddenDeath,
    pub world: World,
}

impl Default for Setup {
    fn default() -> Self {
        Self {
            humans: 1,
            bots: 2,
            difficulty: 1,
            ladder: false,
            rounds: 5,
            power_limit: true,
            sudden_death: SuddenDeath::After25,
            world: World::default(),
        }
    }
}

/// The parts of the setup a battle needs; saved with the battle.
#[derive(Clone, Copy, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct Rules {
    pub world: World,
    pub power_limit: bool,
    pub sudden_death: Option<u32>,
}

impl Setup {
    pub fn rules(&self) -> Rules {
        Rules {
            world: self.world,
            power_limit: self.power_limit,
            sudden_death: self.sudden_death.turn(),
        }
    }

    /// Keeps values valid after loading an old or edited profile.
    pub fn sanitize(&mut self) {
        self.humans = self.humans.clamp(1, 2);
        self.bots = self.bots.clamp(1, MAX_PLAYERS - self.humans);
        self.difficulty = self.difficulty.min(CUSTOM);
        if !ROUND_CHOICES.contains(&self.rounds) {
            self.rounds = 5;
        }
    }
}

pub enum SetupAction {
    Stay,
    Randomize,
    Back,
    Start,
    EditCustom,
}

/// Small terrain sample shown under the World settings; regenerated when the
/// terrain type changes.
#[derive(Default)]
pub struct Preview {
    kind: Option<TerrainKind>,
    terrain: Option<Terrain>,
}

impl Setup {
    /// Every setting as (label key, value), for the pause menu. With
    /// `progress` (round, turn) the rounds line shows where the game is.
    pub fn summary(&self, progress: Option<(u32, u32)>) -> Vec<(&'static str, String)> {
        let on = |b: bool| if b { t!("on") } else { t!("off") };
        let w = &self.world;
        let wind = if w.wind_changes {
            t!("summary-wind-changes", wind = w.wind.text())
        } else {
            w.wind.text()
        };
        let mut lines = vec![
            (
                "setup-players",
                t!("summary-players", humans = self.humans, bots = self.bots),
            ),
            ("setup-difficulty", ai::difficulty_text(self.difficulty)),
            ("setup-ladder", on(self.ladder)),
        ];
        match progress {
            Some((round, turn)) => {
                lines.push((
                    "summary-round",
                    t!("summary-round-of", round = round, rounds = self.rounds),
                ));
                lines.push(("summary-turn", turn.to_string()));
            }
            None => lines.push(("setup-rounds", self.rounds.to_string())),
        }
        lines.extend([
            ("setup-power-limit", on(self.power_limit)),
            ("setup-sudden-death", self.sudden_death.text()),
            ("setup-terrain", w.terrain.text()),
            ("setup-wind", wind),
            ("setup-gravity", w.gravity.text()),
            ("setup-walls", w.walls.text()),
            ("setup-sky", w.sky.text()),
        ]);
        lines
    }

    /// Draws `summary()` as a label/value list starting at the top of `r`.
    pub fn draw_summary(&self, r: Rect, step: f32, progress: Option<(u32, u32)>) {
        let mut y = r.y;
        for (key, value) in self.summary(progress) {
            ui::text_fit(&t!(key), r.x, y, r.w * 0.55, 20.0, LIGHTGRAY);
            let w = ui::measure(&value, 20.0).width.min(r.w * 0.45);
            ui::text_fit(&value, r.x + r.w - w, y, r.w * 0.45, 20.0, WHITE);
            y += step;
        }
    }
}

fn pick<T: Choice>() -> T {
    T::ALL[gen_range(0, T::ALL.len())]
}

impl Setup {
    /// Random match and world. Keeps the number of human players.
    pub fn randomize(&mut self) {
        self.bots = gen_range(1, MAX_PLAYERS - self.humans + 1);
        self.difficulty = gen_range(0, CUSTOM);
        self.rounds = ROUND_CHOICES[gen_range(0, ROUND_CHOICES.len())];
        self.power_limit = gen_range(0, 2) == 1;
        self.sudden_death = pick();
        self.world = World {
            terrain: pick(),
            wind: pick(),
            wind_changes: gen_range(0, 2) == 1,
            gravity: pick(),
            walls: pick(),
            sky: pick(),
        };
    }
}

pub fn screen(setup: &mut Setup, custom: &AiProfile, preview: &mut Preview) -> SetupAction {
    ui::text_centered(&t!("setup-title"), ui::WIDTH / 2.0, 62.0, 48.0, ORANGE);

    // Left: who plays and how.
    let left = Rect::new(50.0, 90.0, 570.0, 520.0);
    ui::panel(left);
    ui::text_at(t!("setup-match"), left.x + 24.0, left.y + 36.0, 26.0, GOLD);
    let row = |area: Rect, i: f32| {
        Rect::new(area.x + 20.0, area.y + 52.0 + i * 52.0, area.w - 40.0, 44.0)
    };

    let mut humans = setup.humans - 1;
    if ui::selector(
        row(left, 0.0),
        &t!("setup-players"),
        &["1".to_string(), t!("setup-hotseat")],
        &mut humans,
    ) {
        setup.humans = humans + 1;
        setup.bots = setup.bots.min(MAX_PLAYERS - setup.humans);
    }
    let bot_labels = ["1", "2", "3"];
    let mut bots = setup.bots - 1;
    let max_bots = MAX_PLAYERS - setup.humans;
    if ui::selector(
        row(left, 1.0),
        &t!("setup-bots"),
        &bot_labels[..max_bots],
        &mut bots,
    ) {
        setup.bots = bots + 1;
    }
    ui::selector(
        row(left, 2.0),
        &t!("setup-difficulty"),
        &(0..DIFFICULTIES.len())
            .map(ai::difficulty_text)
            .collect::<Vec<_>>(),
        &mut setup.difficulty,
    );
    ui::text_wrapped(
        &ai::describe(setup.difficulty, custom),
        left.x + 30.0,
        left.y + 52.0 + 3.0 * 52.0 + 14.0,
        left.w - 60.0,
        16.0,
        GRAY,
    );
    let edit = ui::button(row(left, 4.0), &t!("setup-edit-custom"), true);
    ui::toggle(row(left, 5.0), &t!("setup-ladder"), &mut setup.ladder);
    let round_labels: Vec<String> = ROUND_CHOICES.iter().map(u32::to_string).collect();
    let mut rounds = ROUND_CHOICES
        .iter()
        .position(|&r| r == setup.rounds)
        .unwrap_or(1);
    if ui::selector(
        row(left, 6.0),
        &t!("setup-rounds"),
        &round_labels,
        &mut rounds,
    ) {
        setup.rounds = ROUND_CHOICES[rounds];
    }
    ui::toggle(
        row(left, 7.0),
        &t!("setup-power-limit"),
        &mut setup.power_limit,
    );
    ui::choice(
        row(left, 8.0),
        &t!("setup-sudden-death"),
        &mut setup.sudden_death,
    );

    // Right: the battlefield.
    let right = Rect::new(660.0, 90.0, 570.0, 520.0);
    ui::panel(right);
    ui::text_at(
        t!("setup-world"),
        right.x + 24.0,
        right.y + 36.0,
        26.0,
        GOLD,
    );
    let world = &mut setup.world;
    ui::choice(row(right, 0.0), &t!("setup-terrain"), &mut world.terrain);
    ui::choice(row(right, 1.0), &t!("setup-wind"), &mut world.wind);
    ui::toggle(
        row(right, 2.0),
        &t!("setup-wind-changes"),
        &mut world.wind_changes,
    );
    ui::choice(row(right, 3.0), &t!("setup-gravity"), &mut world.gravity);
    ui::choice(row(right, 4.0), &t!("setup-walls"), &mut world.walls);
    ui::choice(row(right, 5.0), &t!("setup-sky"), &mut world.sky);
    draw_preview(
        preview,
        world,
        Rect::new(
            right.x + 20.0,
            right.y + 52.0 + 6.0 * 52.0 + 8.0,
            right.w - 40.0,
            140.0,
        ),
    );

    let back = ui::button(Rect::new(50.0, 640.0, 220.0, 52.0), &t!("back"), true)
        || is_key_pressed(KeyCode::Escape);
    let random = ui::button(
        Rect::new(530.0, 640.0, 220.0, 52.0),
        &t!("setup-lucky"),
        true,
    );
    let start = ui::button(
        Rect::new(1010.0, 640.0, 220.0, 52.0),
        &t!("setup-start"),
        true,
    );
    if start {
        SetupAction::Start
    } else if random {
        SetupAction::Randomize
    } else if edit {
        SetupAction::EditCustom
    } else if back {
        SetupAction::Back
    } else {
        SetupAction::Stay
    }
}

fn draw_preview(preview: &mut Preview, world: &World, r: Rect) {
    if preview.kind != Some(world.terrain) {
        preview.kind = Some(world.terrain);
        preview.terrain = Some(Terrain::generate(world.terrain));
    }
    let Some(terrain) = &preview.terrain else {
        return;
    };
    // Sky and ground, squeezed into the preview box.
    let (sx, sy) = (r.w / WIDTH as f32, r.h / HEIGHT);
    world.sky.draw_in(r);
    let step = 4.0;
    let mut x = 0.0;
    while x < WIDTH as f32 {
        let g = terrain.height_at(x);
        draw_rectangle(
            r.x + x * sx,
            r.y + g * sy,
            step * sx + 0.5,
            (HEIGHT - g) * sy,
            Color::from_rgba(120, 85, 50, 255),
        );
        x += step;
    }
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 1.0, GRAY);
    ui::text_at(
        t!("setup-preview-walls", walls = world.walls.text()),
        r.x + 8.0,
        r.y + 20.0,
        16.0,
        WHITE,
    );
}
