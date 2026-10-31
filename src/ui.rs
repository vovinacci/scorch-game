//! Tiny immediate-mode UI helpers: `button(...)` draws itself every frame
//! and returns `true` on the frame it is clicked.

use std::cell::{Cell, RefCell};

use macroquad::prelude::*;

use crate::choice::Choice;
use crate::i18n::t;

/// The game is laid out on a fixed virtual screen of this size.
/// `camera()` scales it to fit the real window, with black bars if needed.
pub const WIDTH: f32 = 1280.0;
pub const HEIGHT: f32 = 720.0;

pub const PANEL: Color = Color::new(0.05, 0.06, 0.1, 0.9);
pub const ACCENT: Color = SKYBLUE;

pub fn chance(p: f32) -> bool {
    macroquad::rand::gen_range(0.0, 1.0) < p
}

/// Camera that maps the virtual screen onto the largest centered area of the window
/// with the same aspect ratio.
pub fn camera() -> Camera2D {
    let scale = scale();
    let (w, h) = (WIDTH * scale, HEIGHT * scale);
    Camera2D {
        target: vec2(WIDTH / 2.0, HEIGHT / 2.0),
        // Positive y zoom = y grows downward, like screen coordinates.
        zoom: vec2(2.0 / WIDTH, 2.0 / HEIGHT),
        viewport: Some((
            ((screen_width() - w) / 2.0) as i32,
            ((screen_height() - h) / 2.0) as i32,
            w as i32,
            h as i32,
        )),
        ..Default::default()
    }
}

/// How many real pixels one virtual pixel takes.
fn scale() -> f32 {
    (screen_width() / WIDTH).min(screen_height() / HEIGHT)
}

/// Draws text. Glyphs are rasterized at the real window resolution and then
/// scaled back, so text stays sharp however the game is scaled.
pub fn text_at(text: impl AsRef<str>, x: f32, y: f32, size: f32, color: Color) {
    let text = text.as_ref();
    let s = scale();
    FONT.with(|&font| {
        let params = |color| TextParams {
            font: Some(font),
            font_size: (size * s).round() as u16,
            font_scale: 1.0 / s,
            color,
            ..Default::default()
        };
        if SHADOW.get() {
            // Dark outline, about 1/16 of the text size: readable over any sky or terrain.
            let d = (size / 16.0).max(1.0 / s);
            let dark = Color::new(0.0, 0.0, 0.0, 0.8 * color.a);
            for (dx, dy) in [(-d, 0.0), (d, 0.0), (0.0, -d), (0.0, d), (d, d)] {
                draw_text_ex(text, x + dx, y + dy, params(dark));
            }
        }
        draw_text_ex(text, x, y, params(color));
    });
}

thread_local! {
    static SHADOW: Cell<bool> = const { Cell::new(false) };
    /// Scorch Mono: Terminus with standard Cyrillic ve/de (SIL Open Font License,
    /// see assets/fonts/README.md). Leaked on purpose: dropping it at exit
    /// would free its texture after the window's GL context is gone (segfault).
    static FONT: &'static Font = Box::leak(Box::new(
        load_ttf_font_from_bytes(include_bytes!("../assets/fonts/ScorchMono.ttf"))
            .expect("bundled font loads"),
    ));
}

/// Draws everything in `f` with outlined text. Used over the battlefield,
/// where text can sit on any sky or terrain color.
pub fn with_shadow(f: impl FnOnce()) {
    SHADOW.set(true);
    f();
    SHADOW.set(false);
}

/// Measures text the same way `text_at` draws it.
pub fn measure(text: impl AsRef<str>, size: f32) -> TextDimensions {
    let s = scale();
    FONT.with(|&font| measure_text(text, Some(font), (size * s).round() as u16, 1.0 / s))
}

/// Mouse position in virtual screen coordinates.
pub fn mouse() -> Vec2 {
    let (x, y) = mouse_position();
    camera().screen_to_world(vec2(x, y))
}

/// Keyboard navigation for menus: Up/Down move focus between widgets in the
/// order they are drawn, Enter/Space press the focused one, Left/Right change
/// a focused selector. Moving the mouse over a widget focuses it too.
#[derive(Default)]
struct Nav {
    focus: usize,
    /// Widgets registered so far this frame.
    count: usize,
    enabled: bool,
    activate: bool,
    left: bool,
    right: bool,
}

thread_local! {
    static NAV: RefCell<Nav> = RefCell::new(Nav::default());
}

/// Call once per frame, before any widget. `keyboard` is false during
/// gameplay, where the arrow keys aim the tank.
pub fn begin_frame(keyboard: bool) {
    NAV.with_borrow_mut(|nav| {
        let widgets = nav.count;
        nav.count = 0;
        nav.enabled = keyboard && widgets > 0;
        nav.activate = false;
        nav.left = false;
        nav.right = false;
        if !nav.enabled {
            return;
        }
        if nav.focus >= widgets {
            nav.focus = 0;
        }
        if is_key_pressed(KeyCode::Down) {
            nav.focus = (nav.focus + 1) % widgets;
        }
        if is_key_pressed(KeyCode::Up) {
            nav.focus = (nav.focus + widgets - 1) % widgets;
        }
        nav.activate = is_key_pressed(KeyCode::Enter) || is_key_pressed(KeyCode::Space);
        nav.left = is_key_pressed(KeyCode::Left);
        nav.right = is_key_pressed(KeyCode::Right);
    });
}

/// Call when the screen changes, so focus starts at the first widget.
pub fn reset_focus() {
    NAV.with_borrow_mut(|nav| nav.focus = 0);
}

struct Focus {
    focused: bool,
    activate: bool,
    left: bool,
    right: bool,
}

fn register(hovered: bool) -> Focus {
    NAV.with_borrow_mut(|nav| {
        let id = nav.count;
        nav.count += 1;
        if hovered && mouse_delta_position() != Vec2::ZERO {
            nav.focus = id;
        }
        let focused = nav.enabled && nav.focus == id;
        Focus {
            focused,
            activate: focused && nav.activate,
            left: focused && nav.left,
            right: focused && nav.right,
        }
    })
}

fn widget_background(r: Rect, enabled: bool, highlighted: bool, focused: bool) {
    let bg = if !enabled {
        Color::from_rgba(35, 35, 45, 255)
    } else if highlighted {
        Color::from_rgba(70, 95, 150, 255)
    } else {
        Color::from_rgba(40, 52, 85, 255)
    };
    draw_rectangle(r.x, r.y, r.w, r.h, bg);
    let border = if focused {
        GOLD
    } else if enabled {
        ACCENT
    } else {
        DARKGRAY
    };
    draw_rectangle_lines(r.x, r.y, r.w, r.h, if focused { 3.0 } else { 2.0 }, border);
}

/// Largest text size up to `size` at which `text` fits in `width`.
fn fit(text: &str, mut size: f32, width: f32) -> (f32, TextDimensions) {
    let mut dims = measure(text, size);
    while dims.width > width && size > 12.0 {
        size -= 1.0;
        dims = measure(text, size);
    }
    (size, dims)
}

/// Draws text at `x, y`, shrinking it if needed to fit within `max_w`.
pub fn text_fit(text: &str, x: f32, y: f32, max_w: f32, size: f32, color: Color) {
    let (size, _) = fit(text, size, max_w);
    text_at(text, x, y, size, color);
}

pub fn button(r: Rect, label: &str, enabled: bool) -> bool {
    let hovered = r.contains(mouse());
    let focus = register(hovered);
    widget_background(r, enabled, hovered || focus.focused, focus.focused);

    let (size, dims) = fit(label, 22.0, r.w - 16.0);
    text_at(
        label,
        r.x + (r.w - dims.width) / 2.0,
        r.y + (r.h + dims.offset_y) / 2.0,
        size,
        if enabled { WHITE } else { GRAY },
    );
    enabled && ((hovered && is_mouse_button_pressed(MouseButton::Left)) || focus.activate)
}

/// A row "Label        < value >". Click the left arrow area to go back,
/// anywhere else to go forward; Left/Right/Enter when focused.
/// Returns `true` when the value changed.
pub fn selector(r: Rect, label: &str, values: &[impl AsRef<str>], index: &mut usize) -> bool {
    let hovered = r.contains(mouse());
    let focus = register(hovered);
    widget_background(r, true, hovered || focus.focused, focus.focused);

    let value_box = Rect::new(r.x + r.w * 0.5, r.y, r.w * 0.5, r.h);
    let (size, dims) = fit(label, 22.0, r.w * 0.5 - 24.0);
    text_at(
        label,
        r.x + 14.0,
        r.y + (r.h + dims.offset_y) / 2.0,
        size,
        LIGHTGRAY,
    );
    let value = values.get(*index).map_or("?", |v| v.as_ref());
    let (size, dims) = fit(value, 22.0, value_box.w - 60.0);
    text_at(
        value,
        value_box.x + (value_box.w - dims.width) / 2.0,
        r.y + (r.h + dims.offset_y) / 2.0,
        size,
        WHITE,
    );
    let mid = r.y + r.h / 2.0;
    let arrow = |x: f32, dir: f32| {
        draw_triangle(
            vec2(x + 6.0 * dir, mid),
            vec2(x - 6.0 * dir, mid - 8.0),
            vec2(x - 6.0 * dir, mid + 8.0),
            GOLD,
        )
    };
    arrow(value_box.x + 16.0, -1.0);
    arrow(value_box.x + value_box.w - 16.0, 1.0);

    let n = values.len();
    let clicked = hovered && is_mouse_button_pressed(MouseButton::Left);
    let back = focus.left || (clicked && mouse().x < value_box.x + 36.0);
    let forward = focus.right || focus.activate || (clicked && !back);
    if n > 1 && back {
        *index = (*index + n - 1) % n;
        true
    } else if n > 1 && forward {
        *index = (*index + 1) % n;
        true
    } else {
        false
    }
}

/// Selector for any `Choice` enum.
pub fn choice<T: Choice>(r: Rect, label: &str, value: &mut T) -> bool {
    let labels: Vec<String> = T::ALL.iter().map(|v| v.text()).collect();
    let mut index = T::ALL.iter().position(|v| v == value).unwrap_or(0);
    let changed = selector(r, label, &labels, &mut index);
    *value = T::ALL[index];
    changed
}

/// On/Off selector.
pub fn toggle(r: Rect, label: &str, value: &mut bool) -> bool {
    let mut index = *value as usize;
    let changed = selector(r, label, &[t!("off"), t!("on")], &mut index);
    *value = index == 1;
    changed
}

/// A small globe icon (circle, meridians, equator): the usual sign for "language".
pub fn globe(c: Vec2, r: f32, color: Color) {
    draw_circle_lines(c.x, c.y, r, 1.5, color);
    draw_ellipse_lines(c.x, c.y, r * 0.45, r, 0.0, 1.5, color);
    draw_line(c.x - r, c.y, c.x + r, c.y, 1.5, color);
    draw_line(c.x, c.y - r, c.x, c.y + r, 1.5, color);
}

pub fn panel(r: Rect) {
    draw_rectangle(r.x, r.y, r.w, r.h, PANEL);
    draw_rectangle_lines(r.x, r.y, r.w, r.h, 2.0, Color { a: 0.5, ..ACCENT });
}

/// Centred text; shrinks to stay inside the screen.
pub fn text_centered(text: &str, cx: f32, y: f32, size: f32, color: Color) {
    text_centered_fit(text, cx, y, WIDTH - 32.0, size, color);
}

/// Centred text, shrunk if needed to fit in `max_w`.
pub fn text_centered_fit(text: &str, cx: f32, y: f32, max_w: f32, size: f32, color: Color) {
    let (size, dims) = fit(text, size, max_w);
    text_at(text, cx - dims.width / 2.0, y, size, color);
}

/// Draws word-wrapped text and returns the y just below the last line.
pub fn text_wrapped(text: &str, x: f32, mut y: f32, max_w: f32, size: f32, color: Color) -> f32 {
    for paragraph in text.split('\n') {
        let mut line = String::new();
        for word in paragraph.split_whitespace() {
            let candidate = if line.is_empty() {
                word.to_string()
            } else {
                format!("{line} {word}")
            };
            if !line.is_empty() && measure(&candidate, size).width > max_w {
                text_at(&line, x, y, size, color);
                y += size * 1.25;
                line = word.to_string();
            } else {
                line = candidate;
            }
        }
        text_at(&line, x, y, size, color);
        y += size * 1.25;
    }
    y
}

/// Horizontal slider: click or drag inside `r` to set the value.
pub fn slider(r: Rect, value: f32, min: f32, max: f32) -> f32 {
    let frac = ((value - min) / (max - min)).clamp(0.0, 1.0);
    let mid = r.y + r.h / 2.0;
    draw_rectangle(r.x, mid - 3.0, r.w, 6.0, Color::from_rgba(50, 55, 75, 255));
    draw_rectangle(r.x, mid - 3.0, r.w * frac, 6.0, ACCENT);
    draw_circle(r.x + r.w * frac, mid, 9.0, WHITE);

    let Vec2 { x: mx, y: my } = mouse();
    if is_mouse_button_down(MouseButton::Left) && r.contains(vec2(mx, my)) {
        min + ((mx - r.x) / r.w).clamp(0.0, 1.0) * (max - min)
    } else {
        value
    }
}
