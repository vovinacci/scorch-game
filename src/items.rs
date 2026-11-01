//! Defence items: parachutes and shields. Bought in the shop like weapons, switched on or off per item in the
//! inventory. Adding an item = one new entry here (plus a `Use` if it behaves differently).

use crate::i18n::t;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Use {
    /// Opens by itself when a fall would hurt: the tank drifts down without damage. One per fall.
    Parachute,
    /// Raised at round start; absorbs this much damage before health is touched.
    Shield { energy: u32 },
    /// A weaker shield that also pushes incoming enemy shells away.
    Deflector { energy: u32 },
}

pub struct Item {
    pub name: &'static str,
    pub kind: Use,
    pub price: i32,
    /// How many one purchase gives.
    pub bundle: u32,
}

impl Item {
    /// Name on screen, in the current language.
    pub fn text(&self) -> String {
        t!(&format!(
            "item-{}",
            self.name.to_lowercase().replace(' ', "-")
        ))
    }

    pub fn shield_energy(&self) -> Option<f32> {
        match self.kind {
            Use::Shield { energy } | Use::Deflector { energy } => Some(energy as f32),
            Use::Parachute => None,
        }
    }
}

pub const PARACHUTE: usize = 0;

pub const ITEMS: [Item; 4] = [
    Item {
        name: "Parachute",
        kind: Use::Parachute,
        price: 40,
        bundle: 2,
    },
    Item {
        name: "Shield",
        kind: Use::Shield { energy: 50 },
        price: 120,
        bundle: 1,
    },
    Item {
        name: "Heavy Shield",
        kind: Use::Shield { energy: 100 },
        price: 200,
        bundle: 1,
    },
    Item {
        name: "Mag Deflector",
        kind: Use::Deflector { energy: 40 },
        price: 150,
        bundle: 1,
    },
];

pub const ITEM_COUNT: usize = ITEMS.len();

/// Which shield goes up at round start: the strongest one owned and switched on.
pub fn shield_to_raise(owned: &[u32; ITEM_COUNT], on: &[bool; ITEM_COUNT]) -> Option<usize> {
    (0..ITEM_COUNT)
        .filter(|&i| owned[i] > 0 && on[i])
        .filter_map(|i| Some((i, ITEMS[i].shield_energy()?)))
        .max_by(|a, b| a.1.total_cmp(&b.1))
        .map(|(i, _)| i)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_strongest_switched_on_shield_goes_up() {
        let owned = [3, 1, 1, 1];
        assert_eq!(
            shield_to_raise(&owned, &[true; ITEM_COUNT]),
            Some(2),
            "Heavy Shield"
        );
        assert_eq!(
            shield_to_raise(&owned, &[true, true, false, true]),
            Some(1),
            "Shield beats Deflector"
        );
        assert_eq!(
            shield_to_raise(&owned, &[true, false, false, false]),
            None,
            "parachutes aren't shields"
        );
        assert_eq!(shield_to_raise(&[0; ITEM_COUNT], &[true; ITEM_COUNT]), None);
    }
}
