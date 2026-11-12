//! How bots play: whom they shoot at, what with, and how they find the aim.

use std::ops::RangeInclusive;

use macroquad::prelude::*;

use super::shells::{blast_damage, damage_point, simulate};
use super::{
    AimMemory, BARREL, Battle, CROWD_PENALTY, DIG_HEALTH_SHARE, FACE_OFF_PENALTY, PREFER_LEADER,
    PREFER_NEAREST, PREFER_WEAKEST, Shot, TANK_RADIUS, direction, launch_velocity, you,
};
use crate::ai::{AiProfile, Shots, Targets};
use crate::i18n::t;
use crate::player::Player;
use crate::rng::gen_range;
use crate::terrain::WIDTH;
use crate::weapons::Kind;
use crate::weapons::WEAPONS;

/// The angles a Tosser fires at: within 30 degrees of straight up.
pub(super) const LOB_ANGLES: RangeInclusive<i32> = 61..=121;
/// A Tosser takes a flatter shot when its best lob lands further than this from the target, px.
const LOB_NEAR: f32 = 60.0;
/// A Poolshark takes the shot off a wall when it lands at most this much further from the target than the
/// closest shot, px.
pub(super) const WALL_SHOT_SLACK: f32 = 20.0;

/// A shot a bot has found: where it lands is `miss` px from the target.
#[derive(Clone, Copy)]
pub(super) struct Found {
    pub(super) angle: f32,
    pub(super) power: f32,
    pub(super) miss: f32,
}

impl Battle {
    /// Who a bot shoots at this turn:
    ///   1. whoever hit it since its last shot (revenge),
    ///   2. otherwise the same target as last turn, while it lives,
    ///   3. otherwise the best scored enemy (see `score_targets`).
    ///
    /// A new target means a rough first guess again; the same target keeps
    /// the aim learned so far.
    pub(super) fn choose_target(
        &mut self,
        me: usize,
        players: &[Player],
        profile: &AiProfile,
    ) -> AimMemory {
        let current = self.tanks[me].aim;
        let revenge = self.tanks[me].revenge.take();
        let alive = |j: usize| j != me && self.tanks[j].alive;
        let current = current.filter(|m| alive(m.target));
        // A Spoiler has eyes only for whoever is winning: no revenge, no sharing targets out.
        let leader = (profile.targets == Targets::Leader)
            .then(|| self.leader(me, players))
            .flatten();
        let (current, revenge) = match leader {
            Some(leader) => (current.filter(|m| m.target == leader), None),
            None => (current, revenge),
        };
        // Three bots left: no revenge on one that the third bot is already after (see `three_bots_left`).
        let hunted = |j: usize| self.three_bots_left(players) && self.aimed_at(j, me) > 0;
        let revenge = revenge.filter(|&j| alive(j) && !hunted(j));

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
                let (target, reason) = leader.map_or_else(
                    || self.score_targets(me, players),
                    |leader| (leader, "leader"),
                );
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

    /// The living enemy of `me` with the most round wins, the nearer one of two level; `None` while nobody has won
    /// a round.
    fn leader(&self, me: usize, players: &[Player]) -> Option<usize> {
        let from = self.tanks[me].center();
        (0..self.tanks.len())
            .filter(|&j| j != me && self.tanks[j].alive && players[j].wins > 0)
            .max_by(|&a, &b| {
                let near = |j: usize| -from.distance(self.tanks[j].center());
                (players[a].wins, near(a))
                    .partial_cmp(&(players[b].wins, near(b)))
                    .unwrap_or(std::cmp::Ordering::Equal)
            })
    }

    /// How many other tanks (not `me`) aim at tank `j`.
    fn aimed_at(&self, j: usize, me: usize) -> usize {
        (0..self.tanks.len())
            .filter(|&k| k != me && k != j && self.tanks[k].alive)
            .filter(|&k| self.tanks[k].aim.is_some_and(|m| m.target == j))
            .count()
    }

    /// Exactly three tanks are left and all are bots. Then the usual rules are unfair to the bot that fires first:
    /// the others avoid the tank it aims at, which leaves only itself, and whoever it hits takes revenge on it too,
    /// so two bots end up on it and none on the third. With three, fire is spread evenly only when each bot has one
    /// other after it, so a bot then avoids squaring off with the one that aims at it, and leaves revenge to the
    /// third bot if that one is already on the attacker. With a human in the game bots can't see every aim, and
    /// with four tanks two pairs are even as well, so these rules stay out of it.
    fn three_bots_left(&self, players: &[Player]) -> bool {
        let alive = || (0..self.tanks.len()).filter(|&i| self.tanks[i].alive);
        alive().count() == 3 && alive().all(|i| players[i].is_bot())
    }

    /// Scores every enemy: closer, weaker and more winning tanks score higher;
    /// tanks other bots already aim at score lower. A little randomness keeps
    /// bots from being predictable.
    pub(super) fn score_targets(&self, me: usize, players: &[Player]) -> (usize, &'static str) {
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
            let crowd = self.aimed_at(j, me) as f32;
            // Three bots left: rather not the one that aims at this bot (see `three_bots_left`).
            let facing = self.tanks[j].aim.is_some_and(|m| m.target == me);
            let face_off = if facing && self.three_bots_left(players) {
                FACE_OFF_PENALTY
            } else {
                0.0
            };
            let score =
                near + weak + leader - CROWD_PENALTY * crowd - face_off + gen_range(0.0, 0.3);
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

    /// What a buried bot digs out with, fired straight up into the dirt: a Riot Bomb if it has one (no damage);
    /// else the blast weapon that clears its muzzle in one shot for the least health, if that is no more than
    /// `DIG_HEALTH_SHARE` of what it has left; else a Baby Missile, however many it takes.
    pub(super) fn digging_weapon(&self, me: usize, players: &[Player]) -> usize {
        let has = |w: &usize| players[me].has(*w);
        if let Some(riot) = (0..WEAPONS.len()).find(|w| WEAPONS[*w].kind == Kind::Riot && has(w)) {
            return riot;
        }
        let tank = &self.tanks[me];
        let muzzle = tank.barrel_base() + direction(90.0) * BARREL;
        let dirt = muzzle.y - self.terrain.height_at(muzzle.x);
        let harm = |w: &usize| {
            let weapon = &WEAPONS[*w];
            let reach = weapon.radius + TANK_RADIUS;
            blast_damage(weapon.damage, tank.center().distance(muzzle), reach)
        };
        (0..WEAPONS.len())
            .filter(|w| WEAPONS[*w].kind == Kind::Blast && has(w))
            // A blast at the muzzle takes away a column of dirt as high as the blast is wide.
            .filter(|w| {
                2.0 * WEAPONS[*w].radius > dirt && harm(w) <= DIG_HEALTH_SHARE * tank.health
            })
            .min_by(|a, b| harm(a).total_cmp(&harm(b)))
            .unwrap_or(0)
    }

    /// The angle and power, among `angles` (whole degrees, every third one), that bring `weapon` closest to
    /// `target` without harming the bot itself, and how far off that is. With `walls`, a shot off a wall wins if it
    /// lands within `WALL_SHOT_SLACK` of the closest one.
    pub(super) fn closest_shot(
        &self,
        me: usize,
        weapon: usize,
        target: Vec2,
        angles: RangeInclusive<i32>,
        walls: bool,
    ) -> Option<Found> {
        let shooter = &self.tanks[me];
        let max_power = self.max_power(shooter);
        let safe_distance = WEAPONS[weapon].radius + 25.0;
        let side_speed = WEAPONS[weapon].kind.side_speed();
        let gravity = self.world().gravity();
        // Bots see deflectors like walls and wind, and aim around them.
        let deflectors = self.deflectors(me);
        let (mut best, mut off_wall): (Option<Found>, Option<Found>) = (None, None);
        for angle in angles.step_by(3) {
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
                let miss = hit.distance(target);
                let found = Found { angle, power, miss };
                let closer = |than: Option<Found>| than.is_none_or(|f| miss < f.miss);
                if closer(best) {
                    best = Some(found);
                }
                if landing.off_wall && closer(off_wall) {
                    off_wall = Some(found);
                }
            }
        }
        match (best, off_wall) {
            (Some(direct), Some(wall)) if walls && wall.miss <= direct.miss + WALL_SHOT_SLACK => {
                Some(wall)
            }
            _ => best,
        }
    }

    /// AI: try many angle/power combos in a simulation, take the one landing
    /// closest to a target, then add some error so bots are beatable.
    pub(super) fn plan_shot(&mut self, me: usize, players: &[Player]) -> Shot {
        let profile = self.tanks[me]
            .role
            .or(players[me].bot)
            .expect("only bots plan shots");
        let memory = self.choose_target(me, players, &profile);
        let shooter = &self.tanks[me];
        let target = self.tanks[memory.target].center();
        let max_power = self.max_power(shooter);

        // Buried (no way to point the barrel out of the dirt): any shot goes off around the bot itself, so it digs
        // out instead of firing its biggest weapon.
        let buried = (4..=176).step_by(3).all(|angle| {
            let muzzle = shooter.barrel_base() + direction(angle as f32) * BARREL;
            self.terrain.is_solid(muzzle)
        });
        if buried {
            return Shot {
                angle: 90.0,
                power: max_power.min(500.0),
                weapon: self.digging_weapon(me, players),
            };
        }

        // Usually fire the biggest thing in the inventory (the dearest per shot); sometimes save ammo.
        let mut weapon = 0;
        if crate::rng::chance(0.6) {
            weapon = (1..WEAPONS.len())
                .filter(|&w| players[me].has(w) && !WEAPONS[w].kind.is_utility())
                .max_by_key(|&w| WEAPONS[w].shot_price())
                .unwrap_or(0);
        }
        // A Tosser lobs while a lob gets near; a Poolshark goes off the walls where that lands about as close.
        let found = match profile.shots {
            Shots::Any => self.closest_shot(me, weapon, target, 4..=176, false),
            Shots::Walls => self.closest_shot(me, weapon, target, 4..=176, true),
            Shots::Lobs => self
                .closest_shot(me, weapon, target, LOB_ANGLES, false)
                .filter(|lob| lob.miss <= LOB_NEAR)
                .or_else(|| self.closest_shot(me, weapon, target, 4..=176, false)),
        };
        let mut best = Shot {
            angle: found.map_or(90.0, |f| f.angle),
            power: found.map_or(max_power.min(500.0), |f| f.power),
            weapon,
        };
        best.angle = (best.angle + memory.error.0).clamp(0.0, 180.0);
        best.power = (best.power + memory.error.1).clamp(0.0, max_power);

        // The next shot at this target will be closer, with a little fresh wobble.
        self.tanks[me].aim = Some(AimMemory {
            error: profile.next_error(memory.error),
            ..memory
        });
        best
    }
}
