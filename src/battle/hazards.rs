//! Hazards: with the world's Hazards setting on, the sky now and then throws something at the field once a
//! turn's shells have come to rest.
//!   Meteors   - fall from above the screen and go off like a Missile where they land.
//!   Lightning - a storm cell gathers and the light dims, then a bolt strikes high ground below it: the highest
//!               of a few spots, or a tank standing high.
//!
//! Nobody owns them: a hit pays nobody and calls for no revenge. A tank they finish off still counts for the last
//! enemy who hit it, as with falls and sudden death.

use macroquad::prelude::*;

use super::shells::deflect;
use super::{Battle, Explosion, HUD_HEIGHT, STEP, TANK_RADIUS, wraps};
use crate::i18n::t;
use crate::player::Player;
use crate::rng::{chance, gen_range};
use crate::sound::Sfx;
use crate::terrain::WIDTH;
use crate::weapons::WEAPONS;
use crate::world::{Flight, Hazards, Weather, step_shell};

impl Hazards {
    /// The chance that something happens after a turn, and the most that comes down at once.
    fn odds(self) -> (f32, usize) {
        match self {
            // Random is resolved before a round starts (`World::for_round`).
            Hazards::Off | Hazards::Random => (0.0, 0),
            Hazards::Rare => (0.1, 1),
            Hazards::Often => (0.3, 3),
        }
    }
}

/// A meteor goes off like this weapon: a Missile.
const METEOR_AS: usize = 1;
/// Lightning: damage at the strike, how far from it a tank is still hurt, and how wide a stretch it picks the
/// highest point of, px.
const BOLT_DAMAGE: f32 = 35.0;
const BOLT_REACH: f32 = 30.0;
const BOLT_STRETCH: f32 = 300.0;
/// How many spots under the cell lightning picks from, besides the tanks there.
const BOLT_TRIES: usize = 6;
/// A stormy round: the chance of lightning after each turn, and how thick the overcast is.
const STORM_LIGHTNING: f32 = 0.35;
const OVERCAST: f32 = 0.7;
/// How long a bolt stays on screen, s.
const BOLT_TIME: f32 = 0.4;
/// A storm: how long its cloud takes to gather before the bolt, and to clear after it, s.
const GATHER: f32 = 0.9;
const CLEAR: f32 = 1.0;
/// How dark the field gets under a storm, and how bright the flash of a strike.
const STORM_DIM: f32 = 0.6;
const FLASH: f32 = 0.15;
/// Where the storm cell hangs: the height of its base (its anvil reaches about 130 px higher), and its width, px.
const CLOUD_BASE: f32 = HUD_HEIGHT + 150.0;
const CLOUD_WIDTH: f32 = 280.0;

pub(super) struct Meteor {
    pos: Vec2,
    vel: Vec2,
    trail: Vec<Vec2>,
}

impl Meteor {
    pub(super) fn falling(pos: Vec2, vel: Vec2) -> Self {
        Self {
            pos,
            vel,
            trail: vec![pos],
        }
    }
}

/// A storm cloud over `x`: it gathers for `GATHER` s, then the bolt strikes and it clears for `CLEAR` s.
pub(super) struct Storm {
    x: f32,
    age: f32,
}

/// A bolt on screen: its kinks from the sky down, and how long it has been there.
pub(super) struct Bolt {
    /// From the strike up to the cell.
    pub(super) path: Vec<Vec2>,
    age: f32,
}

impl Battle {
    /// Once a turn, when everything has come to rest: maybe a hazard. Says whether one began (the shells' phase
    /// then runs again until it is over).
    pub(super) fn hazard(&mut self) -> bool {
        // A stormy round has its own lightning, on top of whatever the Hazards setting brings.
        if self.rules.world.weather == Weather::Storm && chance(STORM_LIGHTNING) {
            self.log(t!("log-lightning"), SKYBLUE);
            self.gather_storm();
            return true;
        }
        let (odds, most) = self.rules.world.hazards.odds();
        if most == 0 || !chance(odds) {
            return false;
        }
        let count = gen_range(1, most + 1);
        if chance(0.5) {
            self.log(t!("log-meteors", n = count), ORANGE);
            for _ in 0..count {
                let pos = vec2(gen_range(40.0, WIDTH as f32 - 40.0), -20.0);
                let vel = vec2(gen_range(-160.0, 160.0), gen_range(120.0, 260.0));
                self.meteors.push(Meteor::falling(pos, vel));
            }
        } else {
            self.log(t!("log-lightning"), SKYBLUE);
            self.gather_storm();
        }
        true
    }

    /// Lightning doesn't come out of a clear sky: a storm cell gathers first, and the bolt falls when it is ready.
    fn gather_storm(&mut self) {
        let x = gen_range(CLOUD_WIDTH / 3.0, WIDTH as f32 - CLOUD_WIDTH / 3.0);
        self.storms.push(Storm { x, age: 0.0 });
    }

    /// A stormy round's sky: a grey overcast over the sky as set (the sun hidden), drawn before the ground.
    pub(super) fn draw_overcast(&self) {
        if self.rules.world.weather == Weather::Storm {
            let grey = Color::new(0.22, 0.24, 0.28, OVERCAST);
            draw_rectangle(0.0, 0.0, WIDTH as f32, crate::terrain::HEIGHT, grey);
        }
    }

    /// A stormy round's rain, in two layers: far drops faint, short and slow, near ones longer and faster. Every
    /// drop moves along its own slant, which follows the wind and slow gusts, at its own pace; each time it
    /// starts again at the top it takes a new place, length and strength, so there are no columns or rows.
    fn draw_rain(&self) {
        if self.rules.world.weather != Weather::Storm {
            return;
        }
        let height = crate::terrain::HEIGHT - HUD_HEIGHT;
        let time = get_time() as f32;
        let gust = (time * 0.7).sin() * 0.15 + (time * 1.9).sin() * 0.08;
        // Sideways px per px of fall.
        let lean = (self.wind / 40.0).clamp(-1.5, 1.5) * 0.6 + gust;
        let mut rain = Rain::default();
        // (drops, speed, length, strength)
        for (layer, (drops, speed, length, strength)) in
            [(110u32, 200.0, 18.0, 0.14), (60, 340.0, 30.0, 0.24)]
                .into_iter()
                .enumerate()
        {
            for k in 0..drops {
                let seed = k * 2 + layer as u32;
                let pace = speed * (0.8 + 0.4 * unit(seed, 0));
                let travelled = unit(seed, 1) * height + time * pace;
                // Which fall this is: a new one each time the drop starts again at the top.
                let fall = (travelled / height) as u32;
                let depth = travelled % height;
                let start = unit(seed, 2 + fall * 3) * (WIDTH as f32 + height);
                // It drifts with the wind as it falls, across the field and round it.
                let x = (start + lean * depth).rem_euclid(WIDTH as f32);
                let y = HUD_HEIGHT + depth;
                let long = length * (0.8 + 0.6 * unit(seed, 3 + fall * 3));
                let along = vec2(lean, 1.0).normalize() * long;
                let alpha = strength * (0.5 + unit(seed, 4 + fall * 3));
                // It falls behind the hills, not over them.
                if y + along.y < self.terrain.height_at(x + along.x) {
                    rain.add(vec2(x, y), along, alpha);
                }
            }
        }
        rain.draw();
    }

    /// Lightning under a cell over `x`: the highest of `BOLT_TRIES` random spots within `BOLT_STRETCH` and the tops of
    /// the tanks there; it hurts the tanks close to it. No crater, only a burn mark.
    pub(super) fn strike(&mut self, x: f32, players: &mut [Player]) {
        let (from, to) = (
            (x - BOLT_STRETCH / 2.0).max(0.0) as usize,
            ((x + BOLT_STRETCH / 2.0) as usize).min(WIDTH - 1),
        );
        // A few spots under the cell, picked at random, and the tanks there: the highest of them takes the bolt.
        // High ground draws it, a tank standing high most of all, but not always the same peak.
        let ground = (0..BOLT_TRIES).map(|_| {
            let column = gen_range(from as f32, to as f32);
            vec2(column, self.terrain.height_at(column))
        });
        let tanks = self
            .tanks
            .iter()
            .filter(|tank| tank.alive && (from as f32..=to as f32).contains(&tank.pos.x))
            .map(|tank| tank.center() - vec2(0.0, TANK_RADIUS));
        let Some(at) = ground.chain(tanks).min_by(|a, b| a.y.total_cmp(&b.y)) else {
            return;
        };
        let reach = BOLT_REACH + TANK_RADIUS;
        let hits: Vec<(usize, f32)> = self
            .tanks
            .iter()
            .enumerate()
            .filter(|(_, tank)| tank.alive)
            .map(|(i, tank)| (i, tank.center().distance(at)))
            .filter(|&(_, distance)| distance < reach)
            .map(|(i, distance)| (i, BOLT_DAMAGE * (1.0 - 0.7 * distance / reach)))
            .collect();
        self.hazard_hits(&hits, "lightning", players);
        self.terrain.burn(at, BOLT_REACH / 2.0);
        self.sounds.push(Sfx::Thunder);
        // A jagged line from the cloud's base down to the strike.
        // A jagged line from the cell's base above `x` down to the strike.
        let from = vec2(x, CLOUD_BASE);
        let steps = (from.distance(at) / 26.0).ceil().max(2.0) as usize;
        let path: Vec<Vec2> = (0..=steps)
            .map(|k| {
                let point = at.lerp(from, k as f32 / steps as f32);
                let jag = if k == 0 || k == steps {
                    0.0
                } else {
                    gen_range(-14.0, 14.0)
                };
                point + vec2(jag, 0.0)
            })
            .collect();
        self.bolts.push(Bolt { path, age: 0.0 });
    }

    /// Hurts tanks with no owner behind it, and says so in the log: `what` is the hazard's key.
    fn hazard_hits(&mut self, hits: &[(usize, f32)], what: &str, players: &mut [Player]) {
        let before: Vec<f32> = self.tanks.iter().map(|tank| tank.health).collect();
        self.hurt(hits, None, players);
        for &(i, _) in hits {
            let lost = (before[i] - self.tanks[i].health).round();
            if lost >= 1.0 {
                let text = t!(
                    &format!("log-hit-by-{what}"),
                    target = players[i].label(),
                    target_you = super::you(&players[i]),
                    dmg = lost
                );
                self.log(text, ORANGE);
            }
        }
    }

    /// One physics step for every meteor in the air: wind, gravity and walls as for a shell, and every Mag
    /// Deflector with energy pushes it (for nothing: it is nobody's shell).
    pub(super) fn step_meteors(&mut self, players: &mut [Player]) {
        let world = self.world();
        let deflectors = self.deflectors(usize::MAX);
        for mut meteor in std::mem::take(&mut self.meteors) {
            let flight = step_shell(&mut meteor.pos, &mut meteor.vel, self.wind, world, STEP);
            deflect(meteor.pos, &mut meteor.vel, &deflectors);
            meteor.trail.push(meteor.pos);
            let hit_tank = self
                .tanks
                .iter()
                .any(|tank| tank.alive && tank.center().distance(meteor.pos) < TANK_RADIUS);
            let landed = matches!(flight, Flight::Wall)
                || hit_tank
                || (matches!(flight, Flight::Flying) && self.terrain.is_solid(meteor.pos));
            if landed {
                self.land_meteor(meteor.pos, players);
            } else if matches!(flight, Flight::Flying) {
                self.meteors.push(meteor);
            }
        }
    }

    /// A meteor goes off at `pos`: a Missile's blast, with the log saying whom it hurt.
    pub(super) fn land_meteor(&mut self, pos: Vec2, players: &mut [Player]) {
        let (radius, damage) = (WEAPONS[METEOR_AS].radius, WEAPONS[METEOR_AS].damage);
        let reach = radius + TANK_RADIUS;
        let hits: Vec<(usize, f32)> = self
            .tanks
            .iter()
            .enumerate()
            .filter(|(_, tank)| tank.alive)
            .map(|(i, tank)| (i, tank.center().distance(pos)))
            .filter(|&(_, distance)| distance < reach)
            .map(|(i, distance)| (i, super::shells::blast_damage(damage, distance, reach)))
            .collect();
        self.hazard_hits(&hits, "meteor", players);
        self.sounds.push(Sfx::Blast);
        self.terrain.carve(pos, radius);
        self.terrain.burn(pos, radius);
        self.explosions.push(Explosion {
            pos,
            radius,
            age: 0.0,
            dirt: false,
        });
    }

    /// Storms gather and strike, bolts fade; says whether a hazard is still going on.
    pub(super) fn hazards_busy(&mut self, dt: f32, players: &mut [Player]) -> bool {
        let mut strikes = Vec::new();
        for storm in &mut self.storms {
            let ready = storm.age < GATHER;
            storm.age += dt;
            if ready && storm.age >= GATHER {
                strikes.push(storm.x);
            }
        }
        for x in strikes {
            self.strike(x, players);
        }
        self.storms.retain(|storm| storm.age < GATHER + CLEAR);
        for bolt in &mut self.bolts {
            bolt.age += dt;
        }
        self.bolts.retain(|bolt| bolt.age < BOLT_TIME);
        !self.meteors.is_empty() || !self.bolts.is_empty() || !self.storms.is_empty()
    }

    /// How dark a storm makes the field now, 0 to `STORM_DIM`: up while it gathers, down while it clears.
    fn storm_dim(storm: &Storm) -> f32 {
        let share = if storm.age < GATHER {
            storm.age / GATHER
        } else {
            1.0 - (storm.age - GATHER) / CLEAR
        };
        STORM_DIM * share.clamp(0.0, 1.0)
    }

    /// Storms: the light dims under them, a dark cloud hangs where the bolt will fall, and a strike flashes.
    fn draw_storms(&self) {
        let dim = self.storms.iter().map(Self::storm_dim).fold(0.0, f32::max);
        if dim > 0.0 {
            // Grey overcast, thick enough to hide the sun: a storm, not nightfall.
            let shade = Color::new(0.2, 0.22, 0.27, dim);
            draw_rectangle(0.0, HUD_HEIGHT, WIDTH as f32, crate::terrain::HEIGHT, shade);
        }
        for storm in &self.storms {
            draw_cell(storm.x, Self::storm_dim(storm) / STORM_DIM, self.wind);
        }
        let flash = self
            .bolts
            .iter()
            .map(|bolt| FLASH * (1.0 - bolt.age / 0.12).max(0.0))
            .fold(0.0, f32::max);
        if flash > 0.0 {
            let light = Color::new(0.85, 0.9, 1.0, flash);
            draw_rectangle(0.0, HUD_HEIGHT, WIDTH as f32, crate::terrain::HEIGHT, light);
        }
    }

    /// Rain, storms, meteors with their glowing tails, and bolts.
    pub(super) fn draw_hazards(&self) {
        self.draw_rain();
        self.draw_storms();
        for meteor in &self.meteors {
            let tail = meteor.trail.windows(2).rev().take(40).enumerate();
            for (k, pair) in tail {
                if !wraps(pair[0], pair[1]) {
                    let fade = 1.0 - k as f32 / 40.0;
                    let color = Color::new(1.0, 0.55, 0.15, 0.7 * fade);
                    draw_line(
                        pair[0].x,
                        pair[0].y,
                        pair[1].x,
                        pair[1].y,
                        1.0 + 3.0 * fade,
                        color,
                    );
                }
            }
            draw_circle(
                meteor.pos.x,
                meteor.pos.y,
                4.0,
                Color::new(1.0, 0.85, 0.5, 1.0),
            );
        }
        for bolt in &self.bolts {
            let fade = 1.0 - bolt.age / BOLT_TIME;
            for pair in bolt.path.windows(2) {
                let glow = Color::new(0.6, 0.8, 1.0, 0.35 * fade);
                draw_line(pair[0].x, pair[0].y, pair[1].x, pair[1].y, 6.0, glow);
                let core = Color::new(1.0, 1.0, 1.0, fade);
                draw_line(pair[0].x, pair[0].y, pair[1].x, pair[1].y, 2.0, core);
            }
        }
    }

    /// Developer aid (`--screen hazards`): a meteor on its way down and a bolt that has just struck, held still.
    pub fn show_hazards(&mut self, players: &mut [Player]) {
        let mut meteor = Meteor::falling(vec2(520.0, 60.0), vec2(140.0, 220.0));
        for _ in 0..90 {
            step_shell(&mut meteor.pos, &mut meteor.vel, 0.0, self.world(), STEP);
            meteor.trail.push(meteor.pos);
        }
        self.meteors.push(meteor);
        // A storm that has just struck, its cloud gathered, the flash over.
        self.storms.push(Storm {
            x: 900.0,
            age: GATHER + 0.1,
        });
        self.strike(900.0, players);
        for bolt in &mut self.bolts {
            bolt.age = 0.15;
        }
    }
}

/// A soft blob: an ellipse built of layers that fade towards its edge, like the sky's clouds
/// (`world::draw_cloud`).
fn soft_blob(x: f32, y: f32, rx: f32, ry: f32, color: Color) {
    // Many thin layers, so the edge fades without showing rings.
    for k in 0..10 {
        let shrink = 1.0 - k as f32 * 0.085;
        let layer = Color {
            a: color.a * 0.17,
            ..color
        };
        draw_ellipse(x, y, rx * shrink, ry * shrink, 0.0, layer);
    }
}

/// A thunderstorm cell over `x`, `amount` 0 to 1 as it gathers and clears: a dark, flat base, a tower of
/// billows that pale towards the top, a wide flat anvil, and rain slanting with the `wind` below the base.
fn draw_cell(x: f32, amount: f32, wind: f32) {
    if amount <= 0.0 {
        return;
    }
    let base = CLOUD_BASE;
    let shade = |dark: f32| Color::new(dark, dark + 0.01, dark + 0.05, 0.95 * amount);
    // Rain first, so the base hangs over it: a grey shaft fading downwards, slanting with the wind, and drops
    // falling through it, each with its own place and pace (see `draw_rain`).
    let slant = (wind / 40.0).clamp(-1.0, 1.0) * 30.0;
    for band in 0..16 {
        let down = band as f32 / 16.0;
        let y = base + down * SHAFT;
        let half = CLOUD_WIDTH * (0.36 - 0.08 * down);
        let lean = slant * down * SHAFT / 120.0;
        // The cell's own grey, as the rain under it.
        let color = Color::new(0.3, 0.31, 0.35, 0.08 * amount * (1.0 - down));
        draw_rectangle(x - half + lean, y, 2.0 * half, SHAFT / 16.0, color);
    }
    let time = get_time() as f32;
    let mut rain = Rain::default();
    for k in 0..40u32 {
        let seed = 1000 + k;
        let travelled = unit(seed, 0) * SHAFT + time * 300.0 * (0.8 + 0.4 * unit(seed, 1));
        let fall = (travelled / SHAFT) as u32;
        let depth = travelled % SHAFT;
        let along = (unit(seed, 2 + fall * 3) - 0.5) * CLOUD_WIDTH * 0.66;
        let long = 16.0 + 14.0 * unit(seed, 3 + fall * 3);
        let fade = 1.0 - depth / SHAFT;
        let top = vec2(x + along + slant * depth / 120.0, base + depth);
        let alpha = 0.3 * amount * fade * (0.5 + unit(seed, 4 + fall * 3));
        rain.add(top, vec2(slant * long / 120.0, long), alpha);
    }
    rain.draw();
    // The anvil: wide and flat at the top, spreading downwind.
    let drift = slant * 1.5;
    soft_blob(
        x + drift,
        base - 112.0,
        CLOUD_WIDTH * 0.75,
        20.0,
        shade(0.42),
    );
    soft_blob(
        x + drift * 0.5,
        base - 100.0,
        CLOUD_WIDTH * 0.5,
        18.0,
        shade(0.38),
    );
    // The tower: billows piled up, paler as they rise.
    let billows = [
        (-0.12, -82.0, 0.2, 30.0, 0.34),
        (0.1, -78.0, 0.18, 28.0, 0.33),
        (-0.02, -60.0, 0.26, 32.0, 0.29),
        (-0.22, -42.0, 0.22, 30.0, 0.25),
        (0.2, -40.0, 0.24, 30.0, 0.25),
        (0.0, -30.0, 0.3, 30.0, 0.22),
    ];
    for (along, up, rx, ry, dark) in billows {
        soft_blob(
            x + along * CLOUD_WIDTH,
            base + up,
            rx * CLOUD_WIDTH,
            ry,
            shade(dark),
        );
    }
    // The base: dark and flat, wider than the tower.
    soft_blob(x, base - 8.0, CLOUD_WIDTH * 0.55, 18.0, shade(0.14));
    soft_blob(
        x - CLOUD_WIDTH * 0.2,
        base - 4.0,
        CLOUD_WIDTH * 0.3,
        12.0,
        shade(0.12),
    );
    soft_blob(
        x + CLOUD_WIDTH * 0.22,
        base - 4.0,
        CLOUD_WIDTH * 0.28,
        12.0,
        shade(0.12),
    );
}

/// How far below a storm cell's base its rain shaft reaches, px.
const SHAFT: f32 = 160.0;

/// Rain drops, drawn in one go: each a thin streak shading smoothly from clear at its top to its full strength at
/// its bottom, as a moving drop looks to the eye.
#[derive(Default)]
struct Rain {
    vertices: Vec<Vertex>,
    indices: Vec<u16>,
}

impl Rain {
    /// A drop from `top` along `along`, at most `alpha` strong.
    fn add(&mut self, top: Vec2, along: Vec2, alpha: f32) {
        let side = vec2(-along.y, along.x).normalize_or_zero() * 0.6;
        let (clear, full) = (
            Color::new(0.7, 0.75, 0.85, 0.0),
            Color::new(0.7, 0.75, 0.85, alpha),
        );
        let at = |p: Vec2, color: Color| Vertex::new(p.x, p.y, 0.0, 0.0, 0.0, color);
        let first = self.vertices.len() as u16;
        let bottom = top + along;
        self.vertices.extend([
            at(top - side, clear),
            at(top + side, clear),
            at(bottom + side, full),
            at(bottom - side, full),
        ]);
        self.indices.extend([0, 1, 2, 0, 2, 3].map(|k| first + k));
    }

    fn draw(self) {
        if self.vertices.is_empty() {
            return;
        }
        draw_mesh(&Mesh {
            vertices: self.vertices,
            indices: self.indices,
            texture: None,
        });
    }
}

/// A number from 0 to 1 that depends only on `seed` and `salt`, the same every frame: the rain's scatter.
fn unit(seed: u32, salt: u32) -> f32 {
    let mut n = seed.wrapping_mul(0x9E37_79B1) ^ salt.wrapping_mul(0x85EB_CA77);
    n ^= n >> 15;
    n = n.wrapping_mul(0x2C1B_3C6D);
    n ^= n >> 12;
    (n & 0xFFFF) as f32 / 65535.0
}
