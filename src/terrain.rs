//! Destructible terrain stored as a heightmap: one surface height per pixel column.
//!
//! A heightmap cannot hold floating dirt, which is exactly what we want:
//! when an explosion removes dirt from a column, everything above falls down.

use macroquad::prelude::*;
use macroquad::rand::gen_range;
use serde::{Deserialize, Serialize};

use crate::choice::Choice;
use crate::world::TerrainKind;

pub const WIDTH: usize = 1280;
pub const HEIGHT: f32 = 720.0;

#[derive(Serialize, Deserialize)]
pub struct Terrain {
    /// Screen y of the ground surface for each column (bigger = lower).
    ground: Vec<f32>,
}

impl Terrain {
    pub fn generate(kind: TerrainKind) -> Self {
        let kind = match kind {
            TerrainKind::Random => TerrainKind::ALL[gen_range(0, TerrainKind::ALL.len() - 1)],
            kind => kind,
        };
        // (base height, wave amplitude scale, highest allowed ground)
        let (base, amp, top) = match kind {
            TerrainKind::Mountains => (480.0, 2.2, 110.0),
            TerrainKind::Flat => (540.0, 0.15, 400.0),
            _ => (470.0, 1.0, 180.0),
        };
        // A sum of a few sine waves with random frequency and phase.
        let waves: Vec<(f32, f32, f32)> = (1..=4)
            .map(|i| {
                let i = i as f32;
                (
                    gen_range(30.0, 110.0) * amp / i,
                    gen_range(0.002, 0.005) * i,
                    gen_range(0.0, std::f32::consts::TAU),
                )
            })
            .collect();
        let canyon_x = gen_range(0.35, 0.65) * WIDTH as f32;
        let ground = (0..WIDTH)
            .map(|x| {
                let x = x as f32;
                let mut h: f32 = waves
                    .iter()
                    .map(|(amp, freq, phase)| amp * (x * freq + phase).sin())
                    .sum();
                if kind == TerrainKind::Canyon {
                    // A deep valley somewhere in the middle.
                    h += 230.0 * (-((x - canyon_x) / 110.0).powi(2)).exp();
                    h -= 60.0;
                }
                (base + h).clamp(top, HEIGHT - 30.0)
            })
            .collect();
        Self { ground }
    }

    pub fn height_at(&self, x: f32) -> f32 {
        match self.column(x) {
            Some(c) => self.ground[c],
            None => HEIGHT,
        }
    }

    pub fn is_solid(&self, p: Vec2) -> bool {
        self.column(p.x).is_some_and(|c| p.y >= self.ground[c])
    }

    /// Highest ground point under a tank `half` pixels wide each side.
    pub fn support_y(&self, x: f32, half: f32) -> f32 {
        let from = (x - half).max(0.0) as usize;
        let to = ((x + half) as usize).min(WIDTH - 1);
        self.ground[from..=to]
            .iter()
            .copied()
            .fold(HEIGHT, f32::min)
    }

    /// Flattens a small pad so tanks start on level ground.
    pub fn flatten(&mut self, x: f32, half: f32) {
        let level = self.height_at(x);
        let from = (x - half).max(0.0) as usize;
        let to = ((x + half) as usize).min(WIDTH - 1);
        for g in &mut self.ground[from..=to] {
            *g = level;
        }
    }

    /// Removes a circle of dirt; dirt above the hole falls into it.
    pub fn carve(&mut self, center: Vec2, radius: f32) {
        self.for_columns(center, radius, |ground, top, bottom| {
            let removed = (bottom.min(HEIGHT) - top.max(*ground)).max(0.0);
            *ground = (*ground + removed).min(HEIGHT);
        });
    }

    /// Drops a circle's worth of dirt onto each column.
    pub fn add_dirt(&mut self, center: Vec2, radius: f32) {
        self.for_columns(center, radius, |ground, top, bottom| {
            *ground = (*ground - (bottom - top)).max(20.0);
        });
    }

    fn for_columns(&mut self, center: Vec2, radius: f32, mut f: impl FnMut(&mut f32, f32, f32)) {
        let from = (center.x - radius).floor().max(0.0) as usize;
        let to = ((center.x + radius).ceil() as usize).min(WIDTH - 1);
        for x in from..=to {
            let dx = x as f32 - center.x;
            if dx.abs() > radius {
                continue;
            }
            let dy = (radius * radius - dx * dx).sqrt();
            f(&mut self.ground[x], center.y - dy, center.y + dy);
        }
    }

    fn column(&self, x: f32) -> Option<usize> {
        (x >= 0.0 && x < WIDTH as f32).then_some(x as usize)
    }

    pub fn draw(&self) {
        let dirt = Color::from_rgba(120, 85, 50, 255);
        let grass = Color::from_rgba(90, 160, 70, 255);
        for (x, &g) in self.ground.iter().enumerate() {
            let x = x as f32;
            draw_rectangle(x, g, 1.0, HEIGHT - g, dirt);
            draw_rectangle(x, g, 1.0, 3.0, grass);
        }
    }
}
