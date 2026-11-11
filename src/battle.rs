//! One round of battle: tanks take turns firing until one is left.
//!
//! Turn phases:
//!   Aim    - current tank adjusts angle/power (keyboard or AI), then fires
//!   Flight - projectiles fly, explode, carve terrain
//!   Settle - tanks fall into craters, dead tanks explode (chain reactions!),
//!            sudden death drains health
//!   Over   - round result shown briefly

use crate::rng::gen_range;
use macroquad::prelude::*;
use serde::{Deserialize, Serialize};

use crate::choice::Choice;
use crate::economy;
use crate::i18n::t;
use crate::items::{ITEMS, PARACHUTE, Use, shield_to_raise};
use crate::player::Player;
use crate::save::Vec2Def;
use crate::settings::Settings;
use crate::setup::Rules;
use crate::sound::Sfx;
use crate::terrain::{HEIGHT, Terrain, WIDTH};
use crate::ui;
use crate::weapons::{Kind, WEAPONS};
use crate::world::World;

mod bots;
mod coach;
mod draw;
mod shells;
#[cfg(test)]
mod tests;

pub use draw::draw_history;
use shells::simulate;

const SPEED_PER_POWER: f32 = 0.75;
pub const MAX_POWER: f32 = 1000.0;
/// With "Health limits power" on: max power = health x this (100 HP = 1000).
pub const POWER_PER_HEALTH: f32 = 10.0;
/// Even a nearly dead tank can lob a short shot.
pub const MIN_POWER_LIMIT: f32 = 50.0;
const TANK_RADIUS: f32 = 12.0;
/// Text size of the name above each tank.
const NAME_SIZE: f32 = 16.0;
const BARREL: f32 = 18.0;
/// Fixed physics step: the same shot always flies the same way, whatever the frame rate.
const STEP: f32 = 1.0 / 120.0;
/// Height of a line in the full battle log.
const LOG_ROW: f32 = 24.0;
const FALL_SPEED: f32 = 240.0;
/// Falls shorter than this are free; every pixel beyond costs `FALL_DAMAGE` HP.
pub const FALL_SAFE: f32 = 15.0;
pub const FALL_DAMAGE: f32 = 0.4;
/// A tank under a parachute drifts down this fast (px/s) and lands without damage.
const PARACHUTE_SPEED: f32 = 90.0;
/// Mag Deflector: enemy shells closer than this (px) to the tank are pushed away...
pub const DEFLECT_RADIUS: f32 = 80.0;
/// ...with this acceleration (px/s^2) at the tank, fading to nothing at the edge.
pub const DEFLECT_FORCE: f32 = 3000.0;
/// Energy a Mag Deflector spends on each enemy shell that enters its field, so it can't push shells away for ever.
pub const DEFLECT_COST: f32 = 10.0;
/// A shell this far (px) outside a Mag Deflector's field has left it: coming back in is a new pass.
const DEFLECT_CLEAR: f32 = 6.0;
/// Roller: rolling speed (px/s), the steepest rise it still rolls over (px per px; bumps, not hills), and the
/// longest roll (s).
const ROLL_SPEED: f32 = 150.0;
const ROLL_CLIMB: f32 = 0.05;
const ROLL_TIME: f32 = 5.0;
/// Leapfrog: each hop keeps this share of the sideways speed and bounces up this fast (px/s).
const HOP_KEEP: f32 = 0.75;
const HOP_UP: f32 = 330.0;
/// Funky Bomb: bomblets fly off this fast (px/s, random within).
const BOMBLET_SPEED: (f32, f32) = (160.0, 320.0);
/// Napalm: drops of fire, how long they burn (s), how fast they run downhill (px/s), and how close (px) a tank must be
/// to burn. Damage is collected and dealt every `FIRE_TICK` seconds.
const FIRE_DROPS: usize = 18;
const FIRE_TIME: f32 = 3.0;
const FIRE_FLOW: f32 = 70.0;
const FIRE_REACH: f32 = 16.0;
const FIRE_TICK: f32 = 0.5;
/// A buried bot digs out with a damaging weapon only if that costs it no more than this share of its health.
const DIG_HEALTH_SHARE: f32 = 0.4;
/// Napalm: radius of the burn mark a drop leaves on the ground where it burns out.
const FIRE_SINGE: f32 = 8.0;
/// Digger: how far it burrows sideways (px), how fast (px/s), and how far the ground above it sinks (px).
const DIG_LENGTH: f32 = 160.0;
const DIG_SPEED: f32 = 200.0;
const DIG_DEPTH: f32 = 40.0;
/// Blasts at least this big (radius, px) get the deeper explosion sound.
const BIG_BLAST: f32 = 50.0;
/// A dying tank explodes with this blast.
pub const DEATH_RADIUS: f32 = 35.0;
pub const DEATH_DAMAGE: f32 = 40.0;
/// Self-destruct: a much bigger blast than a normal death.
pub const SELF_DESTRUCT_RADIUS: f32 = 75.0;
pub const SELF_DESTRUCT_DAMAGE: f32 = 80.0;
/// HP every tank loses at the start of each turn once sudden death begins.
pub const SUDDEN_DEATH_DRAIN: f32 = 5.0;
/// A dying tank's carried ammo cooks off: each shell adds this share of its
/// weapon's radius and damage to the death blast, up to the caps.
pub const AMMO_RADIUS_SHARE: f32 = 0.15;
pub const AMMO_DAMAGE_SHARE: f32 = 0.1;
pub const DEATH_MAX_RADIUS: f32 = 110.0;
pub const DEATH_MAX_DAMAGE: f32 = 120.0;
/// Battle log lines fade out after this many seconds.
const LOG_TIME: f32 = 8.0;
const LOG_LINES: usize = 6;
const EXPLOSION_TIME: f32 = 0.6;

/// What a bot remembers between turns: who it shoots at and how far off it still is.
/// The error shrinks with every shot (see `ai.rs`), like a human walking shots onto a target.
#[derive(Clone, Copy, Serialize, Deserialize)]
struct AimMemory {
    target: usize,
    /// (angle, power) error of the next shot.
    error: (f32, f32),
    /// Why this target was picked, shown on screen.
    #[serde(skip)]
    reason: &'static str,
}

/// How much each reason counts when a bot picks a new target.
const PREFER_NEAREST: f32 = 1.0;
const PREFER_WEAKEST: f32 = 0.8;
const PREFER_LEADER: f32 = 0.8;
/// Penalty per other bot already aiming at a tank: spreads fire, so bots
/// don't all gang up on the same player.
const CROWD_PENALTY: f32 = 0.5;
/// With three bots left: penalty for the tank that already aims at the bot picking a target.
const FACE_OFF_PENALTY: f32 = 0.5;

#[derive(Serialize, Deserialize)]
struct Tank {
    /// Bottom-center of the tank, resting on the ground.
    #[serde(with = "Vec2Def")]
    pos: Vec2,
    health: f32,
    /// Degrees: 0 = right, 90 = straight up, 180 = left.
    angle: f32,
    power: f32,
    weapon: usize,
    alive: bool,
    /// Left the round via the pause menu.
    fled: bool,
    fall_from: Option<f32>,
    last_hit_by: Option<usize>,
    /// Path of this tank's previous shot, drawn faintly to help aiming.
    #[serde(skip)]
    last_trail: Vec<Vec2>,
    /// Bots only.
    aim: Option<AimMemory>,
    /// Who hit this tank since its last shot. Bots shoot back.
    #[serde(default)]
    revenge: Option<usize>,
    /// The last weapon this tank fired (not a Tracer), to go back to after a Tracer.
    #[serde(skip)]
    last_weapon: usize,
    /// Aiming help: the enemy this tank's last missed shot was meant for, until its next turn.
    #[serde(skip)]
    coach_target: Option<usize>,
    /// Aiming help: this turn's advice, shown until the tank fires.
    #[serde(skip)]
    coaching: Option<String>,
    /// The shield raised at round start. A Mag Deflector that has run out stays here (at 0 or below) while the
    /// shells that paid for it fly, then goes down (`drop_spent_deflectors`); any other shield goes at 0.
    #[serde(default)]
    shield: Option<ActiveShield>,
    /// A parachute is open (only while falling).
    #[serde(default)]
    parachute: bool,
}

#[derive(Clone, Copy, Serialize, Deserialize)]
struct ActiveShield {
    /// Index into `items::ITEMS`.
    item: usize,
    energy: f32,
}

impl ActiveShield {
    fn deflects(&self) -> bool {
        matches!(ITEMS[self.item].kind, Use::Deflector { .. })
    }

    /// How much is left, 0..1.
    fn strength(&self) -> f32 {
        let full = ITEMS[self.item].shield_energy().unwrap_or(1.0);
        (self.energy / full).clamp(0.0, 1.0)
    }

    /// Ring, bar and number colour: one per kind of shield.
    fn color(&self) -> Color {
        match ITEMS[self.item].kind {
            Use::Deflector { .. } => Color::new(0.85, 0.4, 1.0, 1.0),
            _ if self.item == 2 => Color::new(0.35, 0.55, 1.0, 1.0),
            _ => SKYBLUE,
        }
    }

    /// Energy left, rounded up so a shield never shows 0 while it still holds.
    fn shown(&self) -> f32 {
        self.energy.max(0.0).ceil()
    }
}

impl Tank {
    /// The shield as players see it: up, with energy left (a spent Mag Deflector still pushing its last shells
    /// isn't shown).
    fn shield_up(&self) -> Option<ActiveShield> {
        self.shield.filter(|s| s.energy > 0.0)
    }

    fn center(&self) -> Vec2 {
        self.pos - vec2(0.0, 8.0)
    }

    fn barrel_base(&self) -> Vec2 {
        self.pos - vec2(0.0, 10.0)
    }

    fn muzzle(&self) -> Vec2 {
        self.barrel_base() + direction(self.angle) * BARREL
    }
}

fn direction(angle: f32) -> Vec2 {
    let a = angle.to_radians();
    vec2(a.cos(), -a.sin())
}

/// Height of the HUD bar along the top.
const HUD_HEIGHT: f32 = 44.0;
/// A trail point further sideways than this from the previous one is a shell wrapping around the screen.
const WRAP_JUMP: f32 = 100.0;

/// Whether the step from `a` to `b` is a wrap-around jump (drawn as a gap, not a line across the screen).
fn wraps(a: Vec2, b: Vec2) -> bool {
    (b.x - a.x).abs() > WRAP_JUMP
}

/// Length of a dash (and of the gap after it) in a Tracer's path.
const TRACER_DASH: f32 = 6.0;

/// A Tracer's path, kept for the rest of the round: points `TRACER_DASH` apart, so the dashes come out even.
#[derive(Serialize, Deserialize)]
struct TracerPath {
    owner: usize,
    points: Vec<[f32; 2]>,
}

impl TracerPath {
    fn new(owner: usize, trail: &[Vec2]) -> Self {
        let mut points: Vec<Vec2> = Vec::new();
        for &p in trail {
            match points.last() {
                Some(&last) if !wraps(last, p) && last.distance(p) < TRACER_DASH => {}
                _ => points.push(p),
            }
        }
        Self {
            owner,
            points: points.iter().map(|p| [p.x, p.y]).collect(),
        }
    }
}

/// A human's shot as fired, for the aiming help's coaching: where it first comes down (the unsplit shell for a MIRV,
/// the first touch for Rollers, Diggers and Leapfrogs; `None`: it leaves the field), and where and which way it was
/// fired. After a miss it tells which enemy the shot was meant for.
#[derive(Clone, Copy)]
struct CoachShot {
    landing: Option<Vec2>,
    from: f32,
    dir: f32,
}

/// How far coaching's power advice may be off before it says "a little" / plain / "a lot".
const COACH_LITTLE: f32 = 25.0;
const COACH_SOME: f32 = 75.0;
/// Coaching counts a power as reaching the target when it lands this close (px).
const COACH_REACH: f32 = 40.0;

/// The current turn's Tracer.
#[derive(Clone, Copy, Default, PartialEq)]
enum TracerTurn {
    #[default]
    Unused,
    /// In the air: when it's down, the same tank aims again.
    Flying,
    /// Fired and landed: no second one this turn.
    Used,
}

/// What the aiming preview depends on: whose turn, the aim and the wind.
#[derive(Clone, Copy, PartialEq)]
struct PreviewKey {
    turn: u32,
    current: usize,
    angle: f32,
    power: f32,
    wind: f32,
}

/// Power per pixel between the tank and the cursor when aiming with the mouse: 500 px is full power.
const POWER_PER_PIXEL: f32 = 2.0;
/// Shift slows aiming down this much, with the keys and with the mouse.
const FINE: f32 = 0.2;

/// Mouse aiming: while the left button is held, the barrel points at the cursor and power follows the distance.
#[derive(Clone, Copy)]
struct Drag {
    /// With Shift held: where the cursor and the aim were when Shift went down. Moves then count 5 times less.
    fine: Option<(Vec2, f32, f32)>,
}

/// Angle (degrees, 0 = right, 180 = left) and power for a cursor at `cursor`, aiming from `from`. Below the barrel the
/// angle stops at flat left or right.
fn point_aim(from: Vec2, cursor: Vec2, max_power: f32) -> (f32, f32) {
    let d = cursor - from;
    let angle = (-d.y).atan2(d.x).to_degrees();
    let angle = if angle < 0.0 {
        if d.x < 0.0 { 180.0 } else { 0.0 }
    } else {
        angle
    };
    (angle, (d.length() * POWER_PER_PIXEL).min(max_power))
}

fn launch_velocity(angle: f32, power: f32) -> Vec2 {
    direction(angle) * power * SPEED_PER_POWER
}

struct Projectile {
    pos: Vec2,
    vel: Vec2,
    weapon: usize,
    owner: usize,
    trail: Vec<Vec2>,
    /// A MIRV warhead that already split off.
    child: bool,
    /// Bit per tank: that tank's Mag Deflector already paid for pushing this shell.
    charged: u8,
    motion: Motion,
    /// Leapfrog: hops made so far.
    hops: u8,
}

impl Projectile {
    fn new(pos: Vec2, vel: Vec2, weapon: usize, owner: usize, child: bool) -> Self {
        Self {
            pos,
            vel,
            weapon,
            owner,
            trail: vec![pos],
            child,
            charged: 0,
            motion: Motion::Flying,
            hops: 0,
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
enum Motion {
    Flying,
    /// Roller on the ground: direction (-1 or 1) and seconds rolled.
    Rolling {
        dir: f32,
        time: f32,
    },
    /// Digger under the ground: direction (-1 or 1) and distance left.
    Digging {
        dir: f32,
        left: f32,
    },
}

/// A drop of burning napalm on the ground.
struct Fire {
    x: f32,
    age: f32,
    /// 0..1, fixed per drop: where its flame stands next to the drop and how it flickers, so drops pooled in one
    /// dip burn as one wide, ragged fire.
    seed: f32,
}

struct LogLine {
    text: String,
    color: Color,
    age: f32,
}

/// One line of the round's full battle log (hold L, or from the pause menu).
#[derive(Serialize, Deserialize)]
struct HistoryLine {
    turn: u32,
    text: String,
    color: [f32; 4],
}

struct Explosion {
    pos: Vec2,
    radius: f32,
    age: f32,
    dirt: bool,
}

#[derive(Clone, Copy)]
struct Shot {
    angle: f32,
    power: f32,
    weapon: usize,
}

pub struct RoundEnd {
    pub winner: Option<usize>,
}

#[derive(Clone, Copy)]
enum Phase {
    Aim { think: f32, plan: Option<Shot> },
    Flight,
    Settle,
    Over { timer: f32, winner: Option<usize> },
}

impl Default for Phase {
    /// A loaded battle resumes at the start of the current turn.
    fn default() -> Self {
        Phase::Aim {
            think: 1.0,
            plan: None,
        }
    }
}

/// Saved between sessions; everything mid-flight is skipped, because the game
/// only saves at the start of a turn.
#[derive(Serialize, Deserialize)]
pub struct Battle {
    terrain: Terrain,
    tanks: Vec<Tank>,
    current: usize,
    wind: f32,
    #[serde(skip)]
    phase: Phase,
    #[serde(skip)]
    projectiles: Vec<Projectile>,
    #[serde(skip)]
    explosions: Vec<Explosion>,
    #[serde(skip)]
    fires: Vec<Fire>,
    /// Sound effects since the main loop last took them.
    #[serde(skip)]
    sounds: Vec<Sfx>,
    /// Napalm: who lit the fires, burn damage per tank not dealt yet, and time to the next dealing.
    #[serde(skip)]
    fire_owner: usize,
    #[serde(skip)]
    burns: Vec<f32>,
    #[serde(skip)]
    burn_clock: f32,
    #[serde(skip)]
    accumulator: f32,
    round: u32,
    rounds: u32,
    #[serde(default)]
    rules: Rules,
    /// Turns played this round, for sudden death.
    #[serde(default)]
    turns: u32,
    /// Sudden death already drained health this turn.
    #[serde(default)]
    drained: bool,
    /// Set when a new turn starts: a good moment to autosave.
    #[serde(skip)]
    checkpoint: bool,
    /// Short-lived feed, top left.
    #[serde(skip)]
    log: Vec<LogLine>,
    /// Everything that happened this round, including bot targeting.
    #[serde(default)]
    history: Vec<HistoryLine>,
    /// Index of this turn's "fires" line in `history`, to mark a miss.
    #[serde(skip)]
    shot_line: Option<usize>,
    /// The world as set up, before `Random` settings were picked for this round (`rules.world` has the picks).
    /// `None` in battles saved before Random existed: nothing was random then.
    #[serde(default)]
    setup_world: Option<World>,
    /// This turn's Tracer: one per turn, and it doesn't end the turn.
    #[serde(skip)]
    tracer: TracerTurn,
    /// The current shot hurt an enemy (or dropped one): not a miss.
    #[serde(skip)]
    shot_hit: bool,
    /// Aiming help: where a human's shot first comes down, with and without wind, worked out when it is fired.
    #[serde(skip)]
    coach_shot: Option<CoachShot>,
    /// Aiming help: the last predicted path, kept while the aim doesn't change.
    #[serde(skip)]
    preview: std::cell::RefCell<Option<(PreviewKey, Vec<Vec2>)>>,
    /// Paths of the Tracers fired this round. They stay drawn until the round ends, also after a save.
    #[serde(default)]
    tracers: Vec<TracerPath>,
    /// Mouse aiming in progress (left button held after pressing it on the battlefield).
    #[serde(skip)]
    drag: Option<Drag>,
    /// Firing order this round (tank indices). Empty in battles saved before it existed: player order.
    #[serde(default)]
    order: Vec<usize>,
}

impl Battle {
    /// `order`: who fires when this round (`setup::turn_order`); its first entry starts.
    pub fn new(
        players: &[Player],
        order: Vec<usize>,
        round: u32,
        rounds: u32,
        mut rules: Rules,
    ) -> Self {
        let chosen = rules.world;
        rules.world = chosen.for_round();
        let mut terrain = Terrain::generate(rules.world.terrain);
        let n = players.len();
        // Shots per tank -> the turn it starts (every tank's shot is one turn).
        rules.sudden_death = rules.sudden_death.map(|each| each * n as u32);
        // Shuffle who stands where.
        let mut slots: Vec<usize> = (0..n).collect();
        for i in (1..n).rev() {
            slots.swap(i, gen_range(0, i + 1));
        }
        let tanks = slots
            .iter()
            .map(|&slot| {
                let x = (WIDTH as f32 * (slot as f32 + 0.5) / n as f32 + gen_range(-40.0, 40.0))
                    .clamp(30.0, WIDTH as f32 - 30.0);
                terrain.flatten(x, TANK_RADIUS + 2.0);
                Tank {
                    pos: vec2(x, terrain.height_at(x)),
                    health: 100.0,
                    angle: if x < WIDTH as f32 / 2.0 { 60.0 } else { 120.0 },
                    power: 500.0,
                    weapon: 0,
                    alive: true,
                    fled: false,
                    fall_from: None,
                    last_hit_by: None,
                    last_trail: Vec::new(),
                    aim: None,
                    revenge: None,
                    last_weapon: 0,
                    coaching: None,
                    coach_target: None,
                    shield: None,
                    parachute: false,
                }
            })
            .collect();

        let mut battle = Self {
            terrain,
            tanks,
            current: order.first().copied().unwrap_or(0) % n,
            order,
            wind: rules.world.first_wind(),
            phase: Phase::default(),
            projectiles: Vec::new(),
            explosions: Vec::new(),
            fires: Vec::new(),
            sounds: Vec::new(),
            fire_owner: 0,
            burns: Vec::new(),
            burn_clock: 0.0,
            accumulator: 0.0,
            round,
            rounds,
            rules,
            turns: 0,
            drained: false,
            checkpoint: true,
            log: Vec::new(),
            history: Vec::new(),
            shot_line: None,
            drag: None,
            setup_world: Some(chosen),
            tracers: Vec::new(),
            tracer: TracerTurn::Unused,
            shot_hit: false,
            coach_shot: None,
            preview: std::cell::RefCell::default(),
        };
        let picks = battle.random_picks(true);
        if !picks.is_empty() {
            battle.note(t!("log-random-world", picks = picks.join(", ")), LIGHTGRAY);
        }
        battle
    }

    /// The world as set up and this round's world (with `Random` settings picked).
    pub fn worlds(&self) -> (World, World) {
        (
            self.setup_world.unwrap_or(self.rules.world),
            self.rules.world,
        )
    }

    /// What the `Random` world settings came out as this round, as "Label: value" lines. Without `all`, only what
    /// can't be seen on screen: gravity and walls (the terrain and sky are in view, the HUD shows the wind).
    fn random_picks(&self, all: bool) -> Vec<String> {
        let (chosen, world) = self.worlds();
        let pick = |random: bool, label: &str, value: String| {
            random.then(|| format!("{}: {}", t!(label), value))
        };
        [
            pick(
                all && chosen.terrain.is_random(),
                "setup-terrain",
                world.terrain.text(),
            ),
            pick(
                all && chosen.wind.is_random(),
                "setup-wind",
                world.wind.text(),
            ),
            pick(
                chosen.gravity.is_random(),
                "setup-gravity",
                world.gravity.text(),
            ),
            pick(chosen.walls.is_random(), "setup-walls", world.walls.text()),
            pick(all && chosen.sky.is_random(), "setup-sky", world.sky.text()),
        ]
        .into_iter()
        .flatten()
        .collect()
    }

    /// Raises each player's strongest switched-on shield for this round (uses one up).
    pub fn raise_shields(&mut self, players: &mut [Player]) {
        for (i, player) in players.iter_mut().enumerate() {
            let Some(item) = shield_to_raise(&player.items, player.items_on) else {
                continue;
            };
            player.items[item] -= 1;
            let energy = ITEMS[item].shield_energy().unwrap_or(0.0);
            self.tanks[i].shield = Some(ActiveShield { item, energy });
            let text = t!(
                "log-shield-up",
                name = player.label(),
                you = you(player),
                item = ITEMS[item].text()
            );
            self.note(text, player.color);
        }
    }

    fn world(&self) -> World {
        self.rules.world
    }

    /// Where each living tank is drawn, with its health bar and name above it (as wide as the name).
    fn tank_areas(&self, players: &[Player]) -> Vec<Rect> {
        self.tanks
            .iter()
            .zip(players)
            .filter(|(t, _)| t.alive && !t.fled)
            .map(|(t, p)| {
                // The font is monospaced, half as wide as it is tall.
                let w = (p.label().chars().count() as f32 * NAME_SIZE * 0.5).max(48.0);
                // Name, health number and shield stack up to about 74 px above the ground.
                Rect::new(t.pos.x - w / 2.0, t.pos.y - 74.0, w, 76.0)
            })
            .collect()
    }

    fn max_power(&self, tank: &Tank) -> f32 {
        if self.rules.power_limit {
            (tank.health * POWER_PER_HEALTH).clamp(MIN_POWER_LIMIT, MAX_POWER)
        } else {
            MAX_POWER
        }
    }

    /// No human tank is still in the round: bots play it out at high speed.
    pub fn only_bots_left(&self, players: &[Player]) -> bool {
        !matches!(self.phase, Phase::Over { .. })
            && !self
                .tanks
                .iter()
                .zip(players)
                .any(|(t, p)| t.alive && !p.is_bot())
    }

    /// Runs and draws one frame. Returns `Some` when the round is over.
    pub fn frame(&mut self, players: &mut [Player], settings: &Settings) -> Option<RoundEnd> {
        let mut dt = get_frame_time().min(0.05) * settings.game_speed.factor();
        if self.only_bots_left(players) {
            dt *= settings.bot_speed.factor();
        }
        let over = self.update(dt, players);
        self.draw(players, settings);
        over
    }

    /// Sound effects of what happened since the last call, for the main loop to play.
    pub fn take_sounds(&mut self) -> Vec<Sfx> {
        std::mem::take(&mut self.sounds)
    }

    /// Plays the rest of the round instantly (only bots left), stopping at the result.
    pub fn skip_to_end(&mut self, players: &mut [Player]) {
        const STEP_TIME: f32 = 1.0 / 30.0;
        const MAX_STEPS: u32 = 200_000;
        for _ in 0..MAX_STEPS {
            if matches!(self.phase, Phase::Over { .. }) {
                break;
            }
            self.update(STEP_TIME, players);
        }
        if !matches!(self.phase, Phase::Over { .. }) {
            // Safety net: give the round to the healthiest survivor.
            self.end_by_health();
        }
        self.projectiles.clear();
        self.explosions.clear();
        self.fires.clear();
    }

    fn update(&mut self, dt: f32, players: &mut [Player]) -> Option<RoundEnd> {
        let mut over = None;
        match self.phase {
            Phase::Aim { think, plan } => self.aim(dt, think, plan, players),
            Phase::Flight => self.flight(dt, players),
            Phase::Settle => self.settle(dt, players),
            Phase::Over { timer, winner } => {
                if timer <= 0.0 {
                    over = Some(RoundEnd { winner });
                }
                self.phase = Phase::Over {
                    timer: timer - dt,
                    winner,
                };
            }
        }

        for e in &mut self.explosions {
            e.age += dt;
        }
        self.explosions.retain(|e| e.age < EXPLOSION_TIME);
        for line in &mut self.log {
            line.age += dt;
        }
        self.log.retain(|l| l.age < LOG_TIME);
        over
    }

    fn aim(&mut self, dt: f32, think: f32, plan: Option<Shot>, players: &mut [Player]) {
        let i = self.current;
        let max_power = self.max_power(&self.tanks[i]);
        if players[i].is_bot() {
            let plan = match plan {
                Some(plan) => plan,
                None => self.plan_shot(i, players),
            };
            let tank = &mut self.tanks[i];
            tank.weapon = plan.weapon;
            // Turn the barrel visibly instead of snapping, so humans can follow.
            tank.angle = approach(tank.angle, plan.angle, 90.0 * dt);
            tank.power = approach(tank.power, plan.power, 600.0 * dt);
            #[expect(clippy::float_cmp, reason = "approach() lands exactly on the target")]
            let ready = tank.angle == plan.angle && tank.power == plan.power;
            if think <= 0.0 && ready {
                self.fire(players);
            } else {
                self.phase = Phase::Aim {
                    think: think - dt,
                    plan: Some(plan),
                };
            }
            return;
        }

        let player = &players[i];
        let strip = weapon_strip(player);
        self.mouse_aim(&strip, max_power);
        let tracer_used = self.tracer != TracerTurn::Unused;
        let tank = &mut self.tanks[i];
        let shift = is_key_down(KeyCode::LeftShift);
        let fine = if shift { FINE } else { 1.0 };
        if is_key_down(KeyCode::Left) {
            tank.angle += 50.0 * fine * dt;
        }
        if is_key_down(KeyCode::Right) {
            tank.angle -= 50.0 * fine * dt;
        }
        if is_key_down(KeyCode::Up) {
            tank.power += 350.0 * fine * dt;
        }
        if is_key_down(KeyCode::Down) {
            tank.power -= 350.0 * fine * dt;
        }
        // Mouse wheel: fine power.
        let wheel = mouse_wheel().1;
        if wheel != 0.0 {
            tank.power += wheel.signum() * if shift { 1.0 } else { 10.0 };
        }
        tank.angle = tank.angle.clamp(0.0, 180.0);
        tank.power = tank.power.clamp(0.0, max_power);
        // Weapon strip: click a weapon, or press its key.
        let clicked = is_mouse_button_pressed(MouseButton::Left).then(ui::mouse);
        for (k, &(w, r)) in strip.iter().enumerate() {
            if (STRIP_KEYS
                .get(k)
                .is_some_and(|&(key, _)| is_key_pressed(key))
                || clicked.is_some_and(|m| r.contains(m)))
                && !(tracer_used && WEAPONS[w].kind.is_probe())
            {
                tank.weapon = w;
            }
        }
        if is_key_pressed(KeyCode::Tab) {
            // Cycle to the next weapon we have ammo for.
            for step in 1..=WEAPONS.len() {
                let w = (tank.weapon + step) % WEAPONS.len();
                if player.has(w) && !(tracer_used && WEAPONS[w].kind.is_probe()) {
                    tank.weapon = w;
                    break;
                }
            }
        }
        if is_key_pressed(KeyCode::Space) || is_mouse_button_pressed(MouseButton::Right) {
            self.drag = None;
            self.fire(players);
        }
    }

    /// Left button pressed on the battlefield (not on the HUD bar or the weapon strip) starts aiming with the
    /// mouse; while it's held the barrel follows the cursor; letting go keeps the aim.
    fn mouse_aim(&mut self, strip: &[(usize, Rect)], max_power: f32) {
        let cursor = ui::mouse();
        if is_mouse_button_pressed(MouseButton::Left)
            && cursor.y > HUD_HEIGHT
            && !strip.iter().any(|(_, r)| r.contains(cursor))
        {
            self.drag = Some(Drag { fine: None });
        }
        if !is_mouse_button_down(MouseButton::Left) {
            self.drag = None;
        }
        let Some(drag) = &mut self.drag else {
            return;
        };
        let tank = &mut self.tanks[self.current];
        let from = tank.barrel_base();
        let (angle, power) = if is_key_down(KeyCode::LeftShift) {
            let (anchor, angle0, power0) =
                *drag.fine.get_or_insert((cursor, tank.angle, tank.power));
            let (a0, p0) = point_aim(from, anchor, f32::MAX);
            let (a1, p1) = point_aim(from, cursor, f32::MAX);
            (angle0 + (a1 - a0) * FINE, power0 + (p1 - p0) * FINE)
        } else {
            drag.fine = None;
            point_aim(from, cursor, max_power)
        };
        tank.angle = angle.clamp(0.0, 180.0);
        tank.power = power.clamp(0.0, max_power);
    }

    fn fire(&mut self, players: &mut [Player]) {
        let i = self.current;
        let tank = &mut self.tanks[i];
        let mut weapon = if players[i].has(tank.weapon) {
            tank.weapon
        } else {
            0
        };
        // One Tracer per turn: a second one is a Baby Missile instead (selecting it again is blocked anyway).
        if WEAPONS[weapon].kind.is_probe() && self.tracer != TracerTurn::Unused {
            weapon = 0;
        }
        if weapon != 0 {
            players[i].ammo[weapon] -= 1;
        }
        let probe = WEAPONS[weapon].kind.is_probe();
        if probe {
            self.tracer = TracerTurn::Flying;
            // Back to the weapon fired last, for the real shot.
            let back = tank.last_weapon;
            tank.weapon = if players[i].has(back) { back } else { 0 };
        } else {
            tank.last_weapon = weapon;
        }
        if !probe {
            players[i].round_stats.shots += 1;
        }
        if !players[i].has(weapon) {
            tank.weapon = 0;
        }
        let pos = tank.muzzle();
        let vel = launch_velocity(tank.angle, tank.power);
        let text = t!(
            "log-fires",
            name = players[i].label(),
            you = you(&players[i]),
            weapon = WEAPONS[weapon].text(),
            angle = tank.angle.round(),
            power = tank.power.round()
        );
        // A probe's line gets no "missed": it wasn't aimed at anyone.
        self.shot_line = (!probe).then_some(self.history.len());
        self.shot_hit = false;
        self.coach_shot = None;
        self.tanks[i].coaching = None;
        if self.rules.aim_help.coaches() && !players[i].is_bot() && !probe {
            let (world, deflectors) = (self.world(), self.deflectors(i));
            self.coach_shot = Some(CoachShot {
                landing: simulate(pos, vel, self.wind, world, &self.terrain, &deflectors)
                    .map(|l| l.pos),
                from: self.tanks[i].pos.x,
                dir: vel.x.signum(),
            });
        }
        self.note(text, players[i].color);
        self.projectiles
            .push(Projectile::new(pos, vel, weapon, i, false));
        self.sounds.push(Sfx::Fire);
        self.phase = Phase::Flight;
    }

    fn flight(&mut self, dt: f32, players: &mut [Player]) {
        self.accumulator += dt;
        while self.accumulator >= STEP {
            self.accumulator -= STEP;
            self.step_projectiles(players);
            self.step_fires(players);
        }
        if self.projectiles.is_empty() && self.explosions.is_empty() && self.fires.is_empty() {
            self.drop_spent_deflectors(players);
            self.phase = Phase::Settle;
        }
    }

    /// Adds an event to the feed and the full log.
    fn log(&mut self, text: String, color: Color) {
        self.note(text.clone(), color);
        self.log.push(LogLine {
            text,
            color,
            age: 0.0,
        });
        if self.log.len() > LOG_LINES {
            self.log.remove(0);
        }
    }

    /// Adds a line to the full log only.
    fn note(&mut self, text: String, color: Color) {
        self.history.push(HistoryLine {
            turn: self.turns + 1,
            text,
            color: color.into(),
        });
    }

    fn settle(&mut self, dt: f32, players: &mut [Player]) {
        let mut busy = !self.explosions.is_empty();

        let mut opened = Vec::new();
        for (i, tank) in self.tanks.iter_mut().enumerate() {
            if !tank.alive {
                continue;
            }
            let support = self.terrain.support_y(tank.pos.x, TANK_RADIUS);
            if tank.pos.y < support - 0.5 {
                let start = *tank.fall_from.get_or_insert(tank.pos.y);
                // A parachute opens once the fall would start to hurt.
                let player = &mut players[i];
                if !tank.parachute
                    && tank.pos.y - start > FALL_SAFE
                    && player.items_on[PARACHUTE]
                    && player.items[PARACHUTE] > 0
                {
                    player.items[PARACHUTE] -= 1;
                    tank.parachute = true;
                    opened.push(i);
                }
                let speed = if tank.parachute {
                    PARACHUTE_SPEED
                } else {
                    FALL_SPEED
                };
                tank.pos.y = (tank.pos.y + speed * dt).min(support);
                busy = true;
            } else if let Some(start) = tank.fall_from.take() {
                if !tank.parachute {
                    let fall = tank.pos.y - start;
                    tank.health -= (fall - FALL_SAFE).max(0.0) * FALL_DAMAGE;
                }
                tank.parachute = false;
            }
        }
        for i in opened {
            self.sounds.push(Sfx::Parachute);
            let text = t!(
                "log-parachute",
                name = players[i].label(),
                you = you(&players[i])
            );
            self.log(text, players[i].color);
        }

        for i in 0..self.tanks.len() {
            let tank = &mut self.tanks[i];
            if !tank.alive || tank.health > 0.0 {
                continue;
            }
            tank.alive = false;
            let killer = tank.last_hit_by;
            let center = tank.center();
            players[i].round_stats.died = true;
            let text = match killer {
                Some(k) => t!(
                    "log-destroyed",
                    name = players[k].label(),
                    you = you(&players[k]),
                    target = players[i].label(),
                    target_you = you(&players[i])
                ),
                None => t!(
                    "log-destroyed-self",
                    name = players[i].label(),
                    you = you(&players[i])
                ),
            };
            self.log(text, RED);
            if let Some(k) = killer.filter(|&k| k != i) {
                players[k].kills += 1;
                players[k].round_stats.kills += 1;
                players[k].income.kills += economy::KILL_PAY;
                // Bounty: killing someone who keeps winning pays more.
                players[k].income.bounty += economy::BOUNTY_PER_WIN * players[i].wins.cast_signed();
            }
            // Dying tanks explode (bigger with more ammo aboard) and can take
            // neighbours with them; those kills go to whoever caused the death.
            let (radius, damage) = death_blast(&players[i]);
            self.blast(center, radius, damage, killer.or(Some(i)), players);
            busy = true;
        }

        if busy {
            return;
        }
        self.after_settling(players);
    }

    /// Everything has come to rest: the round ends, sudden death drains, a Tracer's shooter aims again, or the next
    /// turn starts.
    fn after_settling(&mut self, players: &mut [Player]) {
        let alive = self.tanks.iter().filter(|t| t.alive).count();
        if alive <= 1 {
            let winner = self.tanks.iter().position(|t| t.alive);
            self.phase = Phase::Over { timer: 2.5, winner };
            self.sounds.push(Sfx::RoundEnd);
        } else if self.tracer == TracerTurn::Flying {
            // A Tracer doesn't end the turn: the same tank aims again (no new turn, no sudden death drain).
            self.tracer = TracerTurn::Used;
            self.phase = Phase::Aim {
                think: 0.0,
                plan: None,
            };
        } else if self.sudden_death_active() && !self.drained {
            // Sudden death: everyone loses health, then settle again for any deaths.
            self.drained = true;
            if self.turns == self.rules.sudden_death.unwrap_or(0) {
                self.log(t!("log-sudden-death"), RED);
            }
            for tank in self.tanks.iter_mut().filter(|t| t.alive) {
                tank.health -= SUDDEN_DEATH_DRAIN;
            }
        } else {
            self.next_turn(players);
        }
    }

    fn sudden_death_active(&self) -> bool {
        self.rules.sudden_death.is_some_and(|t| self.turns >= t)
    }

    fn next_turn(&mut self, players: &[Player]) {
        self.tracer = TracerTurn::Unused;
        self.terrain.cool();
        // A deflector a blast drained to 0 while things settled goes down now.
        self.drop_spent_deflectors(players);
        // The shot hurt no enemy.
        if let Some(k) = self.shot_line.take()
            && !self.shot_hit
        {
            self.history[k].text += &t!("log-miss");
            if let Some(shot) = self.coach_shot {
                self.coach(shot);
            }
        }
        self.coach_shot = None;
        let n = self.tanks.len();
        let order: Vec<usize> = if self.order.len() == n {
            self.order.clone()
        } else {
            (0..n).collect()
        };
        let at = order.iter().position(|&i| i == self.current).unwrap_or(0);
        if let Some(next) = (1..=n)
            .map(|k| order[(at + k) % n])
            .find(|&i| self.tanks[i].alive)
        {
            self.current = next;
        }
        self.turns += 1;
        self.drained = false;
        self.wind = self.rules.world.next_wind(self.wind);
        let max_power = self.max_power(&self.tanks[self.current]);
        let tank = &mut self.tanks[self.current];
        tank.power = tank.power.min(max_power);
        // Aiming help: advice for this turn, from where the barrel points now and this turn's wind.
        if let Some(target) = tank.coach_target.take()
            && self.tanks[target].alive
        {
            let text = self.advise(self.current, target, players);
            self.note(text.clone(), LIGHTGRAY);
            self.tanks[self.current].coaching = Some(text);
        }
        self.phase = Phase::Aim {
            think: 0.8,
            plan: None,
        };
        self.checkpoint = true;
    }

    fn end_by_health(&mut self) {
        let winner = (0..self.tanks.len())
            .filter(|&i| self.tanks[i].alive)
            .max_by(|&a, &b| self.tanks[a].health.total_cmp(&self.tanks[b].health));
        self.phase = Phase::Over { timer: 2.0, winner };
    }

    /// (round, turn in this round), both counted from 1.
    pub fn progress(&self) -> (u32, u32) {
        (self.round, self.turns + 1)
    }

    /// True once per new turn; the caller autosaves then.
    pub fn take_checkpoint(&mut self) -> bool {
        std::mem::take(&mut self.checkpoint)
    }

    /// Saving captures the battle exactly only between shots.
    pub fn can_save(&self) -> bool {
        matches!(self.phase, Phase::Aim { .. })
    }

    /// A human may flee or self-destruct on their own turn.
    pub fn is_human_turn(&self, players: &[Player]) -> bool {
        self.can_save() && !players[self.current].is_bot()
    }

    /// The current tank retreats. It counts as a lost round, but the tank survives
    /// (no kill for anyone). If no human is left, the round ends right away and the
    /// healthiest survivor takes it.
    /// Developer aid (`--screen buried`): a Dirt Bomb's worth of dirt on every tank.
    pub fn bury_all(&mut self) {
        let radius = WEAPONS
            .iter()
            .find(|w| w.kind == Kind::Dirt)
            .map_or(45.0, |w| w.radius);
        for at in self.tanks.iter().map(|t| t.pos).collect::<Vec<_>>() {
            self.terrain.add_dirt(at, radius);
        }
    }

    pub fn flee(&mut self, players: &mut [Player]) {
        let i = self.current;
        self.tanks[i].alive = false;
        self.tanks[i].fled = true;
        players[i].round_stats.fled = true;
        let text = t!(
            "log-fled",
            name = players[i].label(),
            you = you(&players[i])
        );
        self.log(text, GRAY);

        let alive = self.tanks.iter().filter(|t| t.alive).count();
        let humans_left = self
            .tanks
            .iter()
            .zip(players.iter())
            .any(|(t, p)| t.alive && !p.is_bot());
        if alive <= 1 || !humans_left {
            self.end_by_health();
        } else {
            self.next_turn(players);
        }
    }

    /// The current tank blows itself up with a big blast. Enemies it takes along
    /// count as its kills; its own death gives nobody a kill.
    pub fn self_destruct(&mut self, players: &mut [Player]) {
        let i = self.current;
        let center = self.tanks[i].center();
        self.tanks[i].health = 0.0;
        let text = t!(
            "log-self-destructed",
            name = players[i].label(),
            you = you(&players[i])
        );
        self.log(text, ORANGE);
        self.blast(
            center,
            SELF_DESTRUCT_RADIUS,
            SELF_DESTRUCT_DAMAGE,
            Some(i),
            players,
        );
        self.phase = Phase::Settle;
    }

    /// A human is aiming with the barrel under the dirt: the shot would go off around their own tank.
    fn aiming_into_dirt(&self, players: &[Player]) -> bool {
        matches!(self.phase, Phase::Aim { .. })
            && !players[self.current].is_bot()
            && self.terrain.is_solid(self.tanks[self.current].muzzle())
    }
}

/// The weapon strip's keys, in strip order, with the label shown in front of each weapon.
const STRIP_KEYS: [(KeyCode, &str); 12] = [
    (KeyCode::Key1, "1 "),
    (KeyCode::Key2, "2 "),
    (KeyCode::Key3, "3 "),
    (KeyCode::Key4, "4 "),
    (KeyCode::Key5, "5 "),
    (KeyCode::Key6, "6 "),
    (KeyCode::Key7, "7 "),
    (KeyCode::Key8, "8 "),
    (KeyCode::Key9, "9 "),
    (KeyCode::Key0, "0 "),
    (KeyCode::Minus, "- "),
    (KeyCode::Equal, "= "),
];

/// Weapon strip along the bottom: the weapons `player` has ammo for, in Tab order, with where each one is drawn.
fn weapon_strip(player: &Player) -> Vec<(usize, Rect)> {
    const GAP: f32 = 6.0;
    // More than this and the names get too small: the strip wraps onto a second row above.
    const PER_ROW: usize = 7;
    let owned: Vec<usize> = (0..WEAPONS.len()).filter(|&w| player.has(w)).collect();
    let rows = owned.len().div_ceil(PER_ROW);
    owned
        .chunks(PER_ROW)
        .enumerate()
        .flat_map(|(row, chunk)| {
            let n = chunk.len() as f32;
            let w = ((ui::WIDTH - 32.0 - GAP * (n - 1.0)) / n).min(150.0);
            let left = (ui::WIDTH - (w * n + GAP * (n - 1.0))) / 2.0;
            let y = HEIGHT - 58.0 - (rows - 1 - row) as f32 * (26.0 + GAP);
            chunk.iter().enumerate().map(move |(k, &weapon)| {
                (weapon, Rect::new(left + k as f32 * (w + GAP), y, w, 26.0))
            })
        })
        .collect()
}

/// Whether any box of the strip lies over a tank's area.
fn covers_a_tank(strip: &[(usize, Rect)], tanks: &[Rect]) -> bool {
    strip
        .iter()
        .any(|(_, r)| tanks.iter().any(|t| t.overlaps(r)))
}

/// "yes" for the solo human ("You"), for grammar in translations.
fn you(player: &Player) -> &'static str {
    if player.is_you() { "yes" } else { "no" }
}

/// Death blast of `player`'s tank: the base blast plus a share of every
/// damaging shell it carries (Baby Missiles don't count, they're unlimited).
pub fn death_blast(player: &Player) -> (f32, f32) {
    let (mut radius, mut damage) = (DEATH_RADIUS, DEATH_DAMAGE);
    for (weapon, &count) in WEAPONS.iter().zip(&player.ammo).skip(1) {
        if weapon.kind.is_utility() {
            continue;
        }
        radius += count as f32 * weapon.radius * AMMO_RADIUS_SHARE;
        damage += count as f32 * weapon.damage * AMMO_DAMAGE_SHARE;
    }
    (radius.min(DEATH_MAX_RADIUS), damage.min(DEATH_MAX_DAMAGE))
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Landing {
    pos: Vec2,
    vel: Vec2,
    ground: bool,
}

fn approach(value: f32, target: f32, max_step: f32) -> f32 {
    if (target - value).abs() <= max_step {
        target
    } else {
        value + max_step * (target - value).signum()
    }
}
