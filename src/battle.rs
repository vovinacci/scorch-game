//! One round of battle: tanks take turns firing until one is left.
//!
//! Turn phases:
//!   Aim    - current tank adjusts angle/power (keyboard or AI), then fires
//!   Flight - projectiles fly, explode, carve terrain
//!   Settle - tanks fall into craters, dead tanks explode (chain reactions!),
//!            sudden death drains health
//!   Over   - round result shown briefly

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use serde::{Deserialize, Serialize};

use crate::ai::AiProfile;
use crate::economy;
use crate::i18n::t;
use crate::items::{ITEMS, PARACHUTE, Use, shield_to_raise};
use crate::player::Player;
use crate::save::Vec2Def;
use crate::settings::{HealthDisplay, Settings};
use crate::setup::Rules;
use crate::sound::Sfx;
use crate::terrain::{HEIGHT, Terrain, WIDTH};
use crate::ui;
use crate::weapons::{Kind, WEAPONS};
use crate::world::{Flight, World, step_shell};

const SPEED_PER_POWER: f32 = 0.75;
pub const MAX_POWER: f32 = 1000.0;
/// With "Health limits power" on: max power = health x this (100 HP = 1000).
pub const POWER_PER_HEALTH: f32 = 10.0;
/// Even a nearly dead tank can lob a short shot.
pub const MIN_POWER_LIMIT: f32 = 50.0;
const TANK_RADIUS: f32 = 12.0;
const BARREL: f32 = 18.0;
/// Fixed physics step: the same shot always flies the same way, whatever the frame rate.
const STEP: f32 = 1.0 / 120.0;
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

/// The path of a tank's previous shot.
#[derive(Default)]
struct Trail {
    points: Vec<Vec2>,
    /// It was a Tracer: shown even with "Last shot path" off.
    tracer: bool,
}

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
    last_trail: Trail,
    /// Bots only.
    aim: Option<AimMemory>,
    /// Who hit this tank since its last shot. Bots shoot back.
    #[serde(default)]
    revenge: Option<usize>,
    /// The shield raised at round start, while it has energy left.
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
                    last_trail: Trail::default(),
                    aim: None,
                    revenge: None,
                    shield: None,
                    parachute: false,
                }
            })
            .collect();

        Self {
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
        }
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

    /// Where each living tank is drawn, with its health bar and name above it.
    fn tank_areas(&self) -> Vec<Rect> {
        self.tanks
            .iter()
            .filter(|t| t.alive && !t.fled)
            .map(|t| Rect::new(t.pos.x - 24.0, t.pos.y - 44.0, 48.0, 46.0))
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
        let tank = &mut self.tanks[i];
        let fine = if is_key_down(KeyCode::LeftShift) {
            0.2
        } else {
            1.0
        };
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
        tank.angle = tank.angle.clamp(0.0, 180.0);
        tank.power = tank.power.clamp(0.0, max_power);
        // Weapon strip: click a weapon, or press its key.
        let strip = weapon_strip(player);
        let clicked = is_mouse_button_pressed(MouseButton::Left).then(ui::mouse);
        for (k, &(w, r)) in strip.iter().enumerate() {
            if STRIP_KEYS
                .get(k)
                .is_some_and(|&(key, _)| is_key_pressed(key))
                || clicked.is_some_and(|m| r.contains(m))
            {
                tank.weapon = w;
            }
        }
        if is_key_pressed(KeyCode::Tab) {
            // Cycle to the next weapon we have ammo for.
            for step in 1..=WEAPONS.len() {
                let w = (tank.weapon + step) % WEAPONS.len();
                if player.has(w) {
                    tank.weapon = w;
                    break;
                }
            }
        }
        if is_key_pressed(KeyCode::Space) {
            self.fire(players);
        }
    }

    fn fire(&mut self, players: &mut [Player]) {
        let i = self.current;
        let tank = &mut self.tanks[i];
        let weapon = if players[i].has(tank.weapon) {
            tank.weapon
        } else {
            0
        };
        if weapon != 0 {
            players[i].ammo[weapon] -= 1;
        }
        let probe = WEAPONS[weapon].kind.is_probe();
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
            self.phase = Phase::Settle;
        }
    }

    fn step_projectiles(&mut self, players: &mut [Player]) {
        let world = self.world();
        for mut p in std::mem::take(&mut self.projectiles) {
            let next = match p.motion {
                Motion::Rolling { dir, time } => self.roll(p, dir, time, players),
                Motion::Digging { dir, left } => self.dig(p, dir, left),
                Motion::Flying => {
                    let flight = step_shell(&mut p.pos, &mut p.vel, self.wind, world, STEP);
                    p.trail.push(p.pos);

                    if let Kind::Mirv { warheads, spread } = WEAPONS[p.weapon].kind
                        && !p.child
                        && p.vel.y > 0.0
                    {
                        let middle = f32::from(warheads - 1) / 2.0;
                        for k in 0..warheads {
                            // Every warhead is a new shell for the deflectors.
                            let side = (f32::from(k) - middle) * f32::from(spread);
                            let vel = p.vel + vec2(side, 0.0);
                            self.projectiles
                                .push(Projectile::new(p.pos, vel, p.weapon, p.owner, true));
                        }
                        self.tanks[p.owner].last_trail = Trail {
                            points: p.trail,
                            tracer: false,
                        };
                        continue;
                    }

                    self.charge_deflectors(&mut p, players);
                    deflect(p.pos, &mut p.vel, &self.deflectors(p.owner));

                    let hit_tank = self
                        .tanks
                        .iter()
                        .any(|t| t.alive && t.center().distance(p.pos) < TANK_RADIUS);
                    match flight {
                        Flight::Wall => self.impact(p, false, players),
                        Flight::Flying if hit_tank => self.impact(p, false, players),
                        Flight::Flying if self.terrain.is_solid(p.pos) => {
                            self.impact(p, true, players)
                        }
                        // Still flying (even far above the screen).
                        Flight::Flying => Some(p),
                        Flight::Lost => {
                            self.landed(p);
                            None
                        }
                    }
                }
            };
            if let Some(p) = next {
                self.projectiles.push(p);
            }
        }
    }

    /// A shell is done: its path is kept to help aim the next shot (warheads and bomblets don't count).
    fn landed(&mut self, p: Projectile) {
        if !p.child {
            self.tanks[p.owner].last_trail = Trail {
                points: p.trail,
                tracer: WEAPONS[p.weapon].kind.is_probe(),
            };
        }
    }

    /// A flying shell hits a tank, a wall or (`ground`) the ground. Returns it if it carries on: rolling, digging
    /// or hopping.
    fn impact(
        &mut self,
        mut p: Projectile,
        ground: bool,
        players: &mut [Player],
    ) -> Option<Projectile> {
        match WEAPONS[p.weapon].kind {
            Kind::Roller if ground => {
                p.pos.y = self.terrain.height_at(p.pos.x) - 3.0;
                let dir = downhill(&self.terrain, p.pos.x).unwrap_or(sign(p.vel.x));
                p.motion = Motion::Rolling { dir, time: 0.0 };
                return Some(p);
            }
            Kind::Digger if ground => {
                self.sounds.push(Sfx::Dig);
                p.motion = Motion::Digging {
                    dir: sign(p.vel.x),
                    left: DIG_LENGTH,
                };
                return Some(p);
            }
            Kind::Leapfrog { hops } if ground => {
                self.detonate(p.pos, p.weapon, p.owner, players);
                if p.hops < hops {
                    p.hops += 1;
                    p.charged = 0;
                    p.vel = vec2(p.vel.x * HOP_KEEP, -HOP_UP);
                    p.pos.y = self.terrain.height_at(p.pos.x) - 4.0;
                    return Some(p);
                }
            }
            Kind::Funky { bomblets } if !p.child => {
                self.detonate(p.pos, p.weapon, p.owner, players);
                let from = vec2(p.pos.x, p.pos.y.min(self.terrain.height_at(p.pos.x)) - 4.0);
                for _ in 0..bomblets {
                    let angle = gen_range(200.0_f32, 340.0).to_radians();
                    let speed = gen_range(BOMBLET_SPEED.0, BOMBLET_SPEED.1);
                    let vel = vec2(angle.cos(), angle.sin()) * speed;
                    self.projectiles
                        .push(Projectile::new(from, vel, p.weapon, p.owner, true));
                }
            }
            Kind::Napalm => self.ignite(p.pos.x, p.owner),
            // A Tracer only leaves its path (see `landed`).
            Kind::Tracer => {}
            _ => self.detonate(p.pos, p.weapon, p.owner, players),
        }
        self.landed(p);
        None
    }

    /// One step of a Roller on the ground. It explodes against a tank, a wall, a slope too steep, or after a while.
    fn roll(
        &mut self,
        mut p: Projectile,
        dir: f32,
        time: f32,
        players: &mut [Player],
    ) -> Option<Projectile> {
        let next = roll_step(&self.terrain, p.pos.x, dir);
        let hit_tank = next.is_some_and(|next| {
            self.tanks
                .iter()
                .any(|t| t.alive && t.center().distance(next) < TANK_RADIUS + 3.0)
        });
        let Some(next) = next.filter(|_| !hit_tank && time < ROLL_TIME) else {
            self.detonate(p.pos, p.weapon, p.owner, players);
            self.landed(p);
            return None;
        };
        p.pos = next;
        p.trail.push(next);
        p.motion = Motion::Rolling {
            dir,
            time: time + STEP,
        };
        Some(p)
    }

    /// One step of a Digger burrowing sideways: the ground above sinks as it passes.
    fn dig(&mut self, mut p: Projectile, dir: f32, left: f32) -> Option<Projectile> {
        let step = DIG_SPEED * STEP;
        let x = p.pos.x + dir * step;
        self.terrain.sink(p.pos.x, x, DIG_DEPTH);
        self.undermine(p.pos.x, x, p.owner);
        p.pos = vec2(x, self.terrain.height_at(x) + 4.0);
        let left = left - step;
        if left <= 0.0 || !(0.0..WIDTH as f32).contains(&x) {
            self.explosions.push(Explosion {
                pos: p.pos,
                radius: 14.0,
                age: 0.0,
                dirt: true,
            });
            self.landed(p);
            return None;
        }
        p.motion = Motion::Digging { dir, left };
        Some(p)
    }

    /// Napalm lands: drops of fire spread from `x` and run downhill.
    fn ignite(&mut self, x: f32, owner: usize) {
        self.sounds.push(Sfx::Burn);
        self.fire_owner = owner;
        self.burns = vec![0.0; self.tanks.len()];
        self.burn_clock = FIRE_TICK;
        for _ in 0..FIRE_DROPS {
            self.fires.push(Fire {
                x: (x + gen_range(-14.0, 14.0)).clamp(0.0, WIDTH as f32 - 1.0),
                age: gen_range(0.0, 0.6),
            });
        }
    }

    /// One step of burning napalm: drops run downhill, tanks near any drop burn.
    fn step_fires(&mut self, players: &mut [Player]) {
        if self.fires.is_empty() {
            return;
        }
        for f in &mut self.fires {
            f.age += STEP;
            f.x = flow_step(&self.terrain, f.x);
        }
        self.fires.retain(|f| f.age < FIRE_TIME);
        let dps = WEAPONS
            .iter()
            .find(|w| w.kind == Kind::Napalm)
            .map_or(0.0, |w| w.damage);
        for (i, tank) in self.tanks.iter().enumerate() {
            let burning = tank.alive
                && self.fires.iter().any(|f| {
                    (f.x - tank.pos.x).abs() < FIRE_REACH
                        && (self.terrain.height_at(f.x) - tank.pos.y).abs() < FIRE_REACH * 1.5
                });
            if burning {
                self.burns[i] += dps * STEP;
            }
        }
        self.burn_clock -= STEP;
        if self.burn_clock <= 0.0 || self.fires.is_empty() {
            self.burn_clock = FIRE_TICK;
            let hits: Vec<(usize, f32)> = std::mem::take(&mut self.burns)
                .into_iter()
                .enumerate()
                .filter(|&(_, dmg)| dmg > 0.0)
                .collect();
            self.burns = vec![0.0; self.tanks.len()];
            self.hurt(&hits, Some(self.fire_owner), players);
        }
    }

    /// Each Mag Deflector pays `DEFLECT_COST` once for a shell entering its field; one that runs out goes down.
    fn charge_deflectors(&mut self, p: &mut Projectile, players: &[Player]) {
        if WEAPONS[p.weapon].kind.is_probe() {
            return;
        }
        let mut down = Vec::new();
        for (i, tank) in self.tanks.iter_mut().enumerate() {
            let bit = 1u8 << i;
            if i == p.owner
                || !tank.alive
                || p.charged & bit != 0
                || tank.center().distance(p.pos) >= DEFLECT_RADIUS
            {
                continue;
            }
            let Some(shield) = tank.shield.as_mut().filter(|s| s.deflects()) else {
                continue;
            };
            p.charged |= bit;
            shield.energy -= DEFLECT_COST;
            self.sounds.push(Sfx::Shield);
            if shield.energy <= 0.0 {
                tank.shield = None;
                down.push(i);
            }
        }
        for i in down {
            let text = t!(
                "log-shield-down",
                name = players[i].label(),
                you = you(&players[i])
            );
            self.log(text, GRAY);
        }
    }

    /// Centres of the Mag Deflectors that push `owner`'s shells (everyone's but its own).
    fn deflectors(&self, owner: usize) -> Vec<Vec2> {
        self.tanks
            .iter()
            .enumerate()
            .filter(|(i, t)| *i != owner && t.alive && t.shield.is_some_and(|s| s.deflects()))
            .map(|(_, t)| t.center())
            .collect()
    }

    /// Call after `owner` changed the ground between `x0` and `x1`: an enemy tank left above the ground there will
    /// fall, and if the fall kills it, the kill is `owner`'s.
    fn undermine(&mut self, x0: f32, x1: f32, owner: usize) {
        let (a, b) = (x0.min(x1), x0.max(x1));
        for (i, tank) in self.tanks.iter_mut().enumerate() {
            let falls = self.terrain.height_at(tank.pos.x) > tank.pos.y + 0.5;
            if i != owner && tank.alive && (a..=b).contains(&tank.pos.x) && falls {
                tank.last_hit_by = Some(owner);
            }
        }
    }

    fn detonate(&mut self, pos: Vec2, weapon: usize, owner: usize, players: &mut [Player]) {
        let w = &WEAPONS[weapon];
        match w.kind {
            Kind::Dirt => {
                self.terrain.add_dirt(pos, w.radius);
                self.dirt_burst(pos, w.radius);
            }
            Kind::Riot => {
                self.terrain.carve(pos, w.radius);
                self.undermine(pos.x - w.radius, pos.x + w.radius, owner);
                self.dirt_burst(pos, w.radius);
            }
            _ => self.blast(pos, w.radius, w.damage, Some(owner), players),
        }
    }

    /// The sound and look of earth thrown up or cleared away (no blast).
    fn dirt_burst(&mut self, pos: Vec2, radius: f32) {
        self.sounds.push(Sfx::Dirt);
        self.explosions.push(Explosion {
            pos,
            radius,
            age: 0.0,
            dirt: true,
        });
    }

    /// Damages every tank in range (more damage closer to the center) and carves a crater.
    fn blast(
        &mut self,
        pos: Vec2,
        radius: f32,
        damage: f32,
        owner: Option<usize>,
        players: &mut [Player],
    ) {
        let reach = radius + TANK_RADIUS;
        let hits: Vec<(usize, f32)> = self
            .tanks
            .iter()
            .enumerate()
            .filter(|(_, t)| t.alive)
            .map(|(i, t)| (i, t.center().distance(pos)))
            .filter(|&(_, d)| d < reach)
            .map(|(i, d)| (i, damage * (1.0 - 0.7 * d / reach)))
            .collect();
        self.hurt(&hits, owner, players);
        self.sounds.push(if radius >= BIG_BLAST {
            Sfx::BigBlast
        } else {
            Sfx::Blast
        });
        self.terrain.carve(pos, radius);
        self.explosions.push(Explosion {
            pos,
            radius,
            age: 0.0,
            dirt: false,
        });
    }

    /// Deals damage to tanks: shields take it first, the shooter (`owner`) is paid and credited, and the log says so.
    fn hurt(&mut self, damage: &[(usize, f32)], owner: Option<usize>, players: &mut [Player]) {
        let mut hits = Vec::new();
        for &(i, raw) in damage {
            let tank = &mut self.tanks[i];
            let mut dmg = raw;
            // A shield takes the hit first.
            let mut absorbed = 0.0;
            let mut shield_down = false;
            if let Some(shield) = &mut tank.shield {
                absorbed = dmg.min(shield.energy);
                shield.energy -= absorbed;
                dmg -= absorbed;
                if shield.energy <= 0.0 {
                    tank.shield = None;
                    shield_down = true;
                }
            }
            tank.health -= dmg;
            if absorbed > 0.0 {
                self.sounds.push(Sfx::Shield);
            }
            hits.push((i, dmg, absorbed, shield_down));
            if let Some(o) = owner {
                // Credit goes to the last *enemy* who hit this tank. If it later
                // dies by its own shot, a fall or sudden death, that enemy still
                // gets the kill: your hits caused it.
                if o != i {
                    tank.last_hit_by = Some(o);
                    tank.revenge = Some(o);
                    // A hit on a shield pays like a hit on health, so shields don't shut down the catch-up money.
                    let hit = dmg + absorbed;
                    players[o].income.damage += (hit * economy::DAMAGE_PAY) as i32;
                    let stats = &mut players[o].round_stats;
                    stats.damage += hit;
                    stats.best_hit = stats.best_hit.max(hit);
                }
            }
        }
        if let Some(o) = owner {
            for (i, dmg, absorbed, shield_down) in hits {
                let dmg = dmg.round();
                let mut text = if o == i {
                    t!(
                        "log-hit-self",
                        name = players[o].label(),
                        you = you(&players[o]),
                        dmg = dmg
                    )
                } else {
                    t!(
                        "log-hit",
                        name = players[o].label(),
                        you = you(&players[o]),
                        target = players[i].label(),
                        target_you = you(&players[i]),
                        dmg = dmg
                    )
                };
                if absorbed >= 0.5 {
                    text += &t!("log-shield-absorbed", absorbed = absorbed.round());
                }
                self.log(text, players[o].color);
                if shield_down {
                    let text = t!(
                        "log-shield-down",
                        name = players[i].label(),
                        you = you(&players[i])
                    );
                    self.log(text, GRAY);
                }
            }
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
        let alive = self.tanks.iter().filter(|t| t.alive).count();
        if alive <= 1 {
            let winner = self.tanks.iter().position(|t| t.alive);
            self.phase = Phase::Over { timer: 2.5, winner };
            self.sounds.push(Sfx::RoundEnd);
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
            self.next_turn();
        }
    }

    fn sudden_death_active(&self) -> bool {
        self.rules.sudden_death.is_some_and(|t| self.turns >= t)
    }

    fn next_turn(&mut self) {
        // Nothing was logged after the shot: it hit nobody.
        if let Some(k) = self.shot_line.take()
            && k + 1 == self.history.len()
        {
            self.history[k].text += &t!("log-miss");
        }
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
            self.next_turn();
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

    /// Who a bot shoots at this turn:
    ///   1. whoever hit it since its last shot (revenge),
    ///   2. otherwise the same target as last turn, while it lives,
    ///   3. otherwise the best scored enemy (see `score_targets`).
    ///
    /// A new target means a rough first guess again; the same target keeps
    /// the aim learned so far.
    fn choose_target(&mut self, me: usize, players: &[Player], profile: &AiProfile) -> AimMemory {
        let current = self.tanks[me].aim;
        let revenge = self.tanks[me].revenge.take();
        let alive = |j: usize| j != me && self.tanks[j].alive;
        let current = current.filter(|m| alive(m.target));
        let revenge = revenge.filter(|&j| alive(j));

        let fresh = |target: usize, reason: &'static str| AimMemory {
            target,
            error: profile.first_error(),
            reason,
        };
        let memory = match (current, revenge) {
            (Some(memory), Some(attacker)) if memory.target == attacker => memory,
            (_, Some(attacker)) => fresh(attacker, "revenge"),
            (Some(memory), None) => memory,
            (None, None) => {
                let (target, reason) = self.score_targets(me, players);
                fresh(target, reason)
            }
        };
        if current.map(|m| m.target) != Some(memory.target) {
            let text = t!(
                "log-targets",
                name = players[me].label(),
                target = players[memory.target].label(),
                target_you = you(&players[memory.target]),
                reason = t!(&format!("reason-{}", memory.reason))
            );
            self.note(text, players[me].color);
        }
        memory
    }

    /// Scores every enemy: closer, weaker and more winning tanks score higher;
    /// tanks other bots already aim at score lower. A little randomness keeps
    /// bots from being predictable.
    fn score_targets(&self, me: usize, players: &[Player]) -> (usize, &'static str) {
        let from = self.tanks[me].center();
        let most_wins = players.iter().map(|p| p.wins).max().unwrap_or(0).max(1) as f32;
        let mut best = (me, "", f32::MIN);
        for (j, tank) in self.tanks.iter().enumerate() {
            if j == me || !tank.alive {
                continue;
            }
            let near =
                PREFER_NEAREST * (1.0 - from.distance(tank.center()) / WIDTH as f32).max(0.0);
            let weak = PREFER_WEAKEST * (1.0 - tank.health / 100.0).clamp(0.0, 1.0);
            let leader = PREFER_LEADER * players[j].wins as f32 / most_wins;
            let crowd = (0..self.tanks.len())
                .filter(|&k| k != me && k != j && self.tanks[k].alive)
                .filter(|&k| self.tanks[k].aim.is_some_and(|m| m.target == j))
                .count() as f32;
            let score = near + weak + leader - CROWD_PENALTY * crowd + gen_range(0.0, 0.3);
            if score > best.2 {
                let reason = if leader >= near && leader >= weak {
                    "leader"
                } else if weak >= near {
                    "weakest"
                } else {
                    "nearest"
                };
                best = (j, reason, score);
            }
        }
        (best.0, best.1)
    }

    /// AI: try many angle/power combos in a simulation, take the one landing
    /// closest to a target, then add some error so bots are beatable.
    fn plan_shot(&mut self, me: usize, players: &[Player]) -> Shot {
        let profile = players[me].bot.expect("only bots plan shots");
        let memory = self.choose_target(me, players, &profile);
        let shooter = &self.tanks[me];
        let target = self.tanks[memory.target].center();
        let max_power = self.max_power(shooter);

        // Usually fire the biggest thing in the inventory; sometimes save ammo.
        let mut weapon = 0;
        if ui::chance(0.6) {
            weapon = (1..WEAPONS.len())
                .filter(|&w| players[me].has(w) && !WEAPONS[w].kind.is_utility())
                .max_by_key(|&w| WEAPONS[w].price)
                .unwrap_or(0);
        }
        let safe_distance = WEAPONS[weapon].radius + 25.0;
        let side_speed = WEAPONS[weapon].kind.side_speed();
        let gravity = self.world().gravity();

        let mut best = Shot {
            angle: 90.0,
            power: max_power.min(500.0),
            weapon,
        };
        let mut best_distance = f32::MAX;
        // Bots see deflectors like walls and wind, and aim around them.
        let deflectors = self.deflectors(me);
        for angle in (4..=176).step_by(3) {
            for power in (50..=max_power as i32).step_by(25) {
                let (angle, power) = (angle as f32, power as f32);
                let origin = shooter.barrel_base() + direction(angle) * BARREL;
                let Some(landing) = simulate(
                    origin,
                    launch_velocity(angle, power),
                    self.wind,
                    self.world(),
                    &self.terrain,
                    &deflectors,
                ) else {
                    continue;
                };
                // Rollers and Napalm are aimed at where they end up, not where they land.
                let hit = damage_point(WEAPONS[weapon].kind, landing, &self.terrain);
                // MIRV warheads split at the top of the arc and drift apart for as long as they fall, which the
                // landing speed tells (the fall starts with no vertical speed).
                let drift = side_speed * landing.vel.y.max(0.0) / gravity;
                if hit.distance(shooter.center()) < safe_distance + drift {
                    continue;
                }
                let d = hit.distance(target);
                if d < best_distance {
                    best_distance = d;
                    best = Shot {
                        angle,
                        power,
                        weapon,
                    };
                }
            }
        }
        best.angle = (best.angle + memory.error.0).clamp(0.0, 180.0);
        best.power = (best.power + memory.error.1).clamp(0.0, max_power);

        // The next shot at this target will be closer, with a little fresh wobble.
        self.tanks[me].aim = Some(AimMemory {
            error: profile.next_error(memory.error),
            ..memory
        });
        best
    }

    pub fn draw(&self, players: &[Player], settings: &Settings) {
        ui::with_shadow(|| {
            self.draw_scene(players, settings);
            self.draw_hud(players);
        });
    }

    /// The battlefield without the HUD, behind a full-screen menu that needs the top of the screen.
    pub fn draw_backdrop(&self, players: &[Player], settings: &Settings) {
        ui::with_shadow(|| self.draw_scene(players, settings));
    }

    fn draw_scene(&self, players: &[Player], settings: &Settings) {
        self.world().sky.draw();
        self.terrain.draw();

        let trail = &self.tanks[self.current].last_trail;
        if (settings.shot_path || trail.tracer) && matches!(self.phase, Phase::Aim { .. }) {
            // A Tracer's path is brighter and denser: that's what it was bought for.
            let (step, color) = if trail.tracer {
                (3, Color::new(0.5, 1.0, 1.0, 0.6))
            } else {
                (6, Color::new(1.0, 1.0, 1.0, 0.3))
            };
            for p in trail.points.iter().step_by(step) {
                draw_circle(p.x, p.y, 1.5, color);
            }
        }

        for (i, tank) in self.tanks.iter().enumerate() {
            if tank.fled {
                continue;
            }
            if !tank.alive {
                // Wrecks rest on whatever ground is left below them.
                let y = self.terrain.support_y(tank.pos.x, 10.0);
                draw_rectangle(tank.pos.x - 10.0, y - 5.0, 20.0, 5.0, DARKGRAY);
                continue;
            }
            let is_current = i == self.current && matches!(self.phase, Phase::Aim { .. });
            draw_tank(tank, &players[i], is_current, settings.health_display);
        }

        for p in &self.projectiles {
            for w in p.trail.windows(2).rev().take(60) {
                // Skip the jump across the screen when a shell wraps around.
                if (w[1].x - w[0].x).abs() > 100.0 {
                    continue;
                }
                draw_line(
                    w[0].x,
                    w[0].y,
                    w[1].x,
                    w[1].y,
                    2.0,
                    Color::new(1.0, 0.9, 0.5, 0.5),
                );
            }
            if p.pos.y < 0.0 {
                // Off the top of the screen: show where it is.
                draw_triangle(
                    vec2(p.pos.x, 50.0),
                    vec2(p.pos.x - 6.0, 62.0),
                    vec2(p.pos.x + 6.0, 62.0),
                    YELLOW,
                );
            } else {
                draw_circle(p.pos.x, p.pos.y, 3.0, WHITE);
            }
        }

        for e in &self.explosions {
            let k = e.age / EXPLOSION_TIME;
            let r = e.radius * (0.5 + 0.5 * k.sqrt());
            if e.dirt {
                draw_circle(e.pos.x, e.pos.y, r, Color::new(0.5, 0.35, 0.2, 1.0 - k));
            } else {
                draw_circle(e.pos.x, e.pos.y, r, Color::new(1.0, 0.5, 0.1, 1.0 - k));
                draw_circle(
                    e.pos.x,
                    e.pos.y,
                    r * 0.6,
                    Color::new(1.0, 0.95, 0.6, 1.0 - k),
                );
            }
        }

        for f in &self.fires {
            let ground = self.terrain.height_at(f.x);
            let life = 1.0 - f.age / FIRE_TIME;
            let flicker = gen_range(0.7, 1.0);
            draw_circle(
                f.x,
                ground - 3.0,
                4.0 + 3.0 * life * flicker,
                Color::new(1.0, 0.45, 0.05, 0.8 * life),
            );
            draw_circle(
                f.x,
                ground - 4.0,
                2.0 + 1.5 * life,
                Color::new(1.0, 0.9, 0.4, life),
            );
        }

        if settings.battle_log {
            let mut y = 70.0;
            for line in &self.log {
                let fade = ((LOG_TIME - line.age) / 2.0).clamp(0.0, 1.0);
                ui::text_at(
                    &line.text,
                    16.0,
                    y,
                    18.0,
                    Color {
                        a: fade,
                        ..line.color
                    },
                );
                y += 22.0;
            }
        }
    }

    #[expect(clippy::too_many_lines, reason = "the HUD, drawn left to right")]
    fn draw_hud(&self, players: &[Player]) {
        draw_rectangle(0.0, 0.0, ui::WIDTH, 44.0, ui::PANEL);
        let player = &players[self.current];
        let tank = &self.tanks[self.current];
        // Your turn only: on bot turns the bottom of the screen has the skip button.
        if matches!(self.phase, Phase::Aim { .. }) && !player.is_bot() {
            draw_weapon_strip(player, tank.weapon, &self.tank_areas());
        }
        let name = player.label();
        ui::text_fit(&name, 16.0, 29.0, 260.0, 24.0, player.color);
        // The info starts after the name, however long it is.
        let info_x = (16.0 + ui::measure(&name, 24.0).width.min(260.0) + 24.0).max(180.0);
        let weapon = &WEAPONS[tank.weapon];
        let max_power = self.max_power(tank);
        let power = if self.rules.power_limit {
            format!("{:.0}/{:.0}", tank.power, max_power)
        } else {
            format!("{:.0}", tank.power)
        };
        let info = t!(
            "hud-info",
            angle = tank.angle.round(),
            power = power,
            weapon = weapon.text(),
            ammo = player.ammo_label(tank.weapon),
            money = player.money,
            income = player.income.total()
        );
        ui::text_fit(&info, info_x, 29.0, 860.0 - info_x, 22.0, WHITE);

        // Wind indicator: arrow length and direction show strength.
        let cx = 1000.0;
        ui::text_fit(&t!("hud-wind"), 880.0, 29.0, 70.0, 22.0, LIGHTGRAY);
        let len = self.wind / self.world().max_wind().max(1.0) * 45.0;
        if len.abs() > 1.0 {
            draw_line(cx, 22.0, cx + len, 22.0, 3.0, SKYBLUE);
            let tip = cx + len;
            let back = tip - 8.0 * len.signum();
            draw_triangle(vec2(tip, 22.0), vec2(back, 16.0), vec2(back, 28.0), SKYBLUE);
        }
        let strength = if self.world().max_wind() == 0.0 {
            t!("hud-wind-off")
        } else {
            format!("{:.0}", self.wind.abs())
        };
        ui::text_at(strength, cx + 58.0, 29.0, 22.0, SKYBLUE);
        ui::text_at(
            t!("hud-round", round = self.round, rounds = self.rounds),
            1160.0,
            29.0,
            22.0,
            WHITE,
        );

        // Turn counter, top right under the bar.
        let turn = match self.rules.sudden_death {
            Some(start) if self.turns < start => {
                t!("hud-turn-sudden", turn = self.turns + 1, start = start)
            }
            _ => t!("hud-turn", turn = self.turns + 1),
        };
        let w = ui::measure(&turn, 18.0).width;
        ui::text_at(&turn, ui::WIDTH - 16.0 - w, 66.0, 18.0, LIGHTGRAY);

        // The current tank's shield, under the turn counter.
        if let Some(shield) = tank.shield {
            let text = t!(
                "hud-shield",
                name = ITEMS[shield.item].text(),
                energy = shield.shown(),
                full = ITEMS[shield.item].shield_energy().unwrap_or(0.0)
            );
            let w = ui::measure(&text, 18.0).width;
            ui::text_at(&text, ui::WIDTH - 16.0 - w, 88.0, 18.0, shield.color());
        }

        // Sudden death warning or banner.
        if let Some(start) = self.rules.sudden_death {
            if self.turns >= start {
                ui::text_centered(
                    &t!("hud-sudden-death", hp = SUDDEN_DEATH_DRAIN),
                    ui::WIDTH / 2.0,
                    92.0,
                    22.0,
                    RED,
                );
            } else if start - self.turns <= 5 {
                ui::text_centered(
                    &t!("hud-sudden-death-in", n = start - self.turns),
                    ui::WIDTH / 2.0,
                    92.0,
                    20.0,
                    ORANGE,
                );
            }
        }

        if let (Phase::Aim { .. }, Some(memory)) = (self.phase, tank.aim)
            && player.is_bot()
            && self.tanks[memory.target].alive
        {
            let mut text = t!(
                "hud-aiming-at",
                name = player.label(),
                target = players[memory.target].label(),
                target_you = you(&players[memory.target])
            );
            // The reason isn't saved, so it's unknown right after Continue.
            if !memory.reason.is_empty() {
                let reason = t!(&format!("reason-{}", memory.reason));
                text.push_str(" (");
                text.push_str(&reason);
                text.push(')');
            }
            ui::text_centered(&text, ui::WIDTH / 2.0, HEIGHT - 14.0, 20.0, LIGHTGRAY);
        }

        match self.phase {
            Phase::Aim { .. } if !player.is_bot() => ui::text_centered(
                &t!("hud-controls"),
                ui::WIDTH / 2.0,
                HEIGHT - 14.0,
                20.0,
                WHITE,
            ),
            Phase::Over { winner, .. } => {
                let text = match winner {
                    Some(w) => t!(
                        "round-won",
                        name = players[w].label(),
                        you = you(&players[w])
                    ),
                    None => t!("round-draw"),
                };
                let r = Rect::new(320.0, 280.0, 640.0, 90.0);
                ui::panel(r);
                ui::text_centered_fit(&text, r.center().x, r.y + 56.0, r.w - 40.0, 34.0, GOLD);
            }
            _ => {}
        }
    }
}

/// Full battle log of this round, newest at the bottom.
pub fn draw_history(battle: &Battle) {
    draw_rectangle(
        0.0,
        0.0,
        ui::WIDTH,
        ui::HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.6),
    );
    let r = Rect::new(240.0, 60.0, 800.0, 600.0);
    ui::panel(r);
    ui::text_centered(
        &t!("history-title", round = battle.round),
        r.center().x,
        r.y + 50.0,
        30.0,
        GOLD,
    );
    let top = r.y + 90.0;
    let rows = ((r.h - 160.0) / STEP) as usize;
    if battle.history.is_empty() {
        ui::text_centered(&t!("history-empty"), r.center().x, top + 20.0, 20.0, GRAY);
    }
    let start = battle.history.len().saturating_sub(rows);
    let mut last_turn = None;
    for (k, line) in battle.history[start..].iter().enumerate() {
        let y = top + k as f32 * STEP;
        // Show the turn number once per turn.
        if last_turn != Some(line.turn) {
            ui::text_at(
                t!("hud-turn", turn = line.turn),
                r.x + 30.0,
                y,
                18.0,
                LIGHTGRAY,
            );
            last_turn = Some(line.turn);
        }
        ui::text_fit(
            &line.text,
            r.x + 130.0,
            y,
            r.w - 160.0,
            20.0,
            line.color.into(),
        );
    }
}

/// Weapon strip along the bottom: the weapons `player` has ammo for, in Tab order, with where each one is drawn.
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

/// The strip, on your turn. If a tank stands behind it (deep valleys and craters reach down there), the boxes turn
/// see-through, so the tank stays visible; they still work.
fn draw_weapon_strip(player: &Player, selected: usize, tanks: &[Rect]) {
    let strip = weapon_strip(player);
    let fill = if covers_a_tank(&strip, tanks) {
        Color {
            a: 0.25,
            ..ui::PANEL
        }
    } else {
        ui::PANEL
    };
    for (k, (w, r)) in strip.into_iter().enumerate() {
        let on = w == selected;
        draw_rectangle(r.x, r.y, r.w, r.h, fill);
        let border = if on {
            GOLD
        } else {
            Color::new(1.0, 1.0, 1.0, 0.15)
        };
        draw_rectangle_lines(r.x, r.y, r.w, r.h, if on { 2.0 } else { 1.0 }, border);
        let key = STRIP_KEYS.get(k).map_or("", |&(_, label)| label);
        let text = format!("{key}{} {}", WEAPONS[w].text(), player.ammo_label(w));
        let color = if on { WHITE } else { LIGHTGRAY };
        ui::text_centered_fit(&text, r.center().x, r.y + 18.0, r.w - 8.0, 16.0, color);
    }
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

/// Flies a shot ignoring tanks, with the same physics as a real shot.
/// Returns where it lands and how fast it's going then (`ground` false: it hit a wall), or `None` if it leaves the
/// world.
fn simulate(
    mut pos: Vec2,
    mut vel: Vec2,
    wind: f32,
    world: World,
    terrain: &Terrain,
    deflectors: &[Vec2],
) -> Option<Landing> {
    for _ in 0..2400 {
        let flight = step_shell(&mut pos, &mut vel, wind, world, STEP);
        deflect(pos, &mut vel, deflectors);
        match flight {
            Flight::Wall => {
                return Some(Landing {
                    pos,
                    vel,
                    ground: false,
                });
            }
            Flight::Lost => return None,
            Flight::Flying if terrain.is_solid(pos) => {
                return Some(Landing {
                    pos,
                    vel,
                    ground: true,
                });
            }
            Flight::Flying => {}
        }
    }
    None
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Landing {
    pos: Vec2,
    vel: Vec2,
    ground: bool,
}

/// Where a shell of `kind` does its damage after `landing`: a Roller where it stops rolling, Napalm where its fire
/// runs to, anything else where it lands. Ignores tanks in the way, like the rest of a bot's aiming.
fn damage_point(kind: Kind, landing: Landing, terrain: &Terrain) -> Vec2 {
    match kind {
        Kind::Roller if landing.ground => {
            let mut x = landing.pos.x;
            let dir = downhill(terrain, x).unwrap_or(sign(landing.vel.x));
            let mut time = 0.0;
            while let Some(next) = roll_step(terrain, x, dir).filter(|_| time < ROLL_TIME) {
                x = next.x;
                time += STEP;
            }
            vec2(x, terrain.height_at(x))
        }
        Kind::Napalm => {
            let mut x = landing.pos.x;
            for _ in 0..(FIRE_TIME / STEP) as usize {
                x = flow_step(terrain, x);
            }
            vec2(x, terrain.height_at(x))
        }
        _ => landing.pos,
    }
}

/// A Roller's next position from `x` rolling in `dir`, or `None` where it stops: at the edge or a rise too steep.
fn roll_step(terrain: &Terrain, x: f32, dir: f32) -> Option<Vec2> {
    let next = x + dir * ROLL_SPEED * STEP;
    let rise = terrain.height_at(x) - terrain.height_at(next);
    let on_screen = (0.0..WIDTH as f32).contains(&next);
    (on_screen && rise <= ROLL_CLIMB * ROLL_SPEED * STEP)
        .then(|| vec2(next, terrain.height_at(next) - 3.0))
}

/// A drop of napalm's next position from `x`: a little way downhill, or where it is on level ground.
fn flow_step(terrain: &Terrain, x: f32) -> f32 {
    match downhill(terrain, x) {
        Some(dir) => (x + dir * FIRE_FLOW * STEP).clamp(0.0, WIDTH as f32 - 1.0),
        None => x,
    }
}

/// Which way the ground at `x` slopes down (-1 left, 1 right), or `None` where it is level.
fn downhill(terrain: &Terrain, x: f32) -> Option<f32> {
    let left = terrain.height_at((x - 3.0).max(0.0));
    let right = terrain.height_at((x + 3.0).min(WIDTH as f32 - 1.0));
    // Bigger y is lower ground.
    if right > left + 0.5 {
        Some(1.0)
    } else if left > right + 0.5 {
        Some(-1.0)
    } else {
        None
    }
}

fn sign(v: f32) -> f32 {
    if v < 0.0 { -1.0 } else { 1.0 }
}

/// Mag Deflector push for one physics step: away from each deflector within `DEFLECT_RADIUS`, strongest close by.
fn deflect(pos: Vec2, vel: &mut Vec2, deflectors: &[Vec2]) {
    for &centre in deflectors {
        let away = pos - centre;
        let d = away.length();
        if d < DEFLECT_RADIUS && d > 0.0 {
            *vel += away / d * DEFLECT_FORCE * (1.0 - d / DEFLECT_RADIUS) * STEP;
        }
    }
}

fn approach(value: f32, target: f32, max_step: f32) -> f32 {
    if (target - value).abs() <= max_step {
        target
    } else {
        value + max_step * (target - value).signum()
    }
}

#[expect(
    clippy::too_many_lines,
    reason = "one tank with its bars, labels and shield"
)]
fn draw_tank(tank: &Tank, player: &Player, is_current: bool, health: HealthDisplay) {
    let (x, y) = (tank.pos.x, tank.pos.y);
    let base = tank.barrel_base();
    let tip = tank.muzzle();
    draw_line(base.x, base.y, tip.x, tip.y, 3.0, LIGHTGRAY);
    draw_circle(x, y - 8.0, 7.0, player.color);
    draw_rectangle(x - 12.0, y - 8.0, 24.0, 8.0, player.color);

    if let Some(shield) = tank.shield {
        let color = shield.color();
        let a = 0.25 + 0.55 * shield.strength();
        draw_circle(
            x,
            y - 8.0,
            20.0,
            Color {
                a: a * 0.25,
                ..color
            },
        );
        draw_circle_lines(x, y - 8.0, 20.0, 1.5, Color { a, ..color });
    }
    if tank.parachute {
        // Canopy: a half disc above the tank, with three lines down to it.
        let (cx, cy, r) = (x, y - 44.0, 16.0);
        let canopy = Color::new(0.95, 0.95, 0.9, 0.9);
        for k in 0..12 {
            let a0 = std::f32::consts::PI * (1.0 + k as f32 / 12.0);
            let a1 = std::f32::consts::PI * (1.0 + (k + 1) as f32 / 12.0);
            draw_triangle(
                vec2(cx, cy),
                vec2(cx + r * a0.cos(), cy + r * a0.sin()),
                vec2(cx + r * a1.cos(), cy + r * a1.sin()),
                canopy,
            );
        }
        for dx in [-r, 0.0, r] {
            draw_line(cx + dx, cy, x + dx * 0.5, y - 12.0, 1.0, LIGHTGRAY);
        }
    }

    let hp = (tank.health / 100.0).clamp(0.0, 1.0);
    let hp_color = if hp > 0.5 {
        GREEN
    } else if hp > 0.25 {
        YELLOW
    } else {
        RED
    };
    let mut label_y = if tank.shield.is_some() {
        y - 42.0
    } else {
        y - 38.0
    };
    if health != HealthDisplay::Number {
        draw_rectangle(
            x - 16.0,
            y - 32.0,
            32.0,
            4.0,
            Color::from_rgba(40, 40, 40, 255),
        );
        draw_rectangle(x - 16.0, y - 32.0, 32.0 * hp, 4.0, hp_color);
        // Shield energy: a second bar just above, in the shield's colour.
        if let Some(shield) = tank.shield {
            draw_rectangle(
                x - 16.0,
                y - 36.0,
                32.0,
                3.0,
                Color::from_rgba(40, 40, 40, 255),
            );
            draw_rectangle(
                x - 16.0,
                y - 36.0,
                32.0 * shield.strength(),
                3.0,
                shield.color(),
            );
        }
    }
    if health != HealthDisplay::Bar {
        let number_y = if health == HealthDisplay::Number {
            // Clear of the shield ring when one is up.
            if tank.shield.is_some() {
                y - 32.0
            } else {
                y - 26.0
            }
        } else {
            label_y
        };
        let health_text = format!("{:.0}", tank.health.max(0.0).ceil());
        ui::text_centered(&health_text, x, number_y, 16.0, hp_color);
        // Shield energy follows the health number: "75 +40".
        if let Some(shield) = tank.shield {
            ui::text_at(
                format!("+{:.0}", shield.shown()),
                x + ui::measure(&health_text, 16.0).width / 2.0 + 4.0,
                number_y,
                16.0,
                shield.color(),
            );
        }
        label_y = number_y - 16.0;
    }
    ui::text_centered(&player.label(), x, label_y, 16.0, WHITE);

    if is_current {
        let bob = (get_time() * 5.0).sin() as f32 * 4.0;
        let ty = label_y - 24.0 + bob;
        draw_triangle(
            vec2(x, ty + 10.0),
            vec2(x - 7.0, ty),
            vec2(x + 7.0, ty),
            YELLOW,
        );
    }
}

#[cfg(test)]
mod tests;
