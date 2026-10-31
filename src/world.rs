//! World settings chosen on the setup screen: terrain, wind, gravity, walls, sky.
//! Also `step_shell`, the one function that moves a shell, shared by real
//! shots and bot aiming so bots "see" exactly the same physics.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use serde::{Deserialize, Serialize};

use crate::choice::choice;
use crate::terrain::{HEIGHT, WIDTH};

choice!(TerrainKind {
    Rolling,
    Mountains,
    Flat,
    Canyon,
    Random,
});

choice!(WindLevel {
    Off,
    Calm,
    Normal,
    Strong,
});

choice!(GravityLevel { Low, Normal, High });

choice!(Walls {
    Open,
    Solid,
    Bouncy,
    Wrap,
});

choice!(Sky { Night, Sunset, Day });

/// Fraction of horizontal speed kept when a shell bounces off a Bouncy wall.
const BOUNCE: f32 = 0.8;
/// With Open walls, a shell this far past the screen edge is gone for good.
const OPEN_MARGIN: f32 = 200.0;

#[derive(Clone, Copy, Serialize, Deserialize)]
#[serde(default)]
pub struct World {
    pub terrain: TerrainKind,
    pub wind: WindLevel,
    /// Wind shifts a little every turn.
    pub wind_changes: bool,
    pub gravity: GravityLevel,
    pub walls: Walls,
    pub sky: Sky,
}

impl Default for World {
    fn default() -> Self {
        Self {
            terrain: TerrainKind::Rolling,
            wind: WindLevel::Normal,
            wind_changes: true,
            gravity: GravityLevel::Normal,
            walls: Walls::Open,
            sky: Sky::Night,
        }
    }
}

impl World {
    /// Downward acceleration in pixels per second squared.
    pub fn gravity(&self) -> f32 {
        match self.gravity {
            GravityLevel::Low => 180.0,
            GravityLevel::Normal => 300.0,
            GravityLevel::High => 450.0,
        }
    }

    /// Strongest possible wind (sideways acceleration, pixels per second squared).
    pub fn max_wind(&self) -> f32 {
        match self.wind {
            WindLevel::Off => 0.0,
            WindLevel::Calm => 20.0,
            WindLevel::Normal => 40.0,
            WindLevel::Strong => 70.0,
        }
    }

    pub fn first_wind(&self) -> f32 {
        let max = self.max_wind() * 0.6;
        if max == 0.0 {
            0.0
        } else {
            gen_range(-max, max)
        }
    }

    /// Wind for the next turn: drifts by up to 35% of the maximum, if it changes at all.
    pub fn next_wind(&self, wind: f32) -> f32 {
        let max = self.max_wind();
        if !self.wind_changes || max == 0.0 {
            return wind;
        }
        let drift = max * 0.35;
        (wind + gen_range(-drift, drift)).clamp(-max, max)
    }
}

pub enum Flight {
    Flying,
    /// Hit a Solid wall: explodes there.
    Wall,
    /// Left the world: no explosion.
    Lost,
}

/// Moves a shell one physics step of `dt` seconds and applies the walls.
pub fn step_shell(pos: &mut Vec2, vel: &mut Vec2, wind: f32, world: &World, dt: f32) -> Flight {
    *vel += vec2(wind, world.gravity()) * dt;
    *pos += *vel * dt;
    if pos.y > HEIGHT {
        return Flight::Lost;
    }
    let w = WIDTH as f32;
    match world.walls {
        Walls::Open => {
            if pos.x < -OPEN_MARGIN || pos.x > w + OPEN_MARGIN {
                return Flight::Lost;
            }
        }
        Walls::Solid => {
            if pos.x < 0.0 || pos.x >= w {
                pos.x = pos.x.clamp(0.0, w - 1.0);
                return Flight::Wall;
            }
        }
        Walls::Bouncy => {
            if pos.x < 0.0 {
                pos.x = -pos.x;
                vel.x = -vel.x * BOUNCE;
            } else if pos.x >= w {
                pos.x = 2.0 * (w - 1.0) - pos.x;
                vel.x = -vel.x * BOUNCE;
            }
        }
        Walls::Wrap => pos.x = pos.x.rem_euclid(w),
    }
    Flight::Flying
}

impl Sky {
    /// Fills the whole virtual screen with a vertical gradient.
    pub fn draw(self) {
        self.draw_in(Rect::new(0.0, 0.0, WIDTH as f32, HEIGHT));
    }

    /// Fills `r` with the sky gradient.
    pub fn draw_in(self, r: Rect) {
        let (top, bottom) = match self {
            Sky::Night => (
                Color::new(0.05, 0.05, 0.15, 1.0),
                Color::new(0.30, 0.15, 0.35, 1.0),
            ),
            Sky::Sunset => (
                Color::new(0.20, 0.12, 0.35, 1.0),
                Color::new(0.95, 0.55, 0.30, 1.0),
            ),
            Sky::Day => (
                Color::new(0.30, 0.55, 0.90, 1.0),
                Color::new(0.75, 0.88, 1.00, 1.0),
            ),
        };
        let bands = (r.h / 3.0).max(1.0) as usize;
        let h = r.h / bands as f32;
        for i in 0..bands {
            let t = i as f32 / bands as f32;
            let c = Color::new(
                top.r + (bottom.r - top.r) * t,
                top.g + (bottom.g - top.g) * t,
                top.b + (bottom.b - top.b) * t,
                1.0,
            );
            draw_rectangle(
                r.x,
                r.y + i as f32 * h,
                r.w,
                h.min(r.h - i as f32 * h) + 0.5,
                c,
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn shell_at_right_edge(walls: Walls) -> (Flight, Vec2, Vec2) {
        let world = World {
            walls,
            ..World::default()
        };
        let mut pos = vec2(WIDTH as f32 - 1.0, 300.0);
        let mut vel = vec2(240.0, 0.0);
        let flight = step_shell(&mut pos, &mut vel, 0.0, &world, 0.1);
        (flight, pos, vel)
    }

    #[test]
    fn walls_behave() {
        let (flight, pos, vel) = shell_at_right_edge(Walls::Bouncy);
        assert!(matches!(flight, Flight::Flying));
        assert!(pos.x < WIDTH as f32 && vel.x < 0.0, "bounced back");

        let (flight, pos, _) = shell_at_right_edge(Walls::Wrap);
        assert!(matches!(flight, Flight::Flying));
        assert!(pos.x < 100.0, "came out on the left: {}", pos.x);

        let (flight, _, _) = shell_at_right_edge(Walls::Solid);
        assert!(matches!(flight, Flight::Wall));

        let (flight, _, _) = shell_at_right_edge(Walls::Open);
        assert!(
            matches!(flight, Flight::Flying),
            "open: still flying past the edge"
        );
    }

    #[test]
    fn no_wind_stays_still() {
        let world = World {
            wind: WindLevel::Off,
            ..World::default()
        };
        assert_eq!(world.first_wind(), 0.0);
        assert_eq!(world.next_wind(0.0), 0.0);
    }
}
