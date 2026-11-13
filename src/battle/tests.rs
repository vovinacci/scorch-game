//! Tests for the battle: weapons, shields, turns, bots' aim and the HUD layout.

use super::draw::backing_alpha;
use super::shells::{damage_point, predict_path};
use super::*;
use crate::ai::PRESETS;
use crate::items::PARACHUTE;
use crate::setup::Setup;
use crate::world::{GravityLevel, Sky, Walls, WindLevel};

mod balance;
mod bots;
mod coach;
mod guidance;
mod rules;
mod screen;
mod shields;
mod weapons;

fn bots(n: usize) -> Vec<Player> {
    (0..n)
        .map(|i| Player::new(&format!("B{i}"), WHITE, Some(PRESETS[1])))
        .collect()
}

fn battle(players: &[Player], rules: Rules) -> Battle {
    Battle::new(players, (0..players.len()).collect(), 1, 5, rules)
}

fn weapon(name: &str) -> usize {
    WEAPONS.iter().position(|w| w.name == name).unwrap()
}

/// Two bots, tanks parked at the far edges, on ground shaped by `height`.
fn range(height: impl Fn(f32) -> f32) -> (Vec<Player>, Battle) {
    let players = bots(2);
    let mut b = battle(&players, Setup::default().rules());
    b.terrain = Terrain::from_fn(height);
    b.wind = 0.0;
    for (tank, x) in b.tanks.iter_mut().zip([30.0, 1250.0]) {
        tank.pos = vec2(x, b.terrain.height_at(x));
    }
    (players, b)
}

/// Drops `weapon` straight down onto the ground at `x`, fired by tank 0.
fn drop(b: &mut Battle, weapon: usize, x: f32, vel_x: f32) {
    let y = b.terrain.height_at(x) - 1.0;
    b.projectiles.push(Projectile::new(
        vec2(x, y),
        vec2(vel_x, 200.0),
        weapon,
        0,
        false,
    ));
}

/// Runs shells and fires until they are all done; returns every explosion.
fn run(b: &mut Battle, players: &mut [Player]) -> Vec<Vec2> {
    for _ in 0..20_000 {
        if b.projectiles.is_empty() && b.fires.is_empty() {
            break;
        }
        b.step_projectiles(players);
        b.step_fires(players);
    }
    assert!(
        b.projectiles.is_empty() && b.fires.is_empty(),
        "everything settles"
    );
    b.explosions
        .iter()
        .filter(|e| !e.dirt)
        .map(|e| e.pos)
        .collect()
}

/// A valley with its lowest point at x = 640.
fn valley(x: f32) -> f32 {
    600.0 - (x - 640.0).abs() * 0.2
}

/// A human (tank 0, x = 200) against a bot (tank 1, x = 900) on `ground`, with coaching on.
fn coached(ground: fn(f32) -> f32, wind: f32) -> (Vec<Player>, Battle) {
    let mut players = bots(2);
    players[0].bot = None;
    let mut rules = Setup::default().rules();
    rules.aim_help = crate::setup::AimHelp::Coaching;
    let mut b = battle(&players, rules);
    b.terrain = Terrain::from_fn(ground);
    b.wind = wind;
    b.current = 0;
    for (tank, x) in b.tanks.iter_mut().zip([200.0, 900.0]) {
        tank.pos = vec2(x, ground(x));
    }
    (players, b)
}

/// Fires tank 0's shot with `weapon` at `angle` / `power` and plays the turn out; returns the history after it.
fn fire_and_settle(
    b: &mut Battle,
    players: &mut [Player],
    weapon: usize,
    angle: f32,
    power: f32,
) -> Vec<String> {
    let before = b.history.len();
    players[0].ammo[weapon] = players[0].ammo[weapon].max(1);
    b.tanks[0].weapon = weapon;
    b.tanks[0].angle = angle;
    b.tanks[0].power = power;
    b.fire(players);
    for _ in 0..40_000 {
        if !matches!(b.phase, Phase::Flight | Phase::Settle) {
            break;
        }
        b.update(STEP, players);
    }
    b.history[before..].iter().map(|l| l.text.clone()).collect()
}
