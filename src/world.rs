//! World settings chosen on the setup screen: terrain, wind, gravity, walls, sky.
//! Also `step_shell`, the one function that moves a shell, shared by real
//! shots and bot aiming so bots "see" exactly the same physics.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use serde::{Deserialize, Serialize};

use crate::choice::{Choice, choice};
use crate::terrain::{HEIGHT, WIDTH};

choice!(TerrainKind {
    Rolling,
    Mountains,
    Flat,
    Canyon,
    Random,
});

// `Random` (here and in the choices below) is picked anew for every round: see `World::for_round`.

choice!(WindLevel {
    Off,
    Calm,
    Normal,
    Strong,
    Random,
});

choice!(GravityLevel {
    Low,
    Normal,
    High,
    Random,
});

choice!(Walls {
    Open,
    Solid,
    Bouncy,
    Wrap,
    Random,
});

choice!(Sky {
    Night,
    Sunset,
    Day,
    Random,
});

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
    /// This round's world: every `Random` setting replaced by a real value.
    pub fn for_round(self) -> World {
        World {
            terrain: self.terrain.roll(),
            wind: self.wind.roll(),
            gravity: self.gravity.roll(),
            walls: self.walls.roll(),
            sky: self.sky.roll(),
            ..self
        }
    }

    /// Downward acceleration in pixels per second squared.
    pub fn gravity(self) -> f32 {
        match self.gravity {
            GravityLevel::Low => 180.0,
            // Random is resolved before a round starts (`for_round`).
            GravityLevel::Normal | GravityLevel::Random => 300.0,
            GravityLevel::High => 450.0,
        }
    }

    /// Strongest possible wind (sideways acceleration, pixels per second squared).
    pub fn max_wind(self) -> f32 {
        match self.wind {
            WindLevel::Off => 0.0,
            WindLevel::Calm => 20.0,
            WindLevel::Normal | WindLevel::Random => 40.0,
            WindLevel::Strong => 70.0,
        }
    }

    pub fn first_wind(self) -> f32 {
        let max = self.max_wind() * 0.6;
        if max == 0.0 {
            0.0
        } else {
            gen_range(-max, max)
        }
    }

    /// Wind for the next turn: drifts by up to 35% of the maximum, if it changes at all.
    pub fn next_wind(self, wind: f32) -> f32 {
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
pub fn step_shell(pos: &mut Vec2, vel: &mut Vec2, wind: f32, world: World, dt: f32) -> Flight {
    *vel += vec2(wind, world.gravity()) * dt;
    *pos += *vel * dt;
    if pos.y > HEIGHT {
        return Flight::Lost;
    }
    let w = WIDTH as f32;
    match world.walls {
        Walls::Open | Walls::Random => {
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
            // Random shows as night in the setup preview; a round always has a real sky.
            Sky::Night | Sky::Random => (
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
        self.draw_details(r, top, bottom);
    }

    /// Stars, the sun, clouds and far hills, all still and faint, drawn in screen coordinates squeezed into `r`.
    fn draw_details(self, area: Rect, top: Color, bottom: Color) {
        let scale = area.w / WIDTH as f32;
        let at = |x: f32, y: f32| vec2(area.x + x * scale, area.y + y * area.h / HEIGHT);
        // The sky's own colour at screen height `y`, blended `amount` of the way towards `to`.
        let sky = |y: f32, to: Color, amount: f32| {
            let frac = (y / HEIGHT).clamp(0.0, 1.0);
            let blend = |a: f32, b: f32, o: f32| {
                let v = a + (b - a) * frac;
                v + (o - v) * amount
            };
            Color::new(
                blend(top.r, bottom.r, to.r),
                blend(top.g, bottom.g, to.g),
                blend(top.b, bottom.b, to.b),
                1.0,
            )
        };
        match self {
            Sky::Night | Sky::Random => {
                for i in 0..140 {
                    let (x, y) = (
                        hash(i, 1) * WIDTH as f32,
                        40.0 + hash(i, 2).powf(1.6) * 420.0,
                    );
                    // Fewer and fainter stars towards the horizon.
                    let a = (0.25 + 0.55 * hash(i, 3)) * (1.0 - y / 520.0);
                    let size = if hash(i, 4) > 0.9 { 2.0 } else { 1.0 };
                    let p = at(x, y);
                    draw_rectangle(
                        p.x,
                        p.y,
                        size * scale.max(0.5),
                        size * scale.max(0.5),
                        Color::new(1.0, 1.0, 0.95, a),
                    );
                }
            }
            Sky::Sunset => {
                // A big low sun, partly behind the far hills, with a soft glow.
                draw_sun(
                    at(900.0, 455.0),
                    40.0 * scale,
                    Color::new(1.0, 0.8, 0.5, 1.0),
                    0.03,
                );
            }
            Sky::Day => {
                draw_sun(
                    // Left of the turn panel in the top right corner.
                    at(820.0, 120.0),
                    26.0 * scale,
                    Color::new(1.0, 0.98, 0.86, 1.0),
                    0.02,
                );
                for (cx, cy, w) in [
                    (230.0, 150.0, 1.0),
                    (520.0, 240.0, 0.8),
                    (1040.0, 260.0, 0.9),
                    (380.0, 330.0, 0.6),
                ] {
                    draw_cloud(at, scale, cx, cy, w);
                }
            }
        }
        // Two rows of far hills in the colours of the sky: scenery only, nothing hits them.
        for (row, (base, height, shade)) in [(440.0, 70.0, 0.08), (500.0, 55.0, 0.14)]
            .into_iter()
            .enumerate()
        {
            // A touch darker than the sky behind the hilltops, so they stay in the distance.
            let c = sky(base - height, BLACK, shade);
            let phase = row as f32 * 2.7;
            let step = 4.0;
            let mut x = 0.0;
            while x < WIDTH as f32 {
                let y = base
                    - height
                        * (0.5
                            + 0.3 * (x * 0.004 + phase).sin()
                            + 0.2 * (x * 0.011 + phase * 1.7).sin());
                let p = at(x, y);
                let bottom_edge = at(x, HEIGHT);
                draw_rectangle(p.x, p.y, step * scale + 0.5, bottom_edge.y - p.y, c);
                x += step;
            }
        }
    }
}

/// A thin, wispy cloud centred at (`x`, `y`), `size` times the full size: a few long, flat streaks, each fading out
/// towards its edges.
fn draw_cloud(at: impl Fn(f32, f32) -> Vec2, scale: f32, x: f32, y: f32, size: f32) {
    // (offset x, offset y, half length, half height)
    let streaks = [
        (0.0, 0.0, 110.0, 9.0),
        (-50.0, 7.0, 70.0, 6.0),
        (60.0, -6.0, 80.0, 6.0),
        (20.0, 12.0, 50.0, 4.0),
    ];
    for (dx, dy, w, h) in streaks {
        let c = at(x + dx * size, y + dy * size);
        for k in 0..6 {
            let shrink = 1.0 - k as f32 * 0.14;
            draw_ellipse(
                c.x,
                c.y,
                w * size * scale * shrink,
                h * size * scale * shrink,
                0.0,
                Color::new(1.0, 1.0, 1.0, 0.05),
            );
        }
    }
}

/// A sun of `radius` at `p` in `color`, with a glow fading out over twice its size in many faint steps.
fn draw_sun(p: Vec2, radius: f32, color: Color, glow: f32) {
    for k in (1..=16).rev() {
        let c = Color { a: glow, ..color };
        draw_circle(p.x, p.y, radius * (1.0 + k as f32 / 8.0), c);
    }
    draw_circle(p.x, p.y, radius, color);
}

/// A fixed pseudo-random number in 0..1 for (`i`, `salt`), so the sky looks the same every frame.
fn hash(i: u32, salt: u32) -> f32 {
    let mut n = i.wrapping_mul(0x9E37_79B1) ^ salt.wrapping_mul(0x85EB_CA77);
    n ^= n >> 15;
    n = n.wrapping_mul(0x2C1B_3C6D);
    n ^= n >> 12;
    (n & 0xFFFF) as f32 / 65_536.0
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
        let flight = step_shell(&mut pos, &mut vel, 0.0, world, 0.1);
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
