//! Hazards: with the world's Hazards setting on, the sky now and then throws something at the field once a
//! turn's shells have come to rest.
//!   Meteors   - fall from above the screen and go off like a Missile where they land.
//!   Lightning - strikes the highest point of a stretch of the field: a hilltop, or a tank standing on it.
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
use crate::world::{Flight, Hazards, step_shell};

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
/// How long a bolt stays on screen, s.
const BOLT_TIME: f32 = 0.4;

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

/// A bolt on screen: its kinks from the sky down, and how long it has been there.
pub(super) struct Bolt {
    path: Vec<Vec2>,
    age: f32,
}

impl Battle {
    /// Once a turn, when everything has come to rest: maybe a hazard. Says whether one began (the shells' phase
    /// then runs again until it is over).
    pub(super) fn hazard(&mut self, players: &mut [Player]) -> bool {
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
            self.strike(gen_range(0.0, WIDTH as f32), players);
        }
        true
    }

    /// Lightning near `x`: it finds the highest point within `BOLT_STRETCH`, a tank's top counting as ground, and
    /// hurts the tanks close to it. No crater, only a burn mark.
    pub(super) fn strike(&mut self, x: f32, players: &mut [Player]) {
        let (from, to) = (
            (x - BOLT_STRETCH / 2.0).max(0.0) as usize,
            ((x + BOLT_STRETCH / 2.0) as usize).min(WIDTH - 1),
        );
        let ground = (from..=to)
            .map(|column| vec2(column as f32, self.terrain.height_at(column as f32)))
            .min_by(|a, b| a.y.total_cmp(&b.y));
        let tanks = self
            .tanks
            .iter()
            .filter(|tank| tank.alive && (from as f32..=to as f32).contains(&tank.pos.x))
            .map(|tank| tank.center() - vec2(0.0, TANK_RADIUS));
        let Some(at) = ground
            .into_iter()
            .chain(tanks)
            .min_by(|a, b| a.y.total_cmp(&b.y))
        else {
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
        // A jagged line from above the screen down to the strike.
        let mut path = vec![at];
        let mut point = at;
        while point.y > HUD_HEIGHT - 60.0 {
            point = vec2(
                point.x + gen_range(-14.0, 14.0),
                point.y - gen_range(18.0, 34.0),
            );
            path.push(point);
        }
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

    /// Bolts fade; says whether a hazard is still going on.
    pub(super) fn hazards_busy(&mut self, dt: f32) -> bool {
        for bolt in &mut self.bolts {
            bolt.age += dt;
        }
        self.bolts.retain(|bolt| bolt.age < BOLT_TIME);
        !self.meteors.is_empty() || !self.bolts.is_empty()
    }

    /// Meteors with their glowing tails, and bolts.
    pub(super) fn draw_hazards(&self) {
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
        self.strike(900.0, players);
    }
}
