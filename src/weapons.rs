//! Weapon table. Adding a weapon = one new entry here (plus a `Kind` if it behaves differently).

use crate::i18n::t;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Explodes on impact: damages tanks and carves terrain.
    Blast,
    /// Splits into 5 warheads at the top of its arc.
    Mirv,
    /// Adds a mound of dirt instead of exploding. Great for burying enemies.
    Dirt,
}

pub struct Weapon {
    pub name: &'static str,
    pub kind: Kind,
    pub radius: f32,
    pub damage: f32,
    pub price: i32,
    /// How many shots one purchase gives.
    pub bundle: u32,
}

impl Weapon {
    /// Name on screen, in the current language.
    pub fn text(&self) -> String {
        t!(&format!(
            "weapon-{}",
            self.name.to_lowercase().replace(' ', "-")
        ))
    }
}

/// Index 0 is the default weapon with infinite ammo.
pub const WEAPONS: [Weapon; 5] = [
    Weapon {
        name: "Baby Missile",
        kind: Kind::Blast,
        radius: 20.0,
        damage: 30.0,
        price: 0,
        bundle: 0,
    },
    Weapon {
        name: "Missile",
        kind: Kind::Blast,
        radius: 35.0,
        damage: 55.0,
        price: 60,
        bundle: 3,
    },
    Weapon {
        name: "MIRV",
        kind: Kind::Mirv,
        radius: 25.0,
        damage: 35.0,
        price: 120,
        bundle: 1,
    },
    Weapon {
        name: "Nuke",
        kind: Kind::Blast,
        radius: 85.0,
        damage: 100.0,
        price: 200,
        bundle: 1,
    },
    Weapon {
        name: "Dirt Bomb",
        kind: Kind::Dirt,
        radius: 45.0,
        damage: 0.0,
        price: 40,
        bundle: 2,
    },
];

pub const WEAPON_COUNT: usize = WEAPONS.len();
