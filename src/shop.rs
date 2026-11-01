//! Between-round shop: spend money earned from damage, kills and wins.

use macroquad::prelude::*;
use macroquad::rand::gen_range;

use crate::economy;
use crate::i18n::t;
use crate::items::ITEMS;
use crate::player::Player;
use crate::ui;
use crate::weapons::WEAPONS;

fn buy(player: &mut Player, w: usize) {
    player.money -= WEAPONS[w].price;
    player.ammo[w] += WEAPONS[w].bundle;
}

fn buy_item(player: &mut Player, i: usize) {
    player.money -= ITEMS[i].price;
    player.items[i] += ITEMS[i].bundle;
}

pub enum ShopAction {
    Stay,
    Done,
    SaveAndExit,
    Abandon,
}

/// Draws the between-round screen for one human player: last round's
/// earnings (left), the shop (middle) and the inventory (right).
pub fn frame(player: &mut Player, shoots_first: bool) -> ShopAction {
    draw_income(player, shoots_first);
    let action = shop_panel(player);
    // After the shop's buttons, so keyboard focus starts on the first thing to buy, not on an item switch.
    draw_inventory(player);
    action
}

/// The middle panel: weapons and items to buy, and Done / Save & exit / Abandon.
fn shop_panel(player: &mut Player) -> ShopAction {
    let area = Rect::new(370.0, 70.0, 540.0, 620.0);
    ui::panel(area);
    // Hotseat players need to see whose turn it is to shop.
    let title = if player.is_you() {
        t!("shop-title")
    } else {
        t!("shop-title-for", name = player.label())
    };
    ui::text_centered(&title, area.center().x, area.y + 46.0, 38.0, player.color);
    ui::text_centered(
        &t!("shop-money", money = player.money),
        area.center().x,
        area.y + 82.0,
        26.0,
        GOLD,
    );

    let row = |y: f32| Rect::new(area.x + 40.0, y, area.w - 80.0, 38.0);
    let mut y = area.y + 104.0;
    for (w, weapon) in WEAPONS.iter().enumerate().skip(1) {
        let label = t!(
            "shop-buy",
            weapon = weapon.text(),
            bundle = weapon.bundle,
            price = weapon.price
        );
        if ui::button(row(y), &label, player.money >= weapon.price) {
            buy(player, w);
        }
        y += 44.0;
    }
    ui::text_at(t!("shop-items"), area.x + 40.0, y + 22.0, 22.0, GOLD);
    y += 32.0;
    for (i, item) in ITEMS.iter().enumerate() {
        let label = t!(
            "shop-buy",
            weapon = item.text(),
            bundle = item.bundle,
            price = item.price
        );
        if ui::button(row(y), &label, player.money >= item.price) {
            buy_item(player, i);
        }
        y += 44.0;
    }

    let half = (area.w - 90.0) / 2.0;
    if ui::button(
        Rect::new(area.x + 40.0, y + 12.0, half, 46.0),
        &t!("shop-done"),
        true,
    ) {
        ShopAction::Done
    } else if ui::button(
        Rect::new(area.x + 50.0 + half, y + 12.0, half, 46.0),
        &t!("save-exit-menu"),
        true,
    ) || is_key_pressed(KeyCode::Escape)
    {
        ShopAction::SaveAndExit
    } else if ui::button(
        Rect::new(area.x + 40.0, y + 68.0, area.w - 80.0, 40.0),
        &t!("abandon-game"),
        true,
    ) {
        ShopAction::Abandon
    } else {
        ShopAction::Stay
    }
}

/// What the player owns, a switch per item, and what it means if their tank dies carrying ammo.
fn draw_inventory(player: &mut Player) {
    let area = Rect::new(930.0, 70.0, 320.0, 620.0);
    ui::panel(area);
    ui::text_at(
        t!("inventory-title"),
        area.x + 20.0,
        area.y + 36.0,
        28.0,
        GOLD,
    );
    let mut y = area.y + 72.0;
    for (w, weapon) in WEAPONS.iter().enumerate() {
        let count = player.ammo_label(w);
        let color = if w == 0 || player.ammo[w] > 0 {
            WHITE
        } else {
            GRAY
        };
        ui::text_fit(
            &weapon.text(),
            area.x + 20.0,
            y,
            area.w - 100.0,
            22.0,
            color,
        );
        let cw = ui::measure(&count, 22.0).width;
        ui::text_at(&count, area.x + area.w - 20.0 - cw, y, 22.0, color);
        y += 28.0;
    }
    // Items: one on/off switch each (off = keep them, don't use them).
    ui::text_at(t!("shop-items"), area.x + 20.0, y + 8.0, 22.0, GOLD);
    y += 22.0;
    for (i, item) in ITEMS.iter().enumerate() {
        let owned = player.items[i];
        let color = if owned > 0 { WHITE } else { GRAY };
        ui::text_fit(
            &item.text(),
            area.x + 20.0,
            y + 25.0,
            area.w - 170.0,
            22.0,
            color,
        );
        let count = owned.to_string();
        let cw = ui::measure(&count, 22.0).width;
        ui::text_at(&count, area.x + area.w - 112.0 - cw, y + 25.0, 22.0, color);
        let on = &mut player.items_on[i];
        let label = if *on { t!("on") } else { t!("off") };
        if ui::button(
            Rect::new(area.x + area.w - 100.0, y + 2.0, 80.0, 32.0),
            &label,
            true,
        ) {
            *on = !*on;
        }
        y += 38.0;
    }
    let (radius, damage) = crate::battle::death_blast(player);
    ui::text_wrapped(
        &t!(
            "inventory-death-blast",
            radius = radius.round(),
            damage = damage.round()
        ),
        area.x + 20.0,
        y + 24.0,
        area.w - 40.0,
        20.0,
        SKYBLUE,
    );
}

/// Last round's earnings, line by line, so the catch-up rules are visible.
fn draw_income(player: &Player, shoots_first: bool) {
    let area = Rect::new(30.0, 70.0, 320.0, 420.0);
    ui::panel(area);
    ui::text_at(t!("income-title"), area.x + 20.0, area.y + 36.0, 28.0, GOLD);

    let mut y = area.y + 76.0;
    for (key, amount) in player.income.lines() {
        if amount == 0 {
            continue;
        }
        ui::text_fit(&t!(key), area.x + 20.0, y, area.w - 130.0, 22.0, LIGHTGRAY);
        let value = format!("+${amount}");
        let w = ui::measure(&value, 22.0).width;
        ui::text_at(&value, area.x + area.w - 20.0 - w, y, 22.0, WHITE);
        y += 30.0;
    }
    draw_line(
        area.x + 20.0,
        y - 10.0,
        area.x + area.w - 20.0,
        y - 10.0,
        1.0,
        GRAY,
    );
    let total = format!("+${}", player.income.total());
    let w = ui::measure(&total, 24.0).width;
    ui::text_at(t!("income-total"), area.x + 20.0, y + 16.0, 24.0, WHITE);
    ui::text_at(&total, area.x + area.w - 20.0 - w, y + 16.0, 24.0, GOLD);

    let mut notes = Vec::new();
    if player.loss_streak > 0 {
        notes.push(t!(
            "income-note-losses",
            n = player.loss_streak,
            bonus = economy::loss_bonus(player.loss_streak + 1)
        ));
    }
    if player.wins > 0 {
        notes.push(t!(
            "income-note-bounty",
            n = player.wins,
            bounty = economy::BOUNTY_PER_WIN * player.wins as i32
        ));
    }
    if shoots_first {
        notes.push(t!("income-note-first"));
    }
    let mut y = y + 60.0;
    for note in notes {
        y = ui::text_wrapped(&note, area.x + 20.0, y, area.w - 40.0, 20.0, SKYBLUE) + 8.0;
    }
}

/// Bots buy at random what they can afford: weapons, and now and then a parachute or a shield.
pub fn ai_shop(player: &mut Player) {
    let weapons = WEAPONS.len() - 1;
    for _ in 0..8 {
        let pick = gen_range(0, weapons + ITEMS.len());
        if pick < weapons {
            let w = pick + 1;
            if player.money >= WEAPONS[w].price {
                buy(player, w);
            }
        } else {
            let i = pick - weapons;
            if player.money >= ITEMS[i].price {
                buy_item(player, i);
            }
        }
    }
}
