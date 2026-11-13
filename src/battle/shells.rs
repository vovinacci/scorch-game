//! Shells: how they fly, what happens where they land, fire, Mag Deflectors, blasts and the damage they do.

use macroquad::prelude::*;

use super::guidance::Guide;
use super::{
    BIG_BLAST, BOMBLET_SPEED, Battle, DEFLECT_CLEAR, DEFLECT_COST, DEFLECT_FORCE, DEFLECT_RADIUS,
    DIG_DEPTH, DIG_LENGTH, DIG_SPEED, Explosion, FIRE_DROPS, FIRE_FLOW, FIRE_REACH, FIRE_SINGE,
    FIRE_TICK, FIRE_TIME, Fire, HOP_KEEP, HOP_UP, Landing, Motion, Projectile, ROLL_CLIMB,
    ROLL_SPEED, ROLL_TIME, STEP, TANK_RADIUS, TracerPath, you,
};
use crate::economy;
use crate::i18n::t;
use crate::player::Player;
use crate::rng::gen_range;
use crate::sound::Sfx;
use crate::terrain::Terrain;
use crate::terrain::WIDTH;
use crate::weapons::Kind;
use crate::weapons::WEAPONS;
use crate::world::Flight;
use crate::world::Walls;
use crate::world::World;
use crate::world::step_shell;

impl Battle {
    pub(super) fn step_projectiles(&mut self, players: &mut [Player]) {
        let world = self.world();
        for mut p in std::mem::take(&mut self.projectiles) {
            let next = match p.motion {
                Motion::Rolling { dir, time } => self.roll(p, dir, time, players),
                Motion::Digging { dir, left } => self.dig(p, dir, left, players),
                Motion::Flying => {
                    let from = p.pos;
                    let flight = step_shell(&mut p.pos, &mut p.vel, self.wind, world, STEP);
                    if let Some(guide) = &mut p.guide {
                        guide.steer(from, &mut p.pos, &mut p.vel);
                    }
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
                        self.tanks[p.owner].last_trail = p.trail;
                        continue;
                    }

                    self.charge_deflectors(&mut p);
                    let pushing = self.deflectors_for(&p);
                    if in_field(p.pos, &pushing) {
                        p.guide = None;
                    }
                    deflect(p.pos, &mut p.vel, &pushing);

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

    /// A shell is done: its path is kept to help aim the next shot (warheads and bomblets don't count). A Tracer's
    /// path stays on the field for the rest of the round instead, and the last real shot's path stays as it was.
    pub(super) fn landed(&mut self, p: Projectile) {
        if p.child {
            return;
        }
        if WEAPONS[p.weapon].kind.is_probe() {
            self.tracers.push(TracerPath::new(p.owner, &p.trail));
        } else {
            self.tanks[p.owner].last_trail = p.trail;
        }
    }

    /// A flying shell hits a tank, a wall or (`ground`) the ground. Returns it if it carries on: rolling, digging
    /// or hopping.
    pub(super) fn impact(
        &mut self,
        mut p: Projectile,
        ground: bool,
        players: &mut [Player],
    ) -> Option<Projectile> {
        // Guidance steers a shell to where it comes down, no further.
        p.guide = None;
        // A shell fired from under the dirt (a buried tank) never got out of the ground. It goes off once, where
        // it is: nothing rolls, hops, scatters or runs out to the surface.
        let buried = p.trail.iter().all(|&q| self.terrain.is_solid(q));
        match WEAPONS[p.weapon].kind {
            Kind::Roller if ground && !buried => {
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
                if p.hops < hops && !buried {
                    p.hops += 1;
                    // Each hop is a new pass through any deflector's field, paid for again.
                    p.charged = 0;
                    p.vel = vec2(p.vel.x * HOP_KEEP, -HOP_UP);
                    p.pos.y = self.terrain.height_at(p.pos.x) - 4.0;
                    return Some(p);
                }
            }
            Kind::Funky { bomblets } if !p.child => {
                self.detonate(p.pos, p.weapon, p.owner, players);
                let from = vec2(p.pos.x, p.pos.y.min(self.terrain.height_at(p.pos.x)) - 4.0);
                let bomblets = if buried { 0 } else { bomblets };
                for _ in 0..bomblets {
                    let angle = gen_range(200.0_f32, 340.0).to_radians();
                    let speed = gen_range(BOMBLET_SPEED.0, BOMBLET_SPEED.1);
                    let vel = vec2(angle.cos(), angle.sin()) * speed;
                    self.projectiles
                        .push(Projectile::new(from, vel, p.weapon, p.owner, true));
                }
            }
            // Fire needs air: under the dirt napalm only smoulders out.
            Kind::Napalm if buried => {
                self.dirt_burst(p.pos, 12.0);
                self.log(t!("log-smothered"), GRAY);
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
    pub(super) fn roll(
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
    pub(super) fn dig(
        &mut self,
        mut p: Projectile,
        dir: f32,
        left: f32,
        players: &[Player],
    ) -> Option<Projectile> {
        let step = DIG_SPEED * STEP;
        let x = p.pos.x + dir * step;
        self.terrain.sink(p.pos.x, x, DIG_DEPTH);
        p.pos = vec2(x, self.terrain.height_at(x) + 4.0);
        let left = left - step;
        if left <= 0.0 || !(0.0..WIDTH as f32).contains(&x) {
            // Tanks only fall once the shells are done, so the whole trench can be checked here.
            let start = x - dir * (DIG_LENGTH - left);
            self.undermine(start, x, p.owner, players);
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
    pub(super) fn ignite(&mut self, x: f32, owner: usize) {
        self.sounds.push(Sfx::Burn);
        self.fire_owner = owner;
        self.burns = vec![0.0; self.tanks.len()];
        self.burn_clock = FIRE_TICK;
        for _ in 0..FIRE_DROPS {
            self.fires.push(Fire {
                x: (x + gen_range(-14.0, 14.0)).clamp(0.0, WIDTH as f32 - 1.0),
                age: gen_range(0.0, 0.6),
                seed: gen_range(0.0, 1.0),
            });
        }
    }

    /// One step of burning napalm: drops run downhill, tanks near any drop burn.
    pub(super) fn step_fires(&mut self, players: &mut [Player]) {
        if self.fires.is_empty() {
            return;
        }
        for f in &mut self.fires {
            f.age += STEP;
            f.x = flow_step(&self.terrain, f.x);
        }
        // Where a drop burns out, the ground stays singed for a while.
        for f in self.fires.iter().filter(|f| f.age >= FIRE_TIME) {
            self.terrain
                .burn(vec2(f.x, self.terrain.height_at(f.x)), FIRE_SINGE);
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

    /// Each Mag Deflector with energy left pays `DEFLECT_COST` for every pass of a shell through its field: once when
    /// the shell comes in, and again if it leaves and comes back. One that runs out keeps pushing the shells that
    /// paid until they leave, and goes down when the turn's shells are done.
    pub(super) fn charge_deflectors(&mut self, p: &mut Projectile) {
        if WEAPONS[p.weapon].kind.is_probe() {
            return;
        }
        for (i, tank) in self.tanks.iter_mut().enumerate() {
            let bit = 1u8 << i;
            let distance = tank.center().distance(p.pos);
            // Well clear of the field: this pass is over.
            if distance >= DEFLECT_RADIUS + DEFLECT_CLEAR {
                p.charged &= !bit;
            }
            if i == p.owner || !tank.alive || p.charged & bit != 0 || distance >= DEFLECT_RADIUS {
                continue;
            }
            // A spent deflector takes no new passes, but keeps pushing the shells that paid until they leave.
            let Some(shield) = tank
                .shield
                .as_mut()
                .filter(|s| s.deflects() && s.energy > 0.0)
            else {
                continue;
            };
            p.charged |= bit;
            shield.energy -= DEFLECT_COST;
            self.sounds.push(Sfx::Shield);
        }
    }

    /// Centres of the Mag Deflectors pushing shell `p`: enemy deflectors with energy left, plus a spent one this
    /// shell paid for on its current pass (its last push still works).
    pub(super) fn deflectors_for(&self, p: &Projectile) -> Vec<Vec2> {
        self.deflectors_paid(p.owner, p.charged)
    }

    /// Enemy deflectors (not `owner`'s) with energy left, plus spent ones whose bit is set in `paid`.
    pub(super) fn deflectors_paid(&self, owner: usize, paid: u8) -> Vec<Vec2> {
        self.tanks
            .iter()
            .enumerate()
            .filter(|(i, t)| {
                let paid = paid & (1u8 << i) != 0;
                *i != owner
                    && t.alive
                    && t.shield
                        .is_some_and(|s| s.deflects() && (s.energy > 0.0 || paid))
            })
            .map(|(_, t)| t.center())
            .collect()
    }

    /// Once the shells are done, Mag Deflectors that ran out of energy go down.
    pub(super) fn drop_spent_deflectors(&mut self, players: &[Player]) {
        let mut down = Vec::new();
        for (tank, player) in self.tanks.iter_mut().zip(players) {
            if tank.shield.is_some_and(|s| s.deflects() && s.energy <= 0.0) {
                tank.shield = None;
                down.push(t!(
                    "log-shield-down",
                    name = player.label(),
                    you = you(player)
                ));
            }
        }
        for text in down {
            self.log(text, GRAY);
        }
    }

    /// Centres of the Mag Deflectors that push `owner`'s shells (everyone's but its own, with energy left).
    pub(super) fn deflectors(&self, owner: usize) -> Vec<Vec2> {
        self.deflectors_paid(owner, 0)
    }

    /// Call after `owner` changed the ground between `x0` and `x1`. An enemy tank there that's no longer held up will
    /// fall (the same test as `settle`): it counts as hit by `owner`, for the kill and for revenge, and the log says
    /// so (so the shot isn't logged as a miss).
    pub(super) fn undermine(&mut self, x0: f32, x1: f32, owner: usize, players: &[Player]) {
        let (a, b) = (x0.min(x1), x0.max(x1));
        let mut dropped = Vec::new();
        for (i, tank) in self.tanks.iter_mut().enumerate() {
            let under = tank.pos.x + TANK_RADIUS >= a && tank.pos.x - TANK_RADIUS <= b;
            let falls = self.terrain.support_y(tank.pos.x, TANK_RADIUS) > tank.pos.y + 0.5;
            if i != owner && tank.alive && under && falls {
                tank.last_hit_by = Some(owner);
                tank.revenge = Some(owner);
                dropped.push(i);
                self.shot_hit = true;
            }
        }
        for i in dropped {
            let text = t!(
                "log-undermined",
                name = players[i].label(),
                you = you(&players[i])
            );
            self.log(text, players[owner].color);
        }
    }

    pub(super) fn detonate(
        &mut self,
        pos: Vec2,
        weapon: usize,
        owner: usize,
        players: &mut [Player],
    ) {
        let w = &WEAPONS[weapon];
        match w.kind {
            Kind::Dirt => {
                self.terrain.add_dirt(pos, w.radius);
                self.dirt_burst(pos, w.radius);
            }
            Kind::Riot => {
                self.terrain.carve(pos, w.radius);
                self.undermine(pos.x - w.radius, pos.x + w.radius, owner, players);
                self.dirt_burst(pos, w.radius);
            }
            _ => self.blast(pos, w.radius, w.damage, Some(owner), players),
        }
    }

    /// The sound and look of earth thrown up or cleared away (no blast).
    pub(super) fn dirt_burst(&mut self, pos: Vec2, radius: f32) {
        self.sounds.push(Sfx::Dirt);
        self.explosions.push(Explosion {
            pos,
            radius,
            age: 0.0,
            dirt: true,
        });
    }

    /// Damages every tank in range (more damage closer to the center) and carves a crater.
    pub(super) fn blast(
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
            .map(|(i, d)| (i, blast_damage(damage, d, reach)))
            .collect();
        self.hurt(&hits, owner, players);
        self.sounds.push(if radius >= BIG_BLAST {
            Sfx::BigBlast
        } else {
            Sfx::Blast
        });
        self.terrain.carve(pos, radius);
        self.terrain.burn(pos, radius);
        self.explosions.push(Explosion {
            pos,
            radius,
            age: 0.0,
            dirt: false,
        });
    }

    /// Deals damage to tanks: shields take it first, the shooter (`owner`) is paid and credited, and the log says so.
    pub(super) fn hurt(
        &mut self,
        damage: &[(usize, f32)],
        owner: Option<usize>,
        players: &mut [Player],
    ) {
        let mut hits = Vec::new();
        for &(i, raw) in damage {
            let tank = &mut self.tanks[i];
            let mut dmg = raw;
            // A shield takes the hit first.
            let mut absorbed = 0.0;
            let mut shield_down = false;
            // (A spent Mag Deflector, at 0 or below, absorbs nothing; it goes down when the shells are done.)
            if let Some(shield) = tank.shield.as_mut().filter(|s| s.energy > 0.0) {
                absorbed = dmg.min(shield.energy);
                shield.energy -= absorbed;
                dmg -= absorbed;
                if shield.energy <= 0.0 && !shield.deflects() {
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
                    self.shot_hit = true;
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
}

/// Flies a shot ignoring tanks, with the same physics as a real shot.
/// Returns where it lands and how fast it's going then (`ground` false: it hit a wall), or `None` if it leaves the
/// world.
pub(super) fn simulate(
    pos: Vec2,
    vel: Vec2,
    wind: f32,
    world: World,
    terrain: &Terrain,
    deflectors: &[Vec2],
) -> Option<Landing> {
    fly(pos, vel, wind, world, terrain, deflectors, None, |_| {})
}

/// The path a shot would take (ignoring tanks), every physics step: the aiming help's preview.
pub(super) fn predict_path(
    pos: Vec2,
    vel: Vec2,
    wind: f32,
    world: World,
    terrain: &Terrain,
    deflectors: &[Vec2],
    guide: Option<Guide>,
) -> Vec<Vec2> {
    let mut path = Vec::new();
    fly(pos, vel, wind, world, terrain, deflectors, guide, |p| {
        path.push(p);
    });
    path
}

/// Flies a shell like `simulate`, telling `each` every position on the way.
#[expect(
    clippy::too_many_arguments,
    reason = "a shell, the air, the ground, what steers it and who watches: grouping them would only rename them"
)]
pub(super) fn fly(
    mut pos: Vec2,
    mut vel: Vec2,
    wind: f32,
    world: World,
    terrain: &Terrain,
    deflectors: &[Vec2],
    mut guide: Option<Guide>,
    mut each: impl FnMut(Vec2),
) -> Option<Landing> {
    let mut off_wall = false;
    for _ in 0..2400 {
        // Where the step would take the shell if there were no walls.
        let free = pos.x + (vel.x + wind * STEP) * STEP;
        let from = pos;
        let flight = step_shell(&mut pos, &mut vel, wind, world, STEP);
        if let Some(guide) = &mut guide {
            guide.steer(from, &mut pos, &mut vel);
        }
        if in_field(pos, deflectors) {
            guide = None;
        }
        off_wall |= matches!(world.walls, Walls::Bouncy | Walls::Wrap)
            && !(0.0..WIDTH as f32).contains(&free);
        deflect(pos, &mut vel, deflectors);
        each(pos);
        match flight {
            Flight::Wall => {
                return Some(Landing {
                    pos,
                    vel,
                    ground: false,
                    off_wall,
                });
            }
            Flight::Lost => return None,
            Flight::Flying if terrain.is_solid(pos) => {
                return Some(Landing {
                    pos,
                    vel,
                    ground: true,
                    off_wall,
                });
            }
            Flight::Flying => {}
        }
    }
    None
}

/// Where a shell of `kind` does its damage after `landing`: a Roller where it stops rolling, Napalm where its fire
/// runs to, anything else where it lands. Ignores tanks in the way, like the rest of a bot's aiming.
pub(super) fn damage_point(kind: Kind, landing: Landing, terrain: &Terrain) -> Vec2 {
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

/// What a blast of `damage` does to a tank `distance` from its centre, within its `reach`: all of it at the centre,
/// 30% at the edge.
pub(super) fn blast_damage(damage: f32, distance: f32, reach: f32) -> f32 {
    damage * (1.0 - 0.7 * distance / reach)
}

/// A Roller's next position from `x` rolling in `dir`, or `None` where it stops: at the edge or a rise too steep.
pub(super) fn roll_step(terrain: &Terrain, x: f32, dir: f32) -> Option<Vec2> {
    let next = x + dir * ROLL_SPEED * STEP;
    let rise = terrain.height_at(x) - terrain.height_at(next);
    let on_screen = (0.0..WIDTH as f32).contains(&next);
    (on_screen && rise <= ROLL_CLIMB * ROLL_SPEED * STEP)
        .then(|| vec2(next, terrain.height_at(next) - 3.0))
}

/// A drop of napalm's next position from `x`: a little way downhill, or where it is on level ground.
pub(super) fn flow_step(terrain: &Terrain, x: f32) -> f32 {
    match downhill(terrain, x) {
        Some(dir) => (x + dir * FIRE_FLOW * STEP).clamp(0.0, WIDTH as f32 - 1.0),
        None => x,
    }
}

/// Which way the ground at `x` slopes down (-1 left, 1 right), or `None` where it is level.
pub(super) fn downhill(terrain: &Terrain, x: f32) -> Option<f32> {
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

pub(super) fn sign(v: f32) -> f32 {
    if v < 0.0 { -1.0 } else { 1.0 }
}

/// Mag Deflector push for one physics step: away from each deflector within `DEFLECT_RADIUS`, strongest close by.
/// Is `pos` inside the field of one of `deflectors`? Guidance stops working there.
pub(super) fn in_field(pos: Vec2, deflectors: &[Vec2]) -> bool {
    deflectors
        .iter()
        .any(|centre| centre.distance(pos) < DEFLECT_RADIUS)
}

pub(super) fn deflect(pos: Vec2, vel: &mut Vec2, deflectors: &[Vec2]) {
    for &centre in deflectors {
        let away = pos - centre;
        let d = away.length();
        if d < DEFLECT_RADIUS && d > 0.0 {
            *vel += away / d * DEFLECT_FORCE * (1.0 - d / DEFLECT_RADIUS) * STEP;
        }
    }
}
