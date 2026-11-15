//! Driving: on its turn a tank moves along the ground, on fuel. Slopes cost more, and dirt too steep to climb or
//! a drop too high to take without harm stops it.

use macroquad::prelude::*;

use super::{Battle, FALL_SAFE, TANK_RADIUS};
use crate::items::FUEL;
use crate::player::Player;
use crate::terrain::{HEIGHT, WIDTH};
use crate::ui;

/// How fast a tank drives, px/s.
pub(super) const DRIVE_SPEED: f32 = 60.0;
/// The steepest ground a tank climbs: px up per px along. Steeper is a wall.
const CLIMB: f32 = 1.5;
/// Fuel for a px on the level or downhill; each px of height gained costs `UPHILL_COST` more.
const LEVEL_COST: f32 = 1.0;
const UPHILL_COST: f32 = 2.0;
/// Tanks keep this far apart, centre to centre, px.
const TANK_GAP: f32 = 2.0 * TANK_RADIUS + 4.0;

/// The two buttons to drive with the mouse, beside the weapon strip, and the way each drives.
pub(super) const BUTTONS: [(Rect, f32); 2] = [
    (Rect::new(24.0, HEIGHT - 58.0, 56.0, 26.0), -1.0),
    (Rect::new(ui::WIDTH - 80.0, HEIGHT - 58.0, 56.0, 26.0), 1.0),
];

impl Battle {
    /// The way the mouse drives the tank just now: -1 or 1 while a drive button is held down, else 0. The buttons
    /// are there only while `player` has fuel.
    pub(super) fn drive_buttons_held(player: &Player) -> f32 {
        if player.items[FUEL] == 0 || !is_mouse_button_down(MouseButton::Left) {
            return 0.0;
        }
        let cursor = ui::mouse();
        BUTTONS
            .iter()
            .find(|(r, _)| r.contains(cursor))
            .map_or(0.0, |&(_, dir)| dir)
    }

    /// Draws the drive buttons: an arrow each.
    pub(super) fn draw_drive_buttons() {
        let cursor = ui::mouse();
        for (r, dir) in BUTTONS {
            let lit = r.contains(cursor);
            let back = if lit { 0.9 } else { 0.7 };
            draw_rectangle(r.x, r.y, r.w, r.h, Color::new(0.05, 0.06, 0.12, back));
            let (mid, tip) = (r.center(), r.center().x + dir * 12.0);
            let tail = mid.x - dir * 10.0;
            let color = if lit { WHITE } else { LIGHTGRAY };
            draw_triangle(
                vec2(tip, mid.y),
                vec2(tail, mid.y - 8.0),
                vec2(tail, mid.y + 8.0),
                color,
            );
        }
    }

    /// Where a tank at `pos` (not tank `i`'s place, but its width and its neighbours) is after driving one px
    /// towards `dir` (-1 left, 1 right), and the fuel that px costs; `None` where it can't go.
    pub(super) fn drive_step(&self, i: usize, pos: Vec2, dir: f32) -> Option<(Vec2, f32)> {
        let x = pos.x + dir;
        let edge = TANK_RADIUS;
        if !(edge..=WIDTH as f32 - edge).contains(&x) {
            return None;
        }
        // Under the dirt there is nowhere to drive to.
        if self.terrain.is_solid(pos - vec2(0.0, 8.0)) {
            return None;
        }
        let in_the_way = |(j, tank): (usize, &super::Tank)| {
            let gap = tank.pos.x - x;
            j != i && tank.alive && gap.abs() < TANK_GAP && gap * dir > 0.0
        };
        if self.tanks.iter().enumerate().any(in_the_way) {
            return None;
        }
        let ground = self.terrain.support_y(x, TANK_RADIUS);
        let rise = pos.y - ground;
        if rise > CLIMB || rise < -FALL_SAFE {
            return None;
        }
        Some((vec2(x, ground), LEVEL_COST + UPHILL_COST * rise.max(0.0)))
    }

    /// Tank `i` drives towards `dir` for `dt` seconds, as far as its fuel and the ground let it. Says whether it
    /// moved.
    /// A frame shorter than a px's worth of driving (on a fast screen) keeps the part for the next one.
    pub(super) fn drive(&mut self, i: usize, dir: f32, dt: f32, players: &mut [Player]) -> bool {
        let mut moved = false;
        self.tanks[i].drive_carry += DRIVE_SPEED * dt;
        let blocked = |battle: &Self| {
            players[i].items[FUEL] == 0 || battle.drive_step(i, battle.tanks[i].pos, dir).is_none()
        };
        if blocked(self) {
            self.tanks[i].drive_carry = 0.0;
            return false;
        }
        while self.tanks[i].drive_carry >= 1.0 {
            self.tanks[i].drive_carry -= 1.0;
            if players[i].items[FUEL] == 0 {
                break;
            }
            let Some((pos, cost)) = self.drive_step(i, self.tanks[i].pos, dir) else {
                break;
            };
            let tank = &mut self.tanks[i];
            tank.pos = pos;
            tank.fuel_debt += cost;
            let whole = tank.fuel_debt.floor();
            tank.fuel_debt -= whole;
            players[i].items[FUEL] = players[i].items[FUEL].saturating_sub(whole as u32);
            moved = true;
        }
        moved || self.tanks[i].drive_carry > 0.0
    }

    /// A bot's turn, before it aims: once a turn it looks whether its shots reach the tank it is after, and if
    /// not, for ground within reach of its fuel from where they do (`better_ground`). While it drives there this
    /// returns true and the aiming waits.
    pub(super) fn bot_drives(&mut self, i: usize, dt: f32, players: &mut [Player]) -> bool {
        if self.drive_checked != Some(self.turns) {
            self.drive_checked = Some(self.turns);
            self.drive_goal = self
                .better_ground(i, players)
                .or_else(|| self.dodge(i, players));
        }
        let Some(goal) = self.drive_goal else {
            return false;
        };
        let to_go = goal - self.tanks[i].pos.x;
        if to_go.abs() < 1.0 || !self.drive(i, to_go.signum(), dt, players) {
            self.drive_goal = None;
            return false;
        }
        true
    }

    /// Where tank `i` should drive to before it shoots: nowhere if a Baby Missile from where it stands lands
    /// within `REACH` of the tank it is after; else the place within `SEARCH` px and its fuel from where one lands
    /// closest, if that is closer by `GAIN` or more.
    pub(super) fn better_ground(&mut self, i: usize, players: &[Player]) -> Option<f32> {
        /// A shot landing this close to the target is good enough to stay, px.
        const REACH: f32 = 40.0;
        /// A place has to bring the shot this much closer to be worth the fuel, px.
        const GAIN: f32 = 30.0;
        /// How far it looks each way, and how far apart the places it tries are, px.
        const SEARCH: usize = 300;
        const EVERY: usize = 25;
        if players[i].items[FUEL] == 0 {
            return None;
        }
        let target = self.tanks[self.likely_target(i, players)?].center();
        let home = self.tanks[i].pos;
        let miss_from = |battle: &mut Self, pos: Vec2| {
            battle.tanks[i].pos = pos;
            let miss = battle
                .closest_shot(i, 0, target, 4..=176, false)
                .map_or(f32::MAX, |shot| shot.miss);
            battle.tanks[i].pos = home;
            miss
        };
        let here = miss_from(self, home);
        if here <= REACH {
            return None;
        }
        let mut best = (here - GAIN, None);
        for dir in [-1.0, 1.0] {
            let (mut pos, mut fuel) = (home, players[i].items[FUEL] as f32);
            for px in 1..=SEARCH {
                let Some((next, cost)) = self
                    .drive_step(i, pos, dir)
                    .filter(|&(_, cost)| cost <= fuel)
                else {
                    break;
                };
                (pos, fuel) = (next, fuel - cost);
                if px % EVERY == 0 {
                    let miss = miss_from(self, pos);
                    if miss < best.0 {
                        best = (miss, Some(pos.x));
                    }
                }
            }
        }
        best.1
    }

    /// Where tank `i` drives to so that whoever just hit it has to find the range again (`RERANGE`): up to `DODGE`
    /// px the way it gets furthest. Only when it was hit since its last shot and has little ranging of its own to
    /// lose by moving: no target yet, or a shot still about as rough as a first one.
    pub(super) fn dodge(&self, i: usize, players: &[Player]) -> Option<f32> {
        /// How far it drives, and the least that is worth starting for, px.
        const DODGE: usize = 90;
        const WORTH: f32 = 40.0;
        /// Its own aim counts as rough while its error is over this share of a first shot's most.
        const ROUGH: f32 = 0.3;
        let tank = &self.tanks[i];
        let profile = tank.role.or(players[i].bot)?;
        let hit = tank.revenge.is_some_and(|j| self.tanks[j].alive);
        let rough = tank
            .aim
            .is_none_or(|memory| memory.error.0.abs() > ROUGH * profile.first_angle_error);
        if !hit || !rough || players[i].items[FUEL] == 0 {
            return None;
        }
        let reach = |dir: f32| {
            let (mut pos, mut fuel) = (tank.pos, players[i].items[FUEL] as f32);
            for _ in 0..DODGE {
                let Some((next, cost)) = self
                    .drive_step(i, pos, dir)
                    .filter(|&(_, cost)| cost <= fuel)
                else {
                    break;
                };
                (pos, fuel) = (next, fuel - cost);
            }
            pos.x
        };
        let (left, right) = (reach(-1.0), reach(1.0));
        let goal = if tank.pos.x - left > right - tank.pos.x {
            left
        } else {
            right
        };
        ((goal - tank.pos.x).abs() >= WORTH).then_some(goal)
    }
}
