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
    /// How the ground looks (the terrain type it was made as; older saves: rolling hills).
    #[serde(default)]
    look: Look,
    /// Burnt earth around recent craters, fading turn by turn.
    #[serde(default)]
    scorch: Vec<Scorch>,
}

/// A darker, burnt rim around a crater: `heat` 1 when fresh, fading towards 0.
#[derive(Clone, Copy, Debug, PartialEq, Serialize, Deserialize)]
struct Scorch {
    x: f32,
    y: f32,
    radius: f32,
    heat: f32,
}

/// The colour burnt earth goes towards.
const CHARRED: [f32; 3] = [38.0, 30.0, 26.0];
/// Burn marks kept at most; the oldest go first.
const SCORCH_MAX: usize = 60;
/// How much of its heat a burn mark keeps each turn, and below which heat it's gone (after 6 turns).
const SCORCH_KEEP: f32 = 0.7;
const SCORCH_GONE: f32 = 0.15;

/// The ground's colours, by terrain type.
#[derive(Clone, Copy, Default, PartialEq, Eq, Debug, Serialize, Deserialize)]
pub enum Look {
    #[default]
    Rolling,
    Mountains,
    Flat,
    Canyon,
}

/// A small, fixed variation per pixel (-1..1), so large areas of earth don't look flat.
fn grain(x: usize, y: usize) -> f32 {
    let mut n = (x as u32).wrapping_mul(0x9E37_79B1) ^ (y as u32).wrapping_mul(0x85EB_CA77);
    n ^= n >> 15;
    n = n.wrapping_mul(0x2C1B_3C6D);
    n ^= n >> 12;
    (n & 0xFFFF) as f32 / 32_768.0 - 1.0
}

/// Sandstone strata at (`x`, `y`): a brightness factor for gently wavy, slightly tilted rock layers of uneven
/// thickness, low contrast so they read as rock, not stripes.
fn strata(x: f32, y: f32) -> f32 {
    let wave = 7.0 * (x * 0.006 + 1.3).sin() + 3.0 * (x * 0.017).sin() + 0.015 * x;
    let layer = ((y + wave) / 9.0).floor() as i32;
    // Layers are 9 px slices; neighbouring slices often share a shade, so layers come out 9 to 40 px thick.
    let pick = grain(layer.rem_euclid(1 << 16) as usize, 7) * 0.5 + 0.5;
    [0.93, 1.0, 1.0, 1.05, 0.97][((pick * 5.0) as usize).min(4)]
}

/// Blends `a` to `b` by `t` (0..1).
fn mix(a: [f32; 3], b: [f32; 3], t: f32) -> [f32; 3] {
    let t = t.clamp(0.0, 1.0);
    [
        a[0] + (b[0] - a[0]) * t,
        a[1] + (b[1] - a[1]) * t,
        a[2] + (b[2] - a[2]) * t,
    ]
}

impl Look {
    /// (grass, topsoil, deep earth) in 0..255.
    fn palette(self) -> ([f32; 3], [f32; 3], [f32; 3]) {
        match self {
            Look::Rolling => ([90.0, 160.0, 70.0], [128.0, 92.0, 56.0], [78.0, 56.0, 36.0]),
            Look::Flat => (
                [104.0, 170.0, 74.0],
                [112.0, 86.0, 54.0],
                [70.0, 54.0, 36.0],
            ),
            Look::Mountains => (
                [112.0, 138.0, 92.0],
                [124.0, 114.0, 104.0],
                [74.0, 70.0, 66.0],
            ),
            // No grass in the canyon: a sun-bleached sandstone rim instead.
            Look::Canyon => (
                [214.0, 156.0, 106.0],
                [196.0, 120.0, 76.0],
                [126.0, 70.0, 48.0],
            ),
        }
    }

    /// The colour of the ground at screen (`x`, `y`) where the surface is at `surface`, `steep` if the slope there
    /// is too steep for grass.
    fn color(self, x: usize, y: usize, surface: f32, steep: bool) -> [u8; 4] {
        let (grass, topsoil, deep) = self.palette();
        let (fx, fy) = (x as f32, y as f32);
        let depth = fy - surface;
        // Snow caps the mountain peaks: up to 22 px deep, thinning out towards a gently wavy snow line.
        let line = SNOW_LINE + 8.0 * (fx * 0.013).sin() + 4.0 * (fx * 0.037 + 2.0).sin();
        let snow = self == Look::Mountains
            && depth < ((line - surface) * 0.4).min(22.0) + 2.0 * (fx * 0.09).sin();
        let mut c = if snow {
            [236.0, 240.0, 245.0]
        } else if depth < 3.0 && !steep {
            grass
        } else {
            // Darker further down the screen. Going by screen height, not depth, keeps cliffs and craters from
            // casting shadows into the earth; only a thin band under the surface is lighter.
            let mut c = mix(topsoil, deep, (fy - 380.0) / 340.0);
            c = mix(c, topsoil, 0.5 * (1.0 - depth / 14.0));
            if self == Look::Canyon {
                let shade = strata(fx, fy);
                c = [c[0] * shade, c[1] * shade, c[2] * shade];
            }
            c
        };
        // A just visible grain.
        let g = 1.0 + 0.04 * grain(x, y);
        c = [c[0] * g, c[1] * g, c[2] * g];
        [
            c[0].clamp(0.0, 255.0) as u8,
            c[1].clamp(0.0, 255.0) as u8,
            c[2].clamp(0.0, 255.0) as u8,
            255,
        ]
    }

    fn of(kind: TerrainKind) -> Self {
        match kind {
            TerrainKind::Mountains => Look::Mountains,
            TerrainKind::Flat => Look::Flat,
            TerrainKind::Canyon => Look::Canyon,
            TerrainKind::Rolling | TerrainKind::Random => Look::Rolling,
        }
    }
}

/// Mountain ground higher than this (smaller screen y) gets snow.
const SNOW_LINE: f32 = 290.0;

impl Terrain {
    fn with(ground: Vec<f32>, look: Look) -> Self {
        Self {
            ground,
            look,
            scorch: Vec::new(),
        }
    }

    pub fn generate(kind: TerrainKind) -> Self {
        let kind = kind.roll();
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
        Self::with(ground, Look::of(kind))
    }

    /// Terrain from a height function, for tests.
    #[cfg(test)]
    pub fn from_fn(height: impl Fn(f32) -> f32) -> Self {
        Self::with(
            (0..WIDTH).map(|x| height(x as f32)).collect(),
            Look::Rolling,
        )
    }

    pub fn height_at(&self, x: f32) -> f32 {
        match Self::column(x) {
            Some(c) => self.ground[c],
            None => HEIGHT,
        }
    }

    pub fn is_solid(&self, p: Vec2) -> bool {
        Self::column(p.x).is_some_and(|c| p.y >= self.ground[c])
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

    /// Leaves a burnt rim around a blast at `center`.
    pub fn burn(&mut self, center: Vec2, radius: f32) {
        // Burning the same spot again (napalm drops running together) makes it fresh, not a new mark.
        if let Some(s) = self
            .scorch
            .iter_mut()
            .find(|s| (s.radius - radius).abs() < 1.0 && vec2(s.x, s.y).distance(center) < radius)
        {
            s.heat = 1.0;
            return;
        }
        if self.scorch.len() >= SCORCH_MAX {
            self.scorch.remove(0);
        }
        self.scorch.push(Scorch {
            x: center.x,
            y: center.y,
            radius,
            heat: 1.0,
        });
    }

    /// How many burn marks there are.
    #[cfg(test)]
    pub fn burn_marks(&self) -> usize {
        self.scorch.len()
    }

    /// A turn went by: burn marks fade, and the faintest go away.
    pub fn cool(&mut self) {
        if self.scorch.is_empty() {
            return;
        }
        for s in &mut self.scorch {
            s.heat *= SCORCH_KEEP;
        }
        self.scorch.retain(|s| s.heat >= SCORCH_GONE);
    }

    /// Lowers the ground by `depth` in every column from `x0` to `x1` (either order), like dirt sinking into a
    /// tunnel dug underneath.
    pub fn sink(&mut self, x0: f32, x1: f32, depth: f32) {
        let (a, b) = (x0.min(x1), x0.max(x1));
        let from = a.round().max(0.0) as usize;
        let to = (b.round() as usize).min(WIDTH - 1);
        if from > to {
            return;
        }
        for g in &mut self.ground[from..to] {
            *g = (*g + depth).min(HEIGHT);
        }
    }

    /// Drops a circle's worth of dirt onto each column.
    pub fn add_dirt(&mut self, center: Vec2, radius: f32) {
        // Fresh dirt covers the burn marks under it.
        self.scorch.retain(|s| (s.x - center.x).abs() >= radius);
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

    fn column(x: f32) -> Option<usize> {
        (x >= 0.0 && x < WIDTH as f32).then_some(x as usize)
    }

    /// Draws the ground: painted into a picture once; after that only the columns that changed are painted again.
    pub fn draw(&self) {
        draw_picture(self);
    }

    /// The ground as a picture: grass on top where it isn't too steep (bare dirt on cliffs and fresh crater walls),
    /// a topsoil band, then earth darkening with depth, in the colours of the terrain type, with a faint grain.
    fn paint(&self) -> Image {
        let h = HEIGHT as usize;
        let mut bytes = vec![0u8; WIDTH * h * 4];
        self.paint_columns(&mut bytes, &[true; WIDTH]);
        Image {
            bytes,
            width: WIDTH as u16,
            height: h as u16,
        }
    }

    /// Paints the columns marked in `columns` into the picture `bytes`, sky included.
    fn paint_columns(&self, bytes: &mut [u8], columns: &[bool]) {
        let h = HEIGHT as usize;
        for (x, &g) in self.ground.iter().enumerate() {
            if !columns[x] {
                continue;
            }
            let left = self.ground[x.saturating_sub(1)];
            let right = self.ground[(x + 1).min(WIDTH - 1)];
            // No grass on cliffs, nor where a blast burnt the surface (it grows back when the burn mark is gone).
            // A blast high in the air doesn't reach the grass below it.
            let burnt = self
                .scorch
                .iter()
                .any(|s| (x as f32 - s.x).hypot(g - s.y) < s.radius + 4.0);
            let steep = burnt || (right - left).abs() / 2.0 > 1.3;
            let top = (g.max(0.0) as usize).min(h);
            for y in 0..h {
                let c = if y < top {
                    [0; 4]
                } else {
                    self.look.color(x, y, g, steep)
                };
                let k = (y * WIDTH + x) * 4;
                bytes[k..k + 4].copy_from_slice(&c);
            }
        }
        for s in &self.scorch {
            s.darken(bytes, columns);
        }
    }
}

/// The ground's picture and what it was painted from, to paint again only what changed since.
struct Painted {
    look: Look,
    ground: Vec<f32>,
    scorch: Vec<Scorch>,
    image: Image,
}

impl Painted {
    fn new(terrain: &Terrain) -> Self {
        Self {
            look: terrain.look,
            ground: terrain.ground.clone(),
            scorch: terrain.scorch.clone(),
            image: terrain.paint(),
        }
    }

    /// Brings the picture up to date with `terrain`. Returns the first and last column painted again, if any.
    fn update(&mut self, terrain: &Terrain) -> Option<(usize, usize)> {
        let mut columns = [self.look != terrain.look; WIDTH];
        let mut mark = |from: f32, to: f32| {
            let from = from.floor().clamp(0.0, WIDTH as f32 - 1.0) as usize;
            let to = to.ceil().clamp(0.0, WIDTH as f32 - 1.0) as usize;
            columns[from..=to].fill(true);
        };
        // A column's grass depends on its neighbours' height, so they go along.
        for (x, (old, new)) in self.ground.iter().zip(&terrain.ground).enumerate() {
            if old.to_bits() != new.to_bits() {
                mark(x as f32 - 2.0, x as f32 + 2.0);
            }
        }
        // Burn marks that came, went or faded: everything they reach.
        let (old, new) = (&self.scorch, &terrain.scorch);
        let changed = old
            .iter()
            .filter(|s| !new.contains(s))
            .chain(new.iter().filter(|s| !old.contains(s)));
        for s in changed {
            mark(s.x - s.reach() - 1.0, s.x + s.reach() + 1.0);
        }
        let first = columns.iter().position(|&c| c)?;
        let last = columns.iter().rposition(|&c| c)?;
        terrain.paint_columns(&mut self.image.bytes, &columns);
        self.look = terrain.look;
        self.ground.clone_from(&terrain.ground);
        self.scorch.clone_from(&terrain.scorch);
        Some((first, last))
    }

    /// The part of the picture from column `first` to `last`, as a picture of its own.
    fn columns(&self, first: usize, last: usize) -> Image {
        let h = HEIGHT as usize;
        let mut bytes = Vec::with_capacity((last - first + 1) * h * 4);
        for row in self.image.bytes.as_chunks::<{ WIDTH * 4 }>().0 {
            bytes.extend_from_slice(&row[first * 4..(last + 1) * 4]);
        }
        Image {
            bytes,
            width: (last - first + 1) as u16,
            height: h as u16,
        }
    }
}

impl Scorch {
    /// The width of the charred rim outside the crater.
    fn rim(&self) -> f32 {
        6.0 + 0.5 * self.radius
    }

    /// How far from its centre the mark shows.
    fn reach(&self) -> f32 {
        self.radius + self.rim()
    }

    /// Chars the ground around the crater in the marked `columns`: most at its edge, fading out over the rim.
    fn darken(&self, bytes: &mut [u8], columns: &[bool]) {
        let (rim, reach) = (self.rim(), self.reach());
        let x0 = (self.x - reach).floor().max(0.0) as usize;
        let x1 = ((self.x + reach).ceil().max(0.0) as usize).min(WIDTH - 1);
        let y0 = (self.y - reach).floor().max(0.0) as usize;
        let y1 = ((self.y + reach).ceil().max(0.0) as usize).min(HEIGHT as usize - 1);
        let marked = columns.iter().enumerate().take(x1 + 1).skip(x0);
        for (x, _) in marked.filter(|&(_, &paint)| paint) {
            for y in y0..=y1 {
                let k = (y * WIDTH + x) * 4;
                if bytes[k + 3] == 0 {
                    continue;
                }
                let d = (x as f32 - self.x).hypot(y as f32 - self.y);
                let edge = 1.0 - ((d - self.radius) / rim).clamp(0.0, 1.0);
                // Ragged, not a perfect ring.
                let burnt = 0.85 * self.heat * edge * (0.85 + 0.15 * grain(x, y + 7));
                // Towards charred earth, so grass in the rim burns too.
                for (c, char) in bytes[k..k + 3].iter_mut().zip(CHARRED) {
                    let v = f32::from(*c);
                    *c = (v + (char - v) * burnt) as u8;
                }
            }
        }
    }
}

/// The ground's picture, kept on the GPU between frames (`Terrain::draw`).
fn draw_picture(terrain: &Terrain) {
    thread_local! {
        // Never dropped: GPU objects freed after the window is gone crash the game (see AGENTS.md).
        static PICTURE: &'static std::cell::RefCell<Option<(Painted, Texture2D)>> =
            Box::leak(Box::new(std::cell::RefCell::new(None)));
    }
    PICTURE.with(|picture| {
        let mut picture = picture.borrow_mut();
        let (painted, texture) = picture.get_or_insert_with(|| {
            let painted = Painted::new(terrain);
            let texture = Texture2D::from_image(&painted.image);
            texture.set_filter(FilterMode::Nearest);
            (painted, texture)
        });
        if let Some((first, last)) = painted.update(terrain) {
            let part = painted.columns(first, last);
            let (width, height) = (i32::from(part.width), i32::from(part.height));
            texture.update_part(&part, i32::try_from(first).unwrap_or(0), 0, width, height);
        }
        draw_texture_ex(
            texture,
            0.0,
            0.0,
            WHITE,
            DrawTextureParams {
                dest_size: Some(vec2(WIDTH as f32, HEIGHT)),
                ..Default::default()
            },
        );
    });
}

#[cfg(test)]
mod look_tests {
    use super::*;

    fn pixel(image: &Image, x: usize, y: usize) -> [u8; 4] {
        let k = (y * WIDTH + x) * 4;
        image.bytes[k..k + 4].try_into().unwrap()
    }

    #[test]
    fn the_ground_is_painted_in_layers_with_sky_above() {
        let t = Terrain::from_fn(|_| 500.0);
        let image = t.paint();
        assert_eq!(pixel(&image, 300, 400)[3], 0, "sky stays clear");
        let grass = pixel(&image, 300, 500);
        let deep = pixel(&image, 300, 715);
        assert!(grass[1] > grass[0], "green on top: {grass:?}");
        assert!(
            deep[0] < 110 && deep[3] == 255,
            "darker earth below: {deep:?}"
        );
    }

    #[test]
    fn high_mountains_get_snow() {
        let mut t = Terrain::from_fn(|_| 200.0);
        t.look = Look::Mountains;
        let top = pixel(&t.paint(), 640, 201);
        assert!(top[0] > 220 && top[2] > 220, "white: {top:?}");
    }

    #[test]
    fn burn_marks_darken_the_rim_and_fade() {
        let mut t = Terrain::from_fn(|_| 500.0);
        t.carve(vec2(640.0, 500.0), 30.0);
        let plain = pixel(&t.paint(), 640, 535);
        t.burn(vec2(640.0, 500.0), 30.0);
        let burnt = pixel(&t.paint(), 640, 535);
        assert!(
            f32::from(burnt[0]) < 0.8 * f32::from(plain[0]),
            "dark under the crater: {burnt:?} vs {plain:?}"
        );
        let far = pixel(&t.paint(), 900, 600);
        assert_eq!(far, pixel(&Terrain::from_fn(|_| 500.0).paint(), 900, 600));
        for _ in 0..4 {
            t.cool();
        }
        t.cool();
        assert_eq!(t.scorch.len(), 1, "still faintly there after 5 turns");
        t.cool();
        assert!(t.scorch.is_empty(), "gone after 6 turns");
    }

    #[test]
    fn a_blast_in_the_air_leaves_the_grass_below_alone() {
        let mut t = Terrain::from_fn(|_| 500.0);
        let grass = pixel(&t.paint(), 640, 501);
        // Against a wall, 400 px up.
        t.burn(vec2(640.0, 100.0), 85.0);
        assert_eq!(pixel(&t.paint(), 640, 501), grass);
        // On the ground it burns the grass around the crater.
        t.burn(vec2(640.0, 500.0), 30.0);
        assert_ne!(pixel(&t.paint(), 665, 501), grass);
    }

    #[test]
    fn fresh_dirt_covers_burn_marks() {
        let mut t = Terrain::from_fn(|_| 500.0);
        t.carve(vec2(640.0, 500.0), 30.0);
        t.burn(vec2(640.0, 500.0), 30.0);
        t.burn(vec2(900.0, 500.0), 30.0);
        t.add_dirt(vec2(640.0, 500.0), 45.0);
        assert_eq!(
            t.scorch.len(),
            1,
            "only the mark away from the mound is left"
        );
        let top = t.height_at(640.0) as usize;
        let grass = pixel(&t.paint(), 640, top + 1);
        assert!(grass[1] > grass[0], "grass on the mound: {grass:?}");
    }

    #[test]
    fn burning_the_same_spot_again_refreshes_the_mark() {
        let mut t = Terrain::from_fn(|_| 500.0);
        t.burn(vec2(300.0, 500.0), 4.0);
        t.cool();
        t.burn(vec2(302.0, 500.0), 4.0);
        assert_eq!(t.scorch.len(), 1);
        assert!((t.scorch[0].heat - 1.0).abs() < 1e-6);
        t.burn(vec2(330.0, 500.0), 4.0);
        assert_eq!(t.scorch.len(), 2);
    }

    #[test]
    fn painting_only_what_changed_gives_the_same_picture() {
        type Step = (&'static str, fn(&mut Terrain));
        let mut t = Terrain::from_fn(|x| 420.0 + 60.0 * (x * 0.006).sin());
        let mut painted = Painted::new(&t);
        assert_eq!(painted.update(&t), None, "nothing changed");
        let steps: [Step; 9] = [
            ("crater", |t| {
                t.carve(vec2(300.0, 400.0), 40.0);
                t.burn(vec2(300.0, 400.0), 40.0);
            }),
            ("crater at the edge", |t| {
                t.carve(vec2(3.0, 420.0), 30.0);
                t.burn(vec2(3.0, 420.0), 30.0);
            }),
            ("dirt", |t| t.add_dirt(vec2(320.0, 380.0), 45.0)),
            ("trench", |t| t.sink(700.0, 860.0, 40.0)),
            ("pad", |t| t.flatten(1000.0, 14.0)),
            ("air blast", |t| t.burn(vec2(1270.0, 100.0), 85.0)),
            ("a turn", Terrain::cool),
            ("another look", |t| t.look = Look::Canyon),
            ("five turns", |t| (0..5).for_each(|_| t.cool())),
        ];
        for (name, step) in steps {
            step(&mut t);
            let (first, last) = painted.update(&t).expect(name);
            assert!(first <= last && last < WIDTH, "{name}");
            assert!(painted.image.bytes == t.paint().bytes, "{name}");
        }
        // A small change repaints a small part.
        t.carve(vec2(640.0, 400.0), 20.0);
        let (first, last) = painted.update(&t).unwrap();
        assert!(first >= 600 && last <= 680, "{first}..{last}");
        assert_eq!(
            painted.columns(first, last).bytes.len(),
            (last - first + 1) * 720 * 4
        );
    }
}
