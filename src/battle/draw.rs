//! Drawing the battle: the scene, the HUD, tanks, flames, the weapon strip and the full battle log.

use macroquad::prelude::*;

use super::shells::predict_path;
use super::{
    Battle, EXPLOSION_TIME, FIRE_TIME, Fire, HUD_HEIGHT, LOG_ROW, LOG_TIME, NAME_SIZE, Phase,
    PreviewKey, STRIP_KEYS, SUDDEN_DEATH_DRAIN, Tank, TracerTurn, covers_a_tank, launch_velocity,
    weapon_strip, wraps, you,
};
use crate::i18n::t;
use crate::items::{BATTERY, ITEMS};
use crate::player::Player;
use crate::settings::HealthDisplay;
use crate::settings::Settings;
use crate::terrain::HEIGHT;
use crate::ui;
use crate::weapons::WEAPONS;
use crate::world::Sky;

impl Battle {
    pub fn draw(&self, players: &[Player], settings: &Settings) {
        ui::with_shadow(|| {
            self.draw_scene(players, settings);
            self.draw_hud(players);
        });
    }

    /// The battlefield without the HUD, behind a full-screen menu that needs the top of the screen.
    pub fn draw_backdrop(&self, players: &[Player], settings: &Settings) {
        ui::with_shadow(|| self.draw_scene(players, settings));
    }

    /// While aiming: the dotted mouse-aiming line and the previous shot's path.
    pub(super) fn draw_aim_guides(&self, players: &[Player], settings: &Settings) {
        // Aiming help: the predicted path (or its first part) while a human aims.
        let share = self.rules.aim_help.preview();
        if share > 0.0 && matches!(self.phase, Phase::Aim { .. }) && !players[self.current].is_bot()
        {
            let tank = &self.tanks[self.current];
            let key = PreviewKey {
                turn: self.turns,
                current: self.current,
                angle: tank.angle,
                power: tank.power,
                wind: self.wind,
            };
            let mut cache = self.preview.borrow_mut();
            if cache.as_ref().is_none_or(|(k, _)| *k != key) {
                let vel = launch_velocity(tank.angle, tank.power);
                let path = predict_path(
                    tank.muzzle(),
                    vel,
                    self.wind,
                    self.world(),
                    &self.terrain,
                    &self.deflectors(self.current),
                );
                *cache = Some((key, path));
            }
            let path = &cache.as_ref().expect("filled above").1;
            let shown = (path.len() as f32 * share).ceil() as usize;
            for w in path[..shown.min(path.len())].windows(2).step_by(4) {
                if !wraps(w[0], w[1]) {
                    guide_dot(w[0], Color::new(1.0, 0.95, 0.6, 0.8));
                }
            }
        }

        // Mouse aiming: a dotted line from the barrel to the cursor.
        if self.drag.is_some()
            && is_mouse_button_down(MouseButton::Left)
            && matches!(self.phase, Phase::Aim { .. })
        {
            let from = self.tanks[self.current].muzzle();
            let to = ui::mouse();
            let steps = (from.distance(to) / 8.0) as usize;
            for k in 0..=steps {
                let p = from.lerp(to, k as f32 / steps.max(1) as f32);
                guide_dot(p, Color::new(1.0, 1.0, 1.0, 0.7));
            }
        }

        if settings.shot_path && matches!(self.phase, Phase::Aim { .. }) {
            for p in self.tanks[self.current].last_trail.iter().step_by(6) {
                guide_dot(*p, Color::new(1.0, 1.0, 1.0, 0.45));
            }
        }
    }

    /// Every Tracer path fired this round, in its shooter's colour over a dark edge (readable on any sky), for
    /// everyone to see.
    pub(super) fn draw_tracers(&self, players: &[Player]) {
        let edge = Color::new(0.0, 0.0, 0.0, 0.45);
        for path in &self.tracers {
            let color = Color {
                a: 0.85,
                ..players[path.owner].color
            };
            for w in path.points.windows(2).step_by(2) {
                let (a, b) = (vec2(w[0][0], w[0][1]), vec2(w[1][0], w[1][1]));
                // Off the top of the screen, or the jump of a shell wrapping around: nothing to draw.
                if (a.y < 0.0 && b.y < 0.0) || wraps(a, b) {
                    continue;
                }
                draw_line(a.x, a.y, b.x, b.y, 3.5, edge);
                draw_line(a.x, a.y, b.x, b.y, 1.5, color);
            }
        }
    }

    pub(super) fn draw_scene(&self, players: &[Player], settings: &Settings) {
        self.world().sky.draw();
        self.terrain.draw();

        self.draw_tracers(players);
        self.draw_aim_guides(players, settings);

        for (i, tank) in self.tanks.iter().enumerate() {
            if tank.fled {
                continue;
            }
            if !tank.alive {
                // Wrecks rest on whatever ground is left below them.
                let y = self.terrain.support_y(tank.pos.x, 10.0);
                draw_rectangle(tank.pos.x - 10.0, y - 5.0, 20.0, 5.0, DARKGRAY);
                continue;
            }
            let is_current = i == self.current && matches!(self.phase, Phase::Aim { .. });
            let pushing = self.projectiles.iter().any(|p| p.charged & (1u8 << i) != 0);
            draw_tank(
                tank,
                &players[i],
                is_current,
                settings.health_display,
                pushing,
            );
        }

        for p in &self.projectiles {
            for w in p.trail.windows(2).rev().take(60) {
                // Skip the jump across the screen when a shell wraps around.
                if wraps(w[0], w[1]) {
                    continue;
                }
                draw_line(
                    w[0].x,
                    w[0].y,
                    w[1].x,
                    w[1].y,
                    2.0,
                    Color::new(1.0, 0.9, 0.5, 0.5),
                );
            }
            if p.pos.y < 0.0 {
                // Off the top of the screen: show where it is.
                draw_triangle(
                    vec2(p.pos.x, 50.0),
                    vec2(p.pos.x - 6.0, 62.0),
                    vec2(p.pos.x + 6.0, 62.0),
                    YELLOW,
                );
            } else {
                draw_circle(p.pos.x, p.pos.y, 3.0, WHITE);
            }
        }

        for e in &self.explosions {
            let k = e.age / EXPLOSION_TIME;
            let r = e.radius * (0.5 + 0.5 * k.sqrt());
            if e.dirt {
                draw_circle(e.pos.x, e.pos.y, r, Color::new(0.5, 0.35, 0.2, 1.0 - k));
            } else {
                draw_circle(e.pos.x, e.pos.y, r, Color::new(1.0, 0.5, 0.1, 1.0 - k));
                draw_circle(
                    e.pos.x,
                    e.pos.y,
                    r * 0.6,
                    Color::new(1.0, 0.95, 0.6, 1.0 - k),
                );
            }
        }

        for f in &self.fires {
            draw_flame(f, self.terrain.height_at(f.x));
        }

        if settings.battle_log {
            self.draw_feed();
        }
    }

    /// Recent events, top left, fading out, on a dark backing.
    pub(super) fn draw_feed(&self) {
        if !self.log.is_empty() {
            let widest = self
                .log
                .iter()
                .map(|line| ui::measure(&line.text, 18.0).width)
                .fold(0.0, f32::max);
            let fade = self
                .log
                .iter()
                .map(|line| ((LOG_TIME - line.age) / 2.0).clamp(0.0, 1.0))
                .fold(0.0, f32::max);
            backing_faded(
                Rect::new(8.0, 52.0, widest + 16.0, 22.0 * self.log.len() as f32 + 8.0),
                self.world().sky,
                fade,
            );
            let mut y = 70.0;
            for line in &self.log {
                let fade = ((LOG_TIME - line.age) / 2.0).clamp(0.0, 1.0);
                ui::text_at(
                    &line.text,
                    16.0,
                    y,
                    18.0,
                    Color {
                        a: fade,
                        ..line.color
                    },
                );
                y += 22.0;
            }
        }
    }

    #[expect(clippy::too_many_lines, reason = "the HUD, drawn left to right")]
    pub(super) fn draw_hud(&self, players: &[Player]) {
        draw_rectangle(0.0, 0.0, ui::WIDTH, HUD_HEIGHT, ui::PANEL);
        let player = &players[self.current];
        let tank = &self.tanks[self.current];
        // Your turn only: on bot turns the bottom of the screen has the skip button.
        if matches!(self.phase, Phase::Aim { .. }) && !player.is_bot() {
            draw_weapon_strip(player, tank.weapon, self.tracer != TracerTurn::Unused);
            // A tank down in a deep valley or crater behind the strip is drawn again on top of it (without its
            // name, which would run into the strip's text), so it stays in sight.
            let strip = weapon_strip(player);
            let areas = self.tank_areas(players);
            let shown = self
                .tanks
                .iter()
                .zip(players)
                .filter(|(t, _)| t.alive && !t.fled);
            for ((t, p), area) in shown.zip(areas) {
                if covers_a_tank(&strip, &[area]) {
                    draw_tank_body(t, p.color);
                }
            }
            // Aiming help: what went wrong with your last shot, just above the strip, until you fire.
            if let Some(text) = &tank.coaching {
                let top = strip.iter().map(|(_, r)| r.y).fold(HEIGHT, f32::min);
                let max = 760.0;
                let w = ui::measure(text, 18.0).width.min(max);
                backing(
                    Rect::new((ui::WIDTH - w) / 2.0 - 10.0, top - 32.0, w + 20.0, 26.0),
                    self.world().sky,
                );
                ui::text_centered_fit(text, ui::WIDTH / 2.0, top - 13.0, max, 18.0, LIGHTGRAY);
            }
        }
        let name = player.label();
        ui::text_fit(&name, 16.0, 29.0, 260.0, 24.0, player.color);
        // The info starts after the name, however long it is.
        let info_x = (16.0 + ui::measure(&name, 24.0).width.min(260.0) + 24.0).max(180.0);
        let weapon = &WEAPONS[tank.weapon];
        let max_power = self.max_power(tank);
        let power = if self.rules.power_limit {
            format!("{:.0}/{:.0}", tank.power, max_power)
        } else {
            format!("{:.0}", tank.power)
        };
        let info = t!(
            "hud-info",
            angle = tank.angle.round(),
            power = power,
            weapon = weapon.text(),
            ammo = player.ammo_label(tank.weapon),
            money = player.money,
            income = player.income.total()
        );
        ui::text_fit(&info, info_x, 29.0, 860.0 - info_x, 22.0, WHITE);

        // Wind indicator: arrow length and direction show strength.
        let cx = 1000.0;
        ui::text_fit(&t!("hud-wind"), 880.0, 29.0, 70.0, 22.0, LIGHTGRAY);
        let len = self.wind / self.world().max_wind().max(1.0) * 45.0;
        if len.abs() > 1.0 {
            draw_line(cx, 22.0, cx + len, 22.0, 3.0, SKYBLUE);
            let tip = cx + len;
            let back = tip - 8.0 * len.signum();
            draw_triangle(vec2(tip, 22.0), vec2(back, 16.0), vec2(back, 28.0), SKYBLUE);
        }
        let strength = if self.world().max_wind() == 0.0 {
            t!("hud-wind-off")
        } else {
            format!("{:.0}", self.wind.abs())
        };
        ui::text_at(strength, cx + 58.0, 29.0, 22.0, SKYBLUE);
        ui::text_at(
            t!("hud-round", round = self.round, rounds = self.rounds),
            1160.0,
            29.0,
            22.0,
            WHITE,
        );

        // Turn counter, top right under the bar.
        let turn = match self.rules.sudden_death {
            Some(start) if self.turns < start => {
                t!("hud-turn-sudden", turn = self.turns + 1, start = start)
            }
            _ => t!("hud-turn", turn = self.turns + 1),
        };
        // Under it, one short line each for the Random picks (clear of the centred sudden death banner), then the
        // current tank's shield. All right-aligned on a dark backing, readable on any sky.
        let mut lines = vec![(turn, LIGHTGRAY)];
        for text in self.random_picks(false) {
            lines.push((text, SKYBLUE));
        }
        if let Some(shield) = tank.shield_up() {
            let text = t!(
                "hud-shield",
                name = ITEMS[shield.item].text(),
                energy = shield.shown(),
                full = ITEMS[shield.item].shield_energy().unwrap_or(0.0)
            );
            lines.push((text, shield.color()));
        }
        let batteries = players[self.current].items[BATTERY];
        if batteries > 0
            && !players[self.current].is_bot()
            && matches!(self.phase, Phase::Aim { .. })
        {
            lines.push((t!("hud-battery", n = batteries), GREEN));
        }
        if self.aiming_into_dirt(players) {
            lines.push((t!("hud-buried"), ORANGE));
        }
        let max = 360.0;
        let widths: Vec<f32> = lines
            .iter()
            .map(|(text, _)| ui::measure(text, 18.0).width.min(max))
            .collect();
        let widest = widths.iter().copied().fold(0.0, f32::max);
        backing(
            Rect::new(
                ui::WIDTH - 24.0 - widest,
                50.0,
                widest + 16.0,
                22.0 * lines.len() as f32 + 8.0,
            ),
            self.world().sky,
        );
        for (k, ((text, color), w)) in lines.iter().zip(&widths).enumerate() {
            let y = 66.0 + 22.0 * k as f32;
            ui::text_fit(text, ui::WIDTH - 16.0 - w, y, max, 18.0, *color);
        }

        // Sudden death warning or banner.
        if let Some(start) = self.rules.sudden_death {
            if self.turns >= start {
                ui::text_centered(
                    &t!("hud-sudden-death", hp = SUDDEN_DEATH_DRAIN),
                    ui::WIDTH / 2.0,
                    92.0,
                    22.0,
                    RED,
                );
            } else if start - self.turns <= 5 {
                ui::text_centered(
                    &t!("hud-sudden-death-in", n = start - self.turns),
                    ui::WIDTH / 2.0,
                    92.0,
                    20.0,
                    ORANGE,
                );
            }
        }

        if let (Phase::Aim { .. }, Some(memory)) = (self.phase, tank.aim)
            && player.is_bot()
            && self.tanks[memory.target].alive
        {
            let mut text = t!(
                "hud-aiming-at",
                name = player.label(),
                target = players[memory.target].label(),
                target_you = you(&players[memory.target])
            );
            // The reason isn't saved, so it's unknown right after Continue.
            if !memory.reason.is_empty() {
                let reason = t!(&format!("reason-{}", memory.reason));
                text.push_str(" (");
                text.push_str(&reason);
                text.push(')');
            }
            ui::text_centered(&text, ui::WIDTH / 2.0, HEIGHT - 14.0, 20.0, LIGHTGRAY);
        }

        match self.phase {
            Phase::Aim { .. } if !player.is_bot() => ui::text_centered(
                &t!("hud-controls"),
                ui::WIDTH / 2.0,
                HEIGHT - 14.0,
                20.0,
                WHITE,
            ),
            Phase::Over { winner, .. } => {
                let text = match winner {
                    Some(w) => t!(
                        "round-won",
                        name = players[w].label(),
                        you = you(&players[w])
                    ),
                    None => t!("round-draw"),
                };
                let r = Rect::new(320.0, 280.0, 640.0, 90.0);
                ui::panel(r);
                ui::text_centered_fit(&text, r.center().x, r.y + 56.0, r.w - 40.0, 34.0, GOLD);
            }
            _ => {}
        }
    }
}

/// Full battle log of this round, newest at the bottom.
pub fn draw_history(battle: &Battle) {
    draw_rectangle(
        0.0,
        0.0,
        ui::WIDTH,
        ui::HEIGHT,
        Color::new(0.0, 0.0, 0.0, 0.6),
    );
    let r = Rect::new(240.0, 60.0, 800.0, 600.0);
    ui::panel(r);
    ui::text_centered(
        &t!("history-title", round = battle.round),
        r.center().x,
        r.y + 50.0,
        30.0,
        GOLD,
    );
    let top = r.y + 90.0;
    let rows = ((r.h - 160.0) / LOG_ROW) as usize;
    if battle.history.is_empty() {
        ui::text_centered(&t!("history-empty"), r.center().x, top + 20.0, 20.0, GRAY);
    }
    let start = battle.history.len().saturating_sub(rows);
    let mut last_turn = None;
    for (k, line) in battle.history[start..].iter().enumerate() {
        let y = top + k as f32 * LOG_ROW;
        // Show the turn number once per turn.
        if last_turn != Some(line.turn) {
            ui::text_at(
                t!("hud-turn", turn = line.turn),
                r.x + 30.0,
                y,
                18.0,
                LIGHTGRAY,
            );
            last_turn = Some(line.turn);
        }
        ui::text_fit(
            &line.text,
            r.x + 130.0,
            y,
            r.w - 160.0,
            20.0,
            line.color.into(),
        );
    }
}

/// How dark the panel behind text over the battlefield is: none on the dark skies, where the text's outline is
/// enough, and a light one on the bright day sky.
pub(super) fn backing_alpha(sky: Sky) -> f32 {
    match sky {
        Sky::Day => 0.4,
        _ => 0.0,
    }
}

/// A translucent panel behind text drawn over the battlefield, for the day sky (see `backing_alpha`).
pub(super) fn backing(r: Rect, sky: Sky) {
    backing_faded(r, sky, 1.0);
}

pub(super) fn backing_faded(r: Rect, sky: Sky, fade: f32) {
    let a = backing_alpha(sky) * fade;
    if a > 0.0 {
        draw_rectangle(r.x, r.y, r.w, r.h, Color { a, ..ui::PANEL });
    }
}

/// A small dot with a dark edge, for aiming lines: visible on dark and light skies alike.
pub(super) fn guide_dot(p: Vec2, color: Color) {
    draw_circle(p.x, p.y, 2.5, Color::new(0.0, 0.0, 0.0, 0.35 * color.a));
    draw_circle(p.x, p.y, 1.5, color);
}

/// The strip, on your turn. A Tracer already fired this turn (`tracer_used`) is dimmed: one per turn.
pub(super) fn draw_weapon_strip(player: &Player, selected: usize, tracer_used: bool) {
    for (k, (w, r)) in weapon_strip(player).into_iter().enumerate() {
        let on = w == selected;
        let spent = tracer_used && WEAPONS[w].kind.is_probe();
        // Fully opaque: nothing behind (a tank's name in a deep crater) shows through the text.
        draw_rectangle(
            r.x,
            r.y,
            r.w,
            r.h,
            Color {
                a: 1.0,
                ..ui::PANEL
            },
        );
        let border = if on {
            GOLD
        } else {
            Color::new(1.0, 1.0, 1.0, 0.15)
        };
        draw_rectangle_lines(r.x, r.y, r.w, r.h, if on { 2.0 } else { 1.0 }, border);
        let key = STRIP_KEYS.get(k).map_or("", |&(_, label)| label);
        let text = format!("{key}{} {}", WEAPONS[w].text(), player.ammo_label(w));
        let color = if spent {
            GRAY
        } else if on {
            WHITE
        } else {
            LIGHTGRAY
        };
        ui::text_centered_fit(&text, r.center().x, r.y + 18.0, r.w - 8.0, 16.0, color);
    }
}

/// A drop of napalm: a glowing patch on the ground and a flickering flame licking up from it, swaying and dying
/// down as the drop burns out.
pub(super) fn draw_flame(f: &Fire, ground: f32) {
    // Flares up in the first moments, then slowly dies down.
    let life = (f.age / 0.25).clamp(0.0, 1.0) * (1.0 - f.age / FIRE_TIME);
    let phase = f.seed * 40.0;
    let x = f.x + (f.seed - 0.5) * 18.0;
    let flicker = 0.7 + 0.3 * (f.age * 21.0 + phase).sin() * (f.age * 8.0 + phase * 0.7).cos();
    let height = (18.0 + 16.0 * flicker) * life;
    let sway = 3.0 * (f.age * 5.0 + phase).sin() * life;
    let half = 5.0 + 2.0 * life;
    draw_rectangle(
        x - half,
        ground - 1.5,
        2.0 * half,
        3.0,
        Color::new(0.9, 0.2, 0.0, 0.5 * life),
    );
    draw_triangle(
        vec2(x - half * 1.3, ground),
        vec2(x + half * 1.3, ground),
        vec2(x + sway * 1.2, ground - height * 1.1),
        Color::new(0.85, 0.12, 0.0, 0.75 * life),
    );
    draw_triangle(
        vec2(x - half, ground),
        vec2(x + half, ground),
        vec2(x + sway, ground - height * 0.8),
        Color::new(1.0, 0.5, 0.0, 0.9 * life),
    );
    draw_triangle(
        vec2(x - half * 0.5, ground),
        vec2(x + half * 0.5, ground),
        vec2(x + sway * 0.6, ground - height * 0.45),
        Color::new(1.0, 0.88, 0.25, 0.95 * life),
    );
    // A spark now and then, rising off the flame.
    let rise = (f.age * 1.3 + f.seed).fract();
    if rise < 0.5 {
        draw_rectangle(
            x + sway * 1.5,
            ground - height - 30.0 * rise,
            1.5,
            1.5,
            Color::new(1.0, 0.75, 0.3, (1.0 - 2.0 * rise) * life),
        );
    }
}

/// The tank itself: barrel, turret and hull.
pub(super) fn draw_tank_body(tank: &Tank, color: Color) {
    let (x, y) = (tank.pos.x, tank.pos.y);
    let base = tank.barrel_base();
    let tip = tank.muzzle();
    draw_line(base.x, base.y, tip.x, tip.y, 3.0, LIGHTGRAY);
    draw_circle(x, y - 8.0, 7.0, color);
    draw_rectangle(x - 12.0, y - 8.0, 24.0, 8.0, color);
}

#[expect(
    clippy::too_many_lines,
    reason = "one tank with its bars, labels and shield"
)]
pub(super) fn draw_tank(
    tank: &Tank,
    player: &Player,
    is_current: bool,
    health: HealthDisplay,
    pushing: bool,
) {
    let (x, y) = (tank.pos.x, tank.pos.y);
    draw_tank_body(tank, player.color);

    // A Mag Deflector that has spent its last energy still shows, faintly, while it pushes the shell that paid.
    if let Some(shield) = tank.shield_up().or(tank.shield.filter(|_| pushing)) {
        let color = shield.color();
        let a = 0.25 + 0.55 * shield.strength();
        draw_circle(
            x,
            y - 8.0,
            20.0,
            Color {
                a: a * 0.25,
                ..color
            },
        );
        draw_circle_lines(x, y - 8.0, 20.0, 1.5, Color { a, ..color });
    }
    if tank.parachute {
        // Canopy: a half disc above the tank, with three lines down to it.
        let (cx, cy, r) = (x, y - 44.0, 16.0);
        let canopy = Color::new(0.95, 0.95, 0.9, 0.9);
        for k in 0..12 {
            let a0 = std::f32::consts::PI * (1.0 + k as f32 / 12.0);
            let a1 = std::f32::consts::PI * (1.0 + (k + 1) as f32 / 12.0);
            draw_triangle(
                vec2(cx, cy),
                vec2(cx + r * a0.cos(), cy + r * a0.sin()),
                vec2(cx + r * a1.cos(), cy + r * a1.sin()),
                canopy,
            );
        }
        for dx in [-r, 0.0, r] {
            draw_line(cx + dx, cy, x + dx * 0.5, y - 12.0, 1.0, LIGHTGRAY);
        }
    }

    let hp = (tank.health / 100.0).clamp(0.0, 1.0);
    let hp_color = if hp > 0.5 {
        GREEN
    } else if hp > 0.25 {
        YELLOW
    } else {
        RED
    };
    let mut label_y = if tank.shield_up().is_some() {
        y - 42.0
    } else {
        y - 38.0
    };
    if health != HealthDisplay::Number {
        draw_rectangle(
            x - 16.0,
            y - 32.0,
            32.0,
            4.0,
            Color::from_rgba(40, 40, 40, 255),
        );
        draw_rectangle(x - 16.0, y - 32.0, 32.0 * hp, 4.0, hp_color);
        // Shield energy: a second bar just above, in the shield's colour.
        if let Some(shield) = tank.shield_up() {
            draw_rectangle(
                x - 16.0,
                y - 36.0,
                32.0,
                3.0,
                Color::from_rgba(40, 40, 40, 255),
            );
            draw_rectangle(
                x - 16.0,
                y - 36.0,
                32.0 * shield.strength(),
                3.0,
                shield.color(),
            );
        }
    }
    if health != HealthDisplay::Bar {
        let number_y = if health == HealthDisplay::Number {
            // Clear of the shield ring when one is up.
            if tank.shield_up().is_some() {
                y - 32.0
            } else {
                y - 26.0
            }
        } else {
            label_y
        };
        let health_text = format!("{:.0}", tank.health.max(0.0).ceil());
        ui::text_centered(&health_text, x, number_y, 16.0, hp_color);
        // Shield energy follows the health number: "75 +40".
        if let Some(shield) = tank.shield_up() {
            ui::text_at(
                format!("+{:.0}", shield.shown()),
                x + ui::measure(&health_text, 16.0).width / 2.0 + 4.0,
                number_y,
                16.0,
                shield.color(),
            );
        }
        label_y = number_y - 16.0;
    }
    ui::text_centered(&player.label(), x, label_y, NAME_SIZE, WHITE);

    if is_current {
        let bob = (get_time() * 5.0).sin() as f32 * 4.0;
        let ty = label_y - 24.0 + bob;
        draw_triangle(
            vec2(x, ty + 10.0),
            vec2(x - 7.0, ty),
            vec2(x + 7.0, ty),
            YELLOW,
        );
    }
}
