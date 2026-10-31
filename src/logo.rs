//! Main menu art: the title in wide letter spacing, and a divider with two
//! small tanks facing each other.

use macroquad::prelude::*;

use crate::ui;

const TITLE: &str = "SCORCH";
const SIZE: f32 = 100.0;
const LETTER_GAP: f32 = 20.0;

/// Draws the title centered on `cx`; returns its left edge and width.
pub fn title(cx: f32, baseline: f32) -> (f32, f32) {
    let widths: Vec<f32> = TITLE
        .chars()
        .map(|c| ui::measure(c.to_string(), SIZE).width)
        .collect();
    let total = widths.iter().sum::<f32>() + LETTER_GAP * (widths.len() - 1) as f32;
    let left = cx - total / 2.0;
    let mut x = left;
    for (c, w) in TITLE.chars().zip(&widths) {
        ui::text_at(c.to_string(), x, baseline, SIZE, ORANGE);
        x += w + LETTER_GAP;
    }
    (left, total)
}

/// A divider line centered on `cx`, with two tanks facing each other on its ends.
pub fn divider(cx: f32, y: f32, width: f32) {
    let (left, right) = (cx - width / 2.0, cx + width / 2.0);
    draw_line(left, y, right, y, 2.0, ORANGE);
    tank(left + 16.0, y, 1.0);
    tank(right - 16.0, y, -1.0);
}

/// Small white tank standing on `y`, barrel pointing right (`facing` 1) or left (-1).
fn tank(x: f32, y: f32, facing: f32) {
    draw_rectangle(x - 12.0, y - 11.0, 24.0, 11.0, WHITE);
    draw_circle(x, y - 11.0, 7.0, WHITE);
    draw_line(x, y - 13.0, x + 17.0 * facing, y - 21.0, 2.5, WHITE);
}
