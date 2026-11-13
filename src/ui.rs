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

/// Keyboard navigation for menus: the arrow keys move focus along rows and columns
/// of widgets, inside the panel first (see `step`), Enter/Space press the focused
/// one, Left/Right change a focused selector instead of moving. Moving the mouse
/// over a widget focuses it too.
#[derive(Default)]
#[expect(
    clippy::struct_excessive_bools,
    reason = "independent input flags of one frame"
)]
struct Nav {
    focus: usize,
    /// Widgets registered so far this frame.
    count: usize,
    /// Where this frame's widgets are and what they are.
    places: Vec<Place>,
    /// The same for the frame before, to move focus by.
    last_places: Vec<Place>,
    /// Panels drawn so far this frame: a widget inside one belongs to that section of the screen.
    panels: Vec<Rect>,
    /// A new screen: the focus has yet to settle on its first widget that works.
    fresh: bool,
    /// Where the focused widget is, to find it again when widgets come and go around it.
    focus_rect: Option<Rect>,
    enabled: bool,
    activate: bool,
    left: bool,
    right: bool,
}

/// A widget as keyboard navigation sees it.
#[derive(Clone, Copy)]
struct Place {
    rect: Rect,
    /// Left/Right change its value instead of moving.
    selector: bool,
    /// The arrow keys skip a greyed-out button, and hovering doesn't focus it.
    enabled: bool,
    /// Which panel it is in (0: none): the arrow keys stay inside a section while they can.
    section: usize,
}

thread_local! {
    /// A button or selector was used this frame (the main loop plays a click).
    static CLICKED: Cell<bool> = const { Cell::new(false) };
}

/// True once after a button or selector was used.
pub fn take_click() -> bool {
    CLICKED.replace(false)
}

fn note_click(used: bool) -> bool {
    if used {
        CLICKED.set(true);
    }
    used
}

thread_local! {
    static NAV: RefCell<Nav> = RefCell::new(Nav::default());
    /// A menu key pressed for this frame by `--keys` (a developer aid), as if on the keyboard.
    static SCRIPTED: Cell<Option<KeyCode>> = const { Cell::new(None) };
}

/// Presses `key` in the menus for the frame about to start (`--keys`).
pub fn script_key(key: Option<KeyCode>) {
    SCRIPTED.set(key);
}

/// A menu key went down this frame, on the keyboard or from `--keys`.
fn pressed(key: KeyCode) -> bool {
    is_key_pressed(key) || SCRIPTED.get() == Some(key)
}

/// Call once per frame, before any widget. `keyboard` is false during
/// gameplay, where the arrow keys aim the tank.
pub fn begin_frame(keyboard: bool) {
    NAV.with_borrow_mut(|nav| {
        let widgets = nav.count;
        nav.count = 0;
        nav.last_places = std::mem::take(&mut nav.places);
        nav.panels.clear();
        // The focus is kept as a place in drawing order. When a screen shows or hides widgets (the ladder card in
        // place of three rows), the same place is another widget: follow the one that had the focus.
        if let Some(index) = nav
            .focus_rect
            .and_then(|rect| moved_to(&nav.last_places, nav.focus, rect))
        {
            nav.focus = index;
        }
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
        if nav.fresh {
            nav.fresh = false;
            nav.focus = nav.last_places.iter().position(|p| p.enabled).unwrap_or(0);
        }
        // A focused button that greys out (bought, and now too dear) keeps the focus, so it doesn't jump away
        // under the player's hand; it just can't be pressed.
        let selector = nav.last_places.get(nav.focus).is_some_and(|p| p.selector);
        let (left, right) = (pressed(KeyCode::Left), pressed(KeyCode::Right));
        let dir = if pressed(KeyCode::Down) {
            Some(vec2(0.0, 1.0))
        } else if pressed(KeyCode::Up) {
            Some(vec2(0.0, -1.0))
        } else if left && !selector {
            Some(vec2(-1.0, 0.0))
        } else if right && !selector {
            Some(vec2(1.0, 0.0))
        } else {
            None
        };
        if let Some(next) = dir.and_then(|dir| step(&nav.last_places, nav.focus, dir)) {
            nav.focus = next;
        }
        nav.focus_rect = nav.last_places.get(nav.focus).map(|p| p.rect);
        nav.activate = pressed(KeyCode::Enter) || pressed(KeyCode::Space);
        nav.left = selector && left;
        nav.right = selector && right;
    });
}

/// Call when the screen changes, so focus starts at the first widget that works.
pub fn reset_focus() {
    NAV.with_borrow_mut(|nav| {
        nav.focus = 0;
        nav.fresh = true;
        nav.focus_rect = None;
        // The old screen's widgets say nothing about the new one: no keyboard focus until it has been drawn once.
        nav.count = 0;
        nav.places.clear();
    });
}

#[expect(
    clippy::struct_excessive_bools,
    reason = "independent input flags of one widget"
)]
struct Focus {
    focused: bool,
    activate: bool,
    left: bool,
    right: bool,
}

/// Where the focus goes from widget `from` when an arrow key points `dir` (a unit vector along one axis), or `None` to
/// stay. Greyed-out widgets are skipped. In order:
/// 1. the nearest widget that way in the same row or column, in the same section (panel);
/// 2. Up/Down only: the nearest widget that way in the same section, wherever it is;
/// 3. the nearest widget that way in the same row or column in another section;
/// 4. Up/Down only: around to the far end of the column;
/// 5. Up/Down only: the next or previous widget in drawing order.
///
/// A widget is "that way" only if its near edge is past the middle of `from`, so a wide button below isn't to the
/// right. Left/Right never leave the row: a jump to another row would land somewhere unexpected.
fn step(places: &[Place], from: usize, dir: Vec2) -> Option<usize> {
    /// Another widget, as seen from the focused one.
    struct Other {
        index: usize,
        /// Distance that way, centre to centre (negative: behind).
        along: f32,
        /// How far out of line its centre is, and the gap between the two across the way (0: same row or column).
        across: f32,
        gap: f32,
        same_section: bool,
        ahead: bool,
        behind: bool,
    }
    let me = places.get(from)?;
    let vertical = dir.y != 0.0;
    let side = vec2(dir.y.abs(), dir.x.abs());
    let centre = me.rect.center();
    let span = |r: Rect, axis: Vec2| {
        let (a, b) = (r.point().dot(axis), (r.point() + r.size()).dot(axis));
        (a.min(b), a.max(b))
    };
    let (a0, a1) = span(me.rect, side);
    let others: Vec<Other> = places
        .iter()
        .enumerate()
        .filter(|&(i, p)| i != from && p.enabled)
        .map(|(index, p)| {
            let (b0, b1) = span(p.rect, side);
            let to = p.rect.center() - centre;
            let gap = (b0 - a1).max(a0 - b1).max(0.0);
            Other {
                index,
                along: to.dot(dir),
                across: to.dot(side).abs(),
                gap,
                same_section: p.section == me.section,
                ahead: span(p.rect, dir).0 >= centre.dot(dir),
                behind: span(p.rect, -dir).0 >= centre.dot(-dir),
            }
        })
        .collect();
    let best = |keep: fn(&Other) -> bool, score: fn(&Other) -> f32| {
        others
            .iter()
            .filter(|o| keep(o))
            .min_by(|a, b| score(a).total_cmp(&score(b)))
            .map(|o| o.index)
    };
    // Nearest that way; of two equally near, the one more in line.
    let near: fn(&Other) -> f32 = |o| o.along + 0.01 * o.across;
    let in_drawing_order = || {
        let n = places.len();
        let by = if dir.y > 0.0 { 1 } else { n - 1 };
        (1..n)
            .map(|k| (from + k * by) % n)
            .find(|&i| places[i].enabled)
    };
    best(|o| o.ahead && o.gap <= 0.0 && o.same_section, near)
        .or_else(|| {
            vertical
                .then(|| best(|o| o.ahead && o.same_section, |o| o.along + 3.0 * o.gap))
                .flatten()
        })
        .or_else(|| best(|o| o.ahead && o.gap <= 0.0, near))
        .or_else(|| {
            vertical
                .then(|| best(|o| o.behind && o.gap <= 0.0, near))
                .flatten()
        })
        .or_else(|| vertical.then(in_drawing_order).flatten())
}

/// The new index of the widget that was at `rect`, if it is no longer at index `focus` but still on screen.
fn moved_to(places: &[Place], focus: usize, rect: Rect) -> Option<usize> {
    let same = |p: &Place| {
        let (a, b) = (p.rect, rect);
        (a.x - b.x).abs() < 0.5 && (a.y - b.y).abs() < 0.5 && (a.w - b.w).abs() < 0.5
    };
    if places.get(focus).is_some_and(same) {
        return None;
    }
    places.iter().position(same)
}

fn register(hovered: bool, rect: Rect, selector: bool, enabled: bool) -> Focus {
    NAV.with_borrow_mut(|nav| {
        let id = nav.count;
        nav.count += 1;
        // The last panel drawn around it (a dialog's panel over the screen's).
        let section = nav
            .panels
            .iter()
            .rposition(|p| p.contains(rect.center()))
            .map_or(0, |k| k + 1);
        let place = Place {
            rect,
            selector,
            enabled,
            section,
        };
        nav.places.push(place);
        if hovered && place.enabled && mouse_delta_position() != Vec2::ZERO {
            nav.focus = id;
            nav.focus_rect = Some(rect);
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

/// The largest text size, `size` at most, at which every one of `texts` fits within `width`: one size for a set
/// of labels that belong together.
pub fn common_size(texts: &[String], size: f32, width: f32) -> f32 {
    texts
        .iter()
        .map(|text| fit(text, size, width).0)
        .fold(size, f32::min)
}

/// A button with `left` at its left edge and `right` at its right one, both at `size`: a row of a table that can
/// be pressed.
pub fn button_row(r: Rect, left: &str, right: &str, size: f32, enabled: bool) -> bool {
    let hovered = r.contains(mouse());
    let focus = register(hovered, r, false, enabled);
    widget_background(r, enabled, hovered || focus.focused, focus.focused);
    let color = if enabled { WHITE } else { GRAY };
    let dims = measure(right, size);
    let y = r.y + f32::midpoint(r.h, dims.offset_y);
    text_at(left, r.x + 8.0, y, size, color);
    text_at(right, r.x + r.w - 8.0 - dims.width, y, size, color);
    note_click(
        enabled && ((hovered && is_mouse_button_pressed(MouseButton::Left)) || focus.activate),
    )
}

pub fn button(r: Rect, label: &str, enabled: bool) -> bool {
    let hovered = r.contains(mouse());
    let focus = register(hovered, r, false, enabled);
    widget_background(r, enabled, hovered || focus.focused, focus.focused);

    let (size, dims) = fit(label, 22.0, r.w - 16.0);
    text_at(
        label,
        r.x + (r.w - dims.width) / 2.0,
        r.y + f32::midpoint(r.h, dims.offset_y),
        size,
        if enabled { WHITE } else { GRAY },
    );
    note_click(
        enabled && ((hovered && is_mouse_button_pressed(MouseButton::Left)) || focus.activate),
    )
}

/// A row "Label        < value >". Click the left arrow area to go back,
/// anywhere else to go forward; Left/Right/Enter when focused.
/// Returns `true` when the value changed.
pub fn selector(r: Rect, label: &str, values: &[impl AsRef<str>], index: &mut usize) -> bool {
    let hovered = r.contains(mouse());
    let focus = register(hovered, r, true, true);
    widget_background(r, true, hovered || focus.focused, focus.focused);

    let value_box = Rect::new(r.x + r.w * 0.5, r.y, r.w * 0.5, r.h);
    let (size, dims) = fit(label, 22.0, r.w * 0.5 - 24.0);
    text_at(
        label,
        r.x + 14.0,
        r.y + f32::midpoint(r.h, dims.offset_y),
        size,
        LIGHTGRAY,
    );
    let value = values.get(*index).map_or("?", |v| v.as_ref());
    let (size, dims) = fit(value, 22.0, value_box.w - 60.0);
    text_at(
        value,
        value_box.x + (value_box.w - dims.width) / 2.0,
        r.y + f32::midpoint(r.h, dims.offset_y),
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
        );
    };
    arrow(value_box.x + 16.0, -1.0);
    arrow(value_box.x + value_box.w - 16.0, 1.0);

    let n = values.len();
    let clicked = hovered && is_mouse_button_pressed(MouseButton::Left);
    let back = focus.left || (clicked && mouse().x < value_box.x + 36.0);
    let forward = focus.right || focus.activate || (clicked && !back);
    if n > 1 && back {
        *index = (*index + n - 1) % n;
        note_click(true)
    } else if n > 1 && forward {
        *index = (*index + 1) % n;
        note_click(true)
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
    let mut index = usize::from(*value);
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

/// Darkens the whole screen, for a menu drawn over the battle.
pub fn dim() {
    draw_rectangle(0.0, 0.0, WIDTH, HEIGHT, Color::new(0.0, 0.0, 0.0, 0.6));
}

pub fn panel(r: Rect) {
    NAV.with_borrow_mut(|nav| nav.panels.push(r));
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

#[cfg(test)]
mod tests {
    use super::*;

    const RIGHT: Vec2 = vec2(1.0, 0.0);
    const LEFT: Vec2 = vec2(-1.0, 0.0);
    const DOWN: Vec2 = vec2(0.0, 1.0);
    const UP: Vec2 = vec2(0.0, -1.0);

    fn at(x: f32, y: f32, w: f32, section: usize) -> Place {
        Place {
            rect: Rect::new(x, y, w, 34.0),
            selector: false,
            enabled: true,
            section,
        }
    }

    /// The shop: 0-5 three rows of buy buttons in two columns, 6 Done, 7 Save & exit, 8 a wide Abandon game under
    /// both, all in one panel; 9-10 item switches in the inventory panel to the right, level with rows 2 and 3.
    fn shop() -> Vec<Place> {
        let mut places = Vec::new();
        for row in 0..3 {
            places.push(at(410.0, 174.0 + 40.0 * row as f32, 225.0, 1));
            places.push(at(645.0, 174.0 + 40.0 * row as f32, 225.0, 1));
        }
        places.push(at(410.0, 300.0, 225.0, 1));
        places.push(at(645.0, 300.0, 225.0, 1));
        places.push(at(410.0, 350.0, 460.0, 1));
        places.push(at(1150.0, 250.0, 80.0, 2));
        places.push(at(1150.0, 296.0, 80.0, 2));
        places
    }

    #[test]
    fn arrows_follow_rows_and_columns() {
        let shop = shop();
        assert_eq!(step(&shop, 0, RIGHT), Some(1));
        assert_eq!(step(&shop, 1, LEFT), Some(0));
        assert_eq!(step(&shop, 0, DOWN), Some(2));
        assert_eq!(step(&shop, 3, UP), Some(1));
        assert_eq!(step(&shop, 0, LEFT), None);
        // Done and Save & exit are side by side; the wide button under them is below both, not beside.
        assert_eq!(step(&shop, 6, RIGHT), Some(7));
        assert_eq!(step(&shop, 7, LEFT), Some(6));
        assert_eq!(step(&shop, 6, DOWN), Some(8));
        assert_eq!(step(&shop, 7, DOWN), Some(8));
    }

    #[test]
    fn greyed_out_buttons_are_skipped_along_the_column() {
        let mut shop = shop();
        shop[2].enabled = false;
        shop[4].enabled = false;
        // Down the left column to the next button that works, not across to the right column.
        assert_eq!(step(&shop, 0, DOWN), Some(6));
        // The neighbour is greyed out and nothing else is in this row: stay.
        assert_eq!(step(&shop, 3, LEFT), None);
    }

    #[test]
    fn another_panel_is_reached_only_along_a_row() {
        let shop = shop();
        // Nothing of the inventory is level with the first row.
        assert_eq!(step(&shop, 1, RIGHT), None);
        // The third row is level with the first switch, Save & exit with the second.
        assert_eq!(step(&shop, 5, RIGHT), Some(9));
        assert_eq!(step(&shop, 7, RIGHT), Some(10));
        assert_eq!(step(&shop, 10, LEFT), Some(7));
        // Down from the last switch wraps inside the inventory, not over to the shop's buttons.
        assert_eq!(step(&shop, 10, DOWN), Some(9));
    }

    #[test]
    fn the_focus_follows_its_widget_when_others_come_and_go() {
        let shop = shop();
        let focused = shop[6].rect;
        assert_eq!(moved_to(&shop, 6, focused), None, "still in its place");
        // Three widgets before it are gone: it is now the fourth.
        let fewer: Vec<Place> = shop[3..].to_vec();
        assert_eq!(moved_to(&fewer, 6, focused), Some(3));
        // It is gone itself: the focus stays where it is.
        let without: Vec<Place> = shop.iter().filter(|p| p.rect != focused).copied().collect();
        assert_eq!(moved_to(&without, 6, focused), None);
    }

    #[test]
    fn up_and_down_wrap_around_the_column() {
        let shop = shop();
        assert_eq!(step(&shop, 8, DOWN), Some(0));
        assert_eq!(step(&shop, 1, UP), Some(8));
    }

    #[test]
    fn down_finds_a_button_off_to_the_side_in_the_same_section() {
        // Three buttons in a row and one at the bottom right, like the custom bot screen.
        let places = [
            at(100.0, 400.0, 150.0, 0),
            at(260.0, 400.0, 150.0, 0),
            at(900.0, 640.0, 240.0, 0),
        ];
        assert_eq!(step(&places, 0, DOWN), Some(2));
        assert_eq!(step(&places, 2, UP), Some(1));
    }
}
