//! Guidance: items that steer a fired shell once, on its way.
//!   Heat       - turns onto the first enemy tank it comes close to.
//!   Horizontal - at the height of the tank picked, turns and flies level towards it.
//!   Vertical   - above the tank picked, turns and drops straight down.
//!
//! A turned shell flies in a straight line, whatever the wind and gravity; the ground still stops it. A Mag
//! Deflector's field switches guidance off.

use macroquad::prelude::*;

use crate::items::Steer;

/// A heat-seeking shell turns onto an enemy tank this close, px.
pub(super) const HEAT_RANGE: f32 = 100.0;
/// A turned shell flies at least this fast, px/s: one that was nearly hanging at the top of its arc still gets
/// there.
const MIN_SPEED: f32 = 250.0;

#[derive(Clone, Debug, PartialEq)]
pub(super) struct Guide {
    steer: Steer,
    /// Heat: every living enemy tank's centre. Horizontal and Vertical: the centre of the one picked.
    targets: Vec<Vec2>,
    turned: Option<Turned>,
}

/// What a shell does once guidance has turned it.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Turned {
    /// Flies at this point, at this speed.
    Onto(Vec2, f32),
    /// Flies on with this velocity.
    Course(Vec2),
}

impl Guide {
    pub(super) fn new(steer: Steer, targets: Vec<Vec2>) -> Self {
        Self {
            steer,
            targets,
            turned: None,
        }
    }

    /// Call after every physics step of a flying shell that was at `from` before it: turns the shell when its
    /// moment comes, and from then on keeps it on its line.
    pub(super) fn steer(&mut self, from: Vec2, pos: &mut Vec2, vel: &mut Vec2) {
        if self.turned.is_none() {
            self.turned = self.turn(from, pos, vel.length().max(MIN_SPEED));
        }
        match self.turned {
            Some(Turned::Onto(target, speed)) => {
                if let Some(towards) = (target - *pos).try_normalize() {
                    *vel = towards * speed;
                }
            }
            Some(Turned::Course(course)) => *vel = course,
            None => {}
        }
    }

    /// Is this step the moment to turn?
    fn turn(&self, from: Vec2, pos: &mut Vec2, speed: f32) -> Option<Turned> {
        // A step across a Wrap wall is not a step past anything.
        let stepped = pos.distance(from) < 100.0;
        match self.steer {
            Steer::Heat => self
                .targets
                .iter()
                .filter(|t| t.distance(*pos) < HEAT_RANGE)
                .min_by(|a, b| a.distance(*pos).total_cmp(&b.distance(*pos)))
                .map(|&t| Turned::Onto(t, speed)),
            Steer::Vertical => {
                let target = *self.targets.first()?;
                let over = (from.x - target.x) * (pos.x - target.x) <= 0.0 && pos.y < target.y;
                (stepped && over).then(|| {
                    pos.x = target.x;
                    Turned::Course(vec2(0.0, speed))
                })
            }
            Steer::Horizontal => {
                let target = *self.targets.first()?;
                let level = (from.y - target.y) * (pos.y - target.y) <= 0.0;
                let side = (target.x - pos.x).signum();
                (stepped && level).then(|| {
                    pos.y = target.y;
                    Turned::Course(vec2(side * speed, 0.0))
                })
            }
        }
    }
}
