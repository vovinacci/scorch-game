//! How bots play: whom they shoot at, what with, and how they find the aim.

use macroquad::prelude::*;

use super::shells::{blast_damage, damage_point, simulate};
use super::{
    AimMemory, BARREL, Battle, CROWD_PENALTY, DIG_HEALTH_SHARE, PREFER_LEADER, PREFER_NEAREST,
    PREFER_WEAKEST, Shot, TANK_RADIUS, direction, launch_velocity, you,
};
use crate::ai::AiProfile;
use crate::i18n::t;
use crate::player::Player;
use crate::rng::gen_range;
use crate::terrain::WIDTH;
use crate::weapons::Kind;
use crate::weapons::WEAPONS;

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

    /// AI: try many angle/power combos in a simulation, take the one landing
    /// closest to a target, then add some error so bots are beatable.
    pub(super) fn plan_shot(&mut self, me: usize, players: &[Player]) -> Shot {
        let profile = players[me].bot.expect("only bots plan shots");
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
}
