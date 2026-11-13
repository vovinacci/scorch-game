//! Between-round shop: spend money earned from damage, kills and wins.

use crate::rng::gen_range;
use macroquad::prelude::*;

use crate::economy;
use crate::i18n::t;
use crate::icons;
use crate::items::ITEMS;
use crate::player::Player;
use crate::ui;
use crate::weapons::{Kind, WEAPONS};

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
    /// Height of a row of buy buttons, gap included.
    const ROW: f32 = 34.0;
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

    // Two columns of buttons, filled row by row.
    let col_w = (area.w - 50.0) / 2.0;
    let cell = |top: f32, k: usize| {
        Rect::new(
            area.x + 20.0 + (k % 2) as f32 * (col_w + 10.0),
            top + (k / 2) as f32 * ROW,
            col_w,
            ROW - 6.0,
        )
    };
    let top = area.y + 104.0;
    // Cheapest first.
    let mut weapons: Vec<usize> = (1..WEAPONS.len()).collect();
    weapons.sort_by_key(|&w| WEAPONS[w].price);
    // Every row in one text size, the one its longest line needs.
    let goods = |name: String, bundle: u32, price: i32| {
        (
            t!("shop-buy", weapon = name, bundle = bundle),
            t!("shop-price", price = price),
        )
    };
    let weapon_rows: Vec<(String, String)> = weapons
        .iter()
        .map(|&w| goods(WEAPONS[w].text(), WEAPONS[w].bundle, WEAPONS[w].price))
        .collect();
    let item_rows: Vec<(String, String)> = ITEMS
        .iter()
        .map(|item| goods(item.text(), item.bundle, item.price))
        .collect();
    let lines: Vec<String> = weapon_rows
        .iter()
        .chain(&item_rows)
        .map(|(name, price)| format!("{name} {price}"))
        .collect();
    let size = ui::common_size(&lines, 20.0, col_w - 16.0 - ui::row_icon_space(ROW - 6.0));
    for (k, (&w, (name, price))) in weapons.iter().zip(&weapon_rows).enumerate() {
        let can_pay = player.money >= WEAPONS[w].price;
        let icon = |centre, across, color| icons::weapon(w, centre, across, color);
        if ui::button_row(cell(top, k), icon, (name, price), size, can_pay) {
            buy(player, w);
        }
    }
    let mut y = top + weapons.len().div_ceil(2) as f32 * ROW;
    ui::text_at(t!("shop-items"), area.x + 20.0, y + 20.0, 22.0, GOLD);
    y += 28.0;
    for (i, (name, price)) in item_rows.iter().enumerate() {
        let icon = |centre, across, color| icons::item(i, centre, across, color);
        let can_pay = player.money >= ITEMS[i].price;
        if ui::button_row(cell(y, i), icon, (name, price), size, can_pay) {
            buy_item(player, i);
        }
    }
    y += ITEMS.len().div_ceil(2) as f32 * ROW;

    let half = col_w;
    if ui::button(
        Rect::new(area.x + 20.0, y + 6.0, half, 42.0),
        &t!("shop-done"),
        true,
    ) {
        ShopAction::Done
    } else if ui::button(
        Rect::new(area.x + 30.0 + half, y + 6.0, half, 42.0),
        &t!("save-exit-menu"),
        true,
    ) || is_key_pressed(KeyCode::Escape)
    {
        ShopAction::SaveAndExit
    } else if ui::button(
        Rect::new(area.x + 20.0, y + 56.0, area.w - 40.0, 38.0),
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
    let mut y = area.y + 66.0;
    for (w, weapon) in WEAPONS.iter().enumerate() {
        let count = player.ammo_label(w);
        let color = if w == 0 || player.ammo[w] > 0 {
            WHITE
        } else {
            GRAY
        };
        icons::weapon(w, vec2(area.x + 27.0, y - 6.0), 13.0, color);
        ui::text_fit(
            &weapon.text(),
            area.x + 42.0,
            y,
            area.w - 122.0,
            18.0,
            color,
        );
        let cw = ui::measure(&count, 18.0).width;
        ui::text_at(&count, area.x + area.w - 20.0 - cw, y, 18.0, color);
        y += 19.0;
    }
    // Items: one on/off switch each (off = keep them, don't use them).
    ui::text_at(t!("shop-items"), area.x + 20.0, y + 8.0, 22.0, GOLD);
    y += 20.0;
    for (i, item) in ITEMS.iter().enumerate() {
        let owned = player.items[i];
        let color = if owned > 0 { WHITE } else { GRAY };
        let count = owned.to_string();
        if !item.automatic() {
            // Used by hand in battle: nothing to switch, so a plain line like a weapon's.
            icons::item(i, vec2(area.x + 27.0, y + 13.0), 13.0, color);
            ui::text_fit(
                &item.text(),
                area.x + 42.0,
                y + 19.0,
                area.w - 122.0,
                18.0,
                color,
            );
            let cw = ui::measure(&count, 18.0).width;
            ui::text_at(&count, area.x + area.w - 20.0 - cw, y + 19.0, 18.0, color);
            y += 19.0;
            continue;
        }
        icons::item(i, vec2(area.x + 27.0, y + 13.0), 13.0, color);
        ui::text_fit(
            &item.text(),
            area.x + 42.0,
            y + 19.0,
            area.w - 192.0,
            18.0,
            color,
        );
        let cw = ui::measure(&count, 18.0).width;
        ui::text_at(&count, area.x + area.w - 112.0 - cw, y + 19.0, 18.0, color);
        let on = &mut player.items_on[i];
        let label = if *on { t!("on") } else { t!("off") };
        if ui::button(
            Rect::new(area.x + area.w - 100.0, y + 3.0, 80.0, 22.0),
            &label,
            true,
        ) {
            *on = !*on;
        }
        y += 25.0;
    }
    let (radius, damage) = crate::battle::death_blast(player);
    ui::text_wrapped(
        &t!(
            "inventory-death-blast",
            radius = radius.round(),
            damage = damage.round()
        ),
        area.x + 20.0,
        y + 22.0,
        area.w - 40.0,
        18.0,
        SKYBLUE,
    );
}

/// Last round's earnings, line by line, so the catch-up rules are visible.
fn draw_income(player: &Player, shoots_first: bool) {
    let area = Rect::new(30.0, 70.0, 320.0, 620.0);
    ui::panel(area);
    ui::text_at(t!("income-title"), area.x + 20.0, area.y + 36.0, 28.0, GOLD);

    // Every source of money with what it pays, earned or not: the panel is also where the rules are.
    let rates = [
        t!("income-rate-damage", pay = economy::DAMAGE_PAY),
        t!("income-rate-kills", pay = economy::KILL_PAY),
        t!("income-rate-bounties", pay = economy::BOUNTY_PER_WIN),
        t!("income-rate-round-win", pay = economy::WIN_PAY),
        t!("income-rate-round-pay", pay = economy::ROUND_PAY),
        t!(
            "income-rate-loss-bonus",
            pay = economy::LOSS_BONUS,
            most = economy::loss_bonus(economy::LOSS_BONUS_MAX_STREAK)
        ),
    ];
    let mut y = area.y + 76.0;
    for ((key, amount), rate) in player.income.lines().into_iter().zip(rates) {
        let (label, money) = if amount == 0 {
            (GRAY, GRAY)
        } else {
            (LIGHTGRAY, WHITE)
        };
        ui::text_fit(&t!(key), area.x + 20.0, y, area.w - 130.0, 22.0, label);
        let value = format!("+${amount}");
        let w = ui::measure(&value, 22.0).width;
        ui::text_at(&value, area.x + area.w - 20.0 - w, y, 22.0, money);
        ui::text_fit(&rate, area.x + 20.0, y + 18.0, area.w - 40.0, 16.0, GRAY);
        y += 48.0;
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
            bounty = economy::BOUNTY_PER_WIN * player.wins.cast_signed()
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

/// Bots buy at random what they can afford: weapons they fire, and now and then a parachute or a shield. First of
/// all they see to a Riot Bomb, their way out from under a Dirt Bomb: without one, digging out takes two turns, and
/// a player who buries a bot every other turn keeps it from ever firing.
pub fn ai_shop(player: &mut Player) {
    if let Some(riot) = WEAPONS.iter().position(|w| w.kind == Kind::Riot)
        && player.ammo[riot] == 0
        && player.money >= WEAPONS[riot].price
    {
        buy(player, riot);
    }
    // Bots never fire Dirt Bombs, Diggers or Tracers, so they don't buy them either. A Riot Bomb digs them out when
    // they get buried.
    let weapons: Vec<usize> = (1..WEAPONS.len())
        .filter(|&w| !WEAPONS[w].kind.is_utility() || WEAPONS[w].kind == Kind::Riot)
        .collect();
    for _ in 0..8 {
        let pick = gen_range(0, weapons.len() + ITEMS.len());
        if let Some(&w) = weapons.get(pick) {
            if player.money >= WEAPONS[w].price {
                buy(player, w);
            }
        } else {
            let i = pick - weapons.len();
            if player.money >= ITEMS[i].price {
                buy_item(player, i);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::PRESETS;
    use macroquad::prelude::WHITE;

    fn riot() -> usize {
        WEAPONS.iter().position(|w| w.kind == Kind::Riot).unwrap()
    }

    fn bot(money: i32) -> Player {
        let mut player = Player::new("Bot", WHITE, Some(PRESETS[1]));
        player.money = money;
        player
    }

    #[test]
    fn a_bot_sees_to_a_riot_bomb_first() {
        // Whatever the random picks turn out to be, the pack is bought before them.
        for _ in 0..20 {
            let mut player = bot(WEAPONS[riot()].price);
            ai_shop(&mut player);
            assert_eq!(player.ammo[riot()], WEAPONS[riot()].bundle);
            assert_eq!(player.money, 0);
        }
    }

    #[test]
    fn a_bot_with_riot_bombs_left_spends_its_money_on_other_things() {
        let mut player = bot(0);
        player.ammo[riot()] = 1;
        ai_shop(&mut player);
        assert_eq!(player.ammo[riot()], 1, "no money, nothing bought");
        // One short of the pack: it can't have it yet.
        let mut player = bot(WEAPONS[riot()].price - 1);
        ai_shop(&mut player);
        assert_eq!(player.ammo[riot()], 0);
    }
}
