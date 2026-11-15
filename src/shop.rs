//! Between-round shop: spend money earned from damage, kills and wins.

use crate::rng::gen_range;
use std::cell::Cell;

use macroquad::prelude::*;

use crate::economy::{self, Interest, Market};
use crate::i18n::t;
use crate::icons;
use crate::items::ITEMS;
use crate::player::Player;
use crate::ui;
use crate::weapons::{Kind, WEAPONS};

thread_local! {
    /// The shop shows what you can sell instead of what you can buy.
    static SELLING: Cell<bool> = const { Cell::new(false) };
}

/// The two tabs at the right of the money, Buy and Sell; returns whether the shop sells.
fn buy_or_sell(area: Rect) -> bool {
    let selling = SELLING.get();
    let tabs = [(false, t!("shop-tab-buy")), (true, t!("shop-tab-sell"))];
    for (k, (mode, label)) in tabs.into_iter().enumerate() {
        let r = Rect::new(
            area.x + area.w - 210.0 + k as f32 * 98.0,
            area.y + 64.0,
            92.0,
            30.0,
        );
        let shown = if mode == selling {
            format!("[{label}]")
        } else {
            label
        };
        if ui::button(r, &shown, true) {
            SELLING.set(mode);
        }
    }
    SELLING.get()
}

/// Selling pays this share of what a pack costs.
pub const SELL_SHARE: f32 = 0.5;

/// Something the shop sells: a weapon or an item.
#[derive(Clone, Copy)]
pub enum Ware {
    Weapon(usize),
    Item(usize),
}

impl Ware {
    /// Every ware, weapons then items, in the order of `slot`.
    pub fn all() -> impl Iterator<Item = Ware> {
        (0..WEAPONS.len())
            .map(Ware::Weapon)
            .chain((0..ITEMS.len()).map(Ware::Item))
    }

    /// Its place in the market's lists.
    pub fn slot(self) -> usize {
        match self {
            Ware::Weapon(w) => w,
            Ware::Item(i) => WEAPONS.len() + i,
        }
    }

    /// The price on the list.
    fn list(self) -> i32 {
        match self {
            Ware::Weapon(w) => WEAPONS[w].price,
            Ware::Item(i) => ITEMS[i].price,
        }
    }

    /// What a pack costs now.
    fn price(self, market: &Market) -> i32 {
        market.price(self.slot(), self.list())
    }

    /// What a pack costs now (for the balance tools).
    #[cfg(test)]
    pub fn cost(self, market: &Market) -> i32 {
        self.price(market)
    }

    /// Its English name (for the balance tools).
    #[cfg(test)]
    pub fn name(self) -> &'static str {
        match self {
            Ware::Weapon(w) => WEAPONS[w].name,
            Ware::Item(i) => ITEMS[i].name,
        }
    }

    fn bundle(self) -> u32 {
        match self {
            Ware::Weapon(w) => WEAPONS[w].bundle,
            Ware::Item(i) => ITEMS[i].bundle,
        }
    }

    fn text(self) -> String {
        match self {
            Ware::Weapon(w) => WEAPONS[w].text(),
            Ware::Item(i) => ITEMS[i].text(),
        }
    }

    fn owned(self, player: &Player) -> u32 {
        match self {
            Ware::Weapon(w) => player.ammo[w],
            Ware::Item(i) => player.items[i],
        }
    }

    fn owned_mut(self, player: &mut Player) -> &mut u32 {
        match self {
            Ware::Weapon(w) => &mut player.ammo[w],
            Ware::Item(i) => &mut player.items[i],
        }
    }

    fn icon(self, centre: Vec2, across: f32, color: Color) {
        match self {
            Ware::Weapon(w) => icons::weapon(w, centre, across, color),
            Ware::Item(i) => icons::item(i, centre, across, color),
        }
    }

    /// The row's name and price; a market price that has moved shows the list price beside it.
    fn buy_label(self, market: &Market) -> (String, String) {
        let price = self.price(market);
        let shown = if price == self.list() {
            t!("shop-price", price = price)
        } else {
            t!("shop-price-moved", price = price, list = self.list())
        };
        (
            t!("shop-buy", weapon = self.text(), bundle = self.bundle()),
            shown,
        )
    }

    fn buy(self, player: &mut Player, market: &mut Market) {
        player.money -= self.price(market);
        *self.owned_mut(player) += self.bundle();
        market.trade(self.slot(), 1);
    }

    /// What selling this now takes away and pays: a pack, or what is left of one, at `SELL_SHARE` of its price.
    fn sale(self, player: &Player, market: &Market) -> (u32, i32) {
        let count = self.owned(player).min(self.bundle());
        let refund = self.price(market) as f32 * SELL_SHARE * count as f32 / self.bundle() as f32;
        (count, refund as i32)
    }

    fn sell(self, player: &mut Player, market: &mut Market) {
        let (count, refund) = self.sale(player, market);
        *self.owned_mut(player) -= count;
        player.money += refund;
        market.trade(self.slot(), -1);
    }
}

pub enum ShopAction {
    Stay,
    Done,
    SaveAndExit,
    Abandon,
}

/// Draws the between-round screen for one human player: last round's
/// earnings (left), the shop (middle) and the inventory (right).
pub fn frame(
    player: &mut Player,
    shoots_first: bool,
    interest: Interest,
    market: &mut Market,
) -> ShopAction {
    draw_income(player, shoots_first, interest);
    let action = shop_panel(player, market);
    // After the shop's buttons, so keyboard focus starts on the first thing to buy, not on an item switch.
    draw_inventory(player);
    action
}

/// The middle panel: weapons and items to buy, and Done / Save & exit / Abandon.
fn shop_panel(player: &mut Player, market: &mut Market) -> ShopAction {
    /// Height of a row of buy buttons, gap included.
    const ROW: f32 = 31.0;
    let area = Rect::new(370.0, 70.0, 540.0, 620.0);
    ui::panel(area);
    // Hotseat players need to see whose turn it is to shop.
    let title = if player.is_you() {
        t!("shop-title")
    } else {
        t!("shop-title-for", name = player.label())
    };
    ui::text_centered(&title, area.center().x, area.y + 46.0, 38.0, player.color);
    // The money at the left, the Buy and Sell tabs at the right (`buy_or_sell`).
    ui::text_fit(
        &t!("shop-money", money = player.money),
        area.x + 20.0,
        area.y + 86.0,
        area.w - 230.0,
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
    let selling = buy_or_sell(area);
    let wares: Vec<Ware> = weapons
        .iter()
        .map(|&w| Ware::Weapon(w))
        .chain((0..ITEMS.len()).map(Ware::Item))
        .collect();
    // Every row in one text size, the one its longest line needs, buying or selling.
    let lines: Vec<String> = wares
        .iter()
        .flat_map(|ware| {
            let (name, price) = ware.buy_label(market);
            let sell = t!("shop-sell-price", price = ware.price(market));
            [format!("{name} {price}"), format!("{name} {sell}")]
        })
        .collect();
    let size = ui::common_size(&lines, 20.0, col_w - 16.0 - ui::row_icon_space(ROW - 6.0));
    let mut y = top;
    for (k, &ware) in wares.iter().enumerate() {
        let first_item = matches!(ware, Ware::Item(0));
        if first_item {
            y += weapons.len().div_ceil(2) as f32 * ROW;
            ui::text_at(t!("shop-items"), area.x + 20.0, y + 20.0, 22.0, GOLD);
            y += 28.0;
        }
        let k = if let Ware::Item(i) = ware { i } else { k };
        let icon = |centre, across, color| ware.icon(centre, across, color);
        if selling {
            let (count, refund) = ware.sale(player, market);
            let name = t!("shop-buy", weapon = ware.text(), bundle = count);
            let price = t!("shop-sell-price", price = refund);
            if ui::button_row(cell(y, k), icon, (&name, &price), size, count > 0) {
                ware.sell(player, market);
            }
        } else {
            let (name, price) = ware.buy_label(market);
            let can_pay = player.money >= ware.price(market);
            if ui::button_row(cell(y, k), icon, (&name, &price), size, can_pay) {
                ware.buy(player, market);
            }
        }
    }
    y += ITEMS.len().div_ceil(2) as f32 * ROW;

    let half = col_w;
    if ui::button(
        Rect::new(area.x + 20.0, y + 6.0, half, 42.0),
        &t!("shop-done"),
        true,
    ) {
        SELLING.set(false);
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
fn draw_income(player: &Player, shoots_first: bool, interest: Interest) {
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
        t!(
            "income-rate-interest",
            rate = (interest.rate() * 100.0).round(),
            most = economy::INTEREST_MAX
        ),
    ];
    let mut y = area.y + 76.0;
    let lines = player.income.lines().into_iter().zip(rates);
    // Interest only where the game has it.
    for ((key, amount), rate) in
        lines.filter(|((key, _), _)| *key != "income-interest" || interest.rate() > 0.0)
    {
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
pub fn ai_shop(player: &mut Player, market: &mut Market) {
    if let Some(riot) = WEAPONS.iter().position(|w| w.kind == Kind::Riot) {
        let riot = Ware::Weapon(riot);
        if riot.owned(player) == 0 && player.money >= riot.price(market) {
            riot.buy(player, market);
        }
    }
    // Bots never fire Dirt Bombs, Diggers or Tracers, so they don't buy them either. A Riot Bomb digs them out when
    // they get buried.
    let weapons: Vec<usize> = (1..WEAPONS.len())
        .filter(|&w| !WEAPONS[w].kind.is_utility() || WEAPONS[w].kind == Kind::Riot)
        .collect();
    for _ in 0..8 {
        let pick = gen_range(0, weapons.len() + ITEMS.len());
        let ware = weapons
            .get(pick)
            .map_or(Ware::Item(pick.saturating_sub(weapons.len())), |&w| {
                Ware::Weapon(w)
            });
        if player.money >= ware.price(market) {
            ware.buy(player, market);
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
            ai_shop(&mut player, &mut Market::default());
            assert_eq!(player.ammo[riot()], WEAPONS[riot()].bundle);
            assert_eq!(player.money, 0);
        }
    }

    #[test]
    fn selling_pays_half_a_pack_or_its_share_of_what_is_left() {
        let missile = WEAPONS.iter().position(|w| w.name == "Missile").unwrap();
        let mut player = bot(0);
        player.ammo[missile] = 3;
        let ware = Ware::Weapon(missile);
        // A whole pack of two: half its price.
        assert_eq!(
            ware.sale(&player, &Market::default()),
            (2, WEAPONS[missile].price / 2)
        );
        ware.sell(&mut player, &mut Market::default());
        assert_eq!(
            (player.ammo[missile], player.money),
            (1, WEAPONS[missile].price / 2)
        );
        // One left of a pack of two: a quarter.
        assert_eq!(
            ware.sale(&player, &Market::default()),
            (1, WEAPONS[missile].price / 4)
        );
        ware.sell(&mut player, &mut Market::default());
        assert_eq!(player.ammo[missile], 0);
        assert_eq!(
            ware.sale(&player, &Market::default()),
            (0, 0),
            "nothing left to sell"
        );
        // Items too: 37 units of fuel out of a pack of 100.
        let fuel = Ware::Item(crate::items::FUEL);
        player.items[crate::items::FUEL] = 37;
        assert_eq!(fuel.sale(&player, &Market::default()), (37, 7));
        // Buying it back costs the full price: selling is no way to make money.
        let before = player.money;
        ware.buy(&mut player, &mut Market::default());
        ware.sell(&mut player, &mut Market::default());
        assert!(player.money < before);
    }

    #[test]
    fn a_bot_with_riot_bombs_left_spends_its_money_on_other_things() {
        let mut player = bot(0);
        player.ammo[riot()] = 1;
        ai_shop(&mut player, &mut Market::default());
        assert_eq!(player.ammo[riot()], 1, "no money, nothing bought");
        // One short of the pack: it can't have it yet.
        let mut player = bot(WEAPONS[riot()].price - 1);
        ai_shop(&mut player, &mut Market::default());
        assert_eq!(player.ammo[riot()], 0);
    }
}
