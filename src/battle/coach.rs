//! Aiming help: the coaching advice after a miss.

use macroquad::prelude::*;

use super::shells::simulate;
use super::{
    BARREL, Battle, COACH_LITTLE, COACH_REACH, COACH_SOME, CoachShot, MAX_POWER, direction,
    launch_velocity,
};
use crate::i18n::t;
use crate::player::Player;
use crate::terrain::WIDTH;
use crate::world::Walls;

impl Battle {
    /// Signed sideways distance from `a` to `b`: the short way round with wrap-around walls.
    pub(super) fn across(&self, a: f32, b: f32) -> f32 {
        let d = b - a;
        let w = WIDTH as f32;
        if self.world().walls == Walls::Wrap && d.abs() > w / 2.0 {
            d - w * d.signum()
        } else {
            d
        }
    }

    /// Aiming help, after a human's missed shot: which enemy it was meant for (the one nearest to where it came down;
    /// off the field, the farthest that way). The advice itself waits for the shooter's next turn (`advise`).
    pub(super) fn coach(&mut self, shot: CoachShot) {
        let me = self.current;
        let landed = shot
            .landing
            .map_or(shot.from + shot.dir * 2.0 * WIDTH as f32, |p| p.x);
        let target = self
            .tanks
            .iter()
            .enumerate()
            .filter(|&(i, t)| i != me && t.alive && !t.fled)
            .min_by(|a, b| {
                let da = self.across(a.1.pos.x, landed).abs();
                let db = self.across(b.1.pos.x, landed).abs();
                da.total_cmp(&db)
            })
            .map(|(i, _)| i);
        self.tanks[me].coach_target = target;
    }

    /// Of all powers this tank can use at `angle`, the one whose shot comes down nearest to `goal`, and how near.
    pub(super) fn best_power(&self, me: usize, angle: f32, goal: Vec2) -> (f32, f32) {
        let tank = &self.tanks[me];
        let deflectors = self.deflectors(me);
        let muzzle = tank.barrel_base() + direction(angle) * BARREL;
        let mut best = (f32::MAX, tank.power);
        for power in (0..=self.max_power(tank) as i32)
            .step_by(5)
            .map(|p| p as f32)
        {
            let vel = launch_velocity(angle, power);
            if let Some(l) = simulate(
                muzzle,
                vel,
                self.wind,
                self.world(),
                &self.terrain,
                &deflectors,
            ) {
                let d = self.across(l.pos.x, goal.x).abs();
                if d < best.0 {
                    best = (d, power);
                }
            }
        }
        best
    }

    /// Aiming help at the start of a human's turn after a miss: from this tank's current aim and this turn's wind,
    /// which way to change the power to reach `target` and roughly how much; or which way to turn the barrel if this
    /// angle can't reach it; or that no aim within this tank's power can.
    pub(super) fn advise(&self, me: usize, target: usize, players: &[Player]) -> String {
        let tank = &self.tanks[me];
        let goal = self.tanks[target].pos;
        let name = players[target].label();
        let (miss, power) = self.best_power(me, tank.angle, goal);
        if miss > COACH_REACH {
            // Not at this angle. Can any angle reach it, and is it steeper or flatter than now?
            let reach = (2..=34)
                .map(|k| k as f32 * 5.0)
                .filter(|&a| self.best_power(me, a, goal).0 <= COACH_REACH)
                .min_by(|a, b| (a - tank.angle).abs().total_cmp(&(b - tank.angle).abs()));
            return match reach {
                Some(angle) if (90.0 - angle).abs() < (90.0 - tank.angle).abs() => {
                    t!("coach-aim-higher", name = name)
                }
                Some(_) => t!("coach-aim-lower", name = name),
                // Too far for this tank's power: with "Health limits power", that's the damage.
                None if self.max_power(tank) < MAX_POWER => {
                    t!("coach-out-of-reach-health", name = name)
                }
                None => t!("coach-out-of-reach", name = name),
            };
        }
        let change = power - tank.power;
        let key = match (change.abs(), change > 0.0) {
            (d, _) if d < 8.0 => "coach-power-right",
            (d, true) if d <= COACH_LITTLE => "coach-more-little",
            (d, true) if d <= COACH_SOME => "coach-more",
            (_, true) => "coach-more-lot",
            (d, false) if d <= COACH_LITTLE => "coach-less-little",
            (d, false) if d <= COACH_SOME => "coach-less",
            (_, false) => "coach-less-lot",
        };
        t!(key, name = name)
    }
}
