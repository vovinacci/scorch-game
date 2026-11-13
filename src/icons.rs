//! Small one-colour icons for the weapons and items, drawn from lines and dots: beside their names in the shop and
//! the inventory. No image files.

use macroquad::prelude::*;

use crate::items::{ITEMS, Steer, Use};
use crate::weapons::{Kind, WEAPONS};

/// Draws with `centre` as the origin and `unit` as half the icon's size: every shape below is given in a box
/// from -1 to 1, y down.
struct Pen {
    centre: Vec2,
    unit: f32,
    color: Color,
}

impl Pen {
    fn at(&self, x: f32, y: f32) -> Vec2 {
        self.centre + vec2(x, y) * self.unit
    }

    fn width(&self) -> f32 {
        (self.unit * 0.22).max(1.0)
    }

    fn dot(&self, x: f32, y: f32, r: f32) {
        let p = self.at(x, y);
        draw_circle(p.x, p.y, r * self.unit, self.color);
    }

    fn ring(&self, x: f32, y: f32, r: f32) {
        let p = self.at(x, y);
        draw_circle_lines(p.x, p.y, r * self.unit, self.width(), self.color);
    }

    fn line(&self, from: (f32, f32), to: (f32, f32)) {
        let (a, b) = (self.at(from.0, from.1), self.at(to.0, to.1));
        draw_line(a.x, a.y, b.x, b.y, self.width(), self.color);
    }

    fn triangle(&self, a: (f32, f32), b: (f32, f32), c: (f32, f32)) {
        draw_triangle(
            self.at(a.0, a.1),
            self.at(b.0, b.1),
            self.at(c.0, c.1),
            self.color,
        );
    }

    /// The upper half of a disc of radius `r` around (`x`, `y`).
    fn dome(&self, x: f32, y: f32, r: f32) {
        const SLICES: usize = 8;
        let rim = |k: usize| {
            let turn = std::f32::consts::PI * k as f32 / SLICES as f32;
            (x - r * turn.cos(), y - r * turn.sin())
        };
        for k in 0..SLICES {
            self.triangle((x, y), rim(k), rim(k + 1));
        }
    }

    /// An arrow from `from` to `to`.
    fn arrow(&self, from: (f32, f32), to: (f32, f32)) {
        self.line(from, to);
        let along = (vec2(to.0, to.1) - vec2(from.0, from.1)).normalize_or_zero();
        let across = vec2(-along.y, along.x);
        let back = vec2(to.0, to.1) - along * 0.7;
        let (left, right) = (back + across * 0.5, back - across * 0.5);
        self.triangle(to, (left.x, left.y), (right.x, right.y));
    }
}

/// The icon of weapon `w`, `size` px wide and high around `centre`.
pub fn weapon(w: usize, centre: Vec2, size: f32, color: Color) {
    let pen = Pen {
        centre,
        unit: size / 2.0,
        color,
    };
    match WEAPONS[w].kind {
        // A shell, bigger with its blast.
        Kind::Blast => {
            let r = (0.2 + WEAPONS[w].radius / 110.0).min(0.97);
            // The nukes: a ring around the shell.
            if WEAPONS[w].radius > 50.0 {
                pen.ring(0.0, 0.0, r);
                pen.dot(0.0, 0.0, r * 0.55);
            } else {
                pen.dot(0.0, 0.0, r);
            }
        }
        // Warheads fanning out.
        Kind::Mirv { warheads, .. } => {
            let n = if warheads > 5 { 5 } else { 3 };
            for k in 0..n {
                let x = -0.75 + 1.5 * k as f32 / (n - 1) as f32;
                pen.dot(x, 0.5 - 0.8 * (1.0 - x * x), 0.25);
            }
        }
        // A heap of dirt.
        Kind::Dirt => pen.dome(0.0, 0.7, 0.9),
        // A hole in the dirt.
        Kind::Riot => {
            pen.line((-1.0, 0.7), (1.0, 0.7));
            pen.ring(0.0, 0.0, 0.6);
        }
        // A ball on a slope.
        Kind::Roller => {
            pen.line((-1.0, 0.35), (1.0, 0.95));
            pen.dot(-0.15, 0.05, 0.5);
            pen.line((0.55, -0.35), (0.95, -0.2));
        }
        // Three hops.
        Kind::Leapfrog { .. } => {
            let path = [
                (-1.0, 0.7),
                (-0.65, -0.7),
                (-0.3, 0.7),
                (0.05, -0.3),
                (0.4, 0.7),
                (0.7, 0.1),
                (1.0, 0.7),
            ];
            for pair in path.windows(2) {
                pen.line(pair[0], pair[1]);
            }
        }
        // A bomb and its bomblets.
        Kind::Funky { .. } => {
            pen.dot(0.0, 0.0, 0.4);
            for (x, y) in [(-0.75, -0.6), (0.75, -0.6), (-0.75, 0.6), (0.75, 0.6)] {
                pen.dot(x, y, 0.2);
            }
        }
        // A flame.
        Kind::Napalm => {
            pen.dome(0.0, 0.35, 0.6);
            pen.triangle((-0.6, 0.35), (0.6, 0.35), (0.0, -1.0));
            pen.triangle((-0.6, 0.35), (0.6, 0.35), (0.0, 0.95));
        }
        // Down into the ground.
        Kind::Digger => {
            pen.line((-1.0, -0.55), (1.0, -0.55));
            pen.arrow((0.0, -0.55), (0.0, 1.0));
        }
        // A dotted path.
        Kind::Tracer => {
            for (x, y) in [(-0.8, 0.5), (-0.3, -0.2), (0.25, -0.45), (0.8, -0.1)] {
                pen.dot(x, y, 0.17);
            }
        }
    }
}

/// The icon of item `i`, `size` px wide and high around `centre`.
pub fn item(i: usize, centre: Vec2, size: f32, color: Color) {
    let pen = Pen {
        centre,
        unit: size / 2.0,
        color,
    };
    match ITEMS[i].kind {
        Use::Parachute => {
            pen.dome(0.0, -0.1, 0.9);
            pen.line((-0.85, -0.1), (0.0, 0.95));
            pen.line((0.85, -0.1), (0.0, 0.95));
        }
        // One ring, or two for the heavy one.
        Use::Shield { energy } => {
            pen.ring(0.0, 0.0, 0.9);
            if energy > 50 {
                pen.ring(0.0, 0.0, 0.5);
            }
        }
        // A ring that pushes outwards.
        Use::Deflector { .. } => {
            pen.ring(0.0, 0.0, 0.5);
            for (x, y) in [(0.0, -1.0), (0.0, 1.0), (-1.0, 0.0), (1.0, 0.0)] {
                pen.line((x * 0.7, y * 0.7), (x, y));
            }
        }
        Use::Battery { .. } => {
            let (corner, far) = (pen.at(-0.9, -0.5), pen.at(0.7, 0.5));
            draw_rectangle_lines(
                corner.x,
                corner.y,
                far.x - corner.x,
                far.y - corner.y,
                pen.width() * 2.0,
                color,
            );
            pen.line((0.85, -0.2), (0.85, 0.2));
            pen.line((-0.45, 0.0), (0.25, 0.0));
            pen.line((-0.1, -0.3), (-0.1, 0.3));
        }
        // Where the shell turns to.
        Use::Guidance(Steer::Heat) => {
            pen.ring(0.5, 0.5, 0.4);
            pen.arrow((-0.9, -0.9), (0.15, 0.15));
        }
        Use::Guidance(Steer::Horizontal) => pen.arrow((-0.95, 0.0), (0.95, 0.0)),
        Use::Guidance(Steer::Vertical) => pen.arrow((0.0, -0.95), (0.0, 0.95)),
    }
}
