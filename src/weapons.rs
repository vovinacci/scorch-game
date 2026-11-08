//! Weapon table. Adding a weapon = one new entry here (plus a `Kind` if it behaves differently).

use crate::i18n::t;

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Kind {
    /// Explodes on impact: damages tanks and carves terrain.
    Blast,
    /// Splits into `warheads` at the top of its arc, `spread` pixels per second apart sideways.
    Mirv { warheads: u8, spread: u8 },
    /// Adds a mound of dirt instead of exploding. Great for burying enemies.
    Dirt,
    /// Lands, rolls downhill and explodes in the next dip, against a tank or a slope it can't climb.
    Roller,
    /// Explodes, then hops on and explodes again, `hops` more times.
    Leapfrog { hops: u8 },
    /// Explodes and throws `bomblets` small bombs around at random.
    Funky { bomblets: u8 },
    /// Spills burning fuel that runs downhill; `damage` is per second of burning.
    Napalm,
    /// Burrows sideways under the ground, so the dirt above it sinks. No damage.
    Digger,
    /// Harmless: only draws its whole path, to help aim the next shot.
    Tracer,
    /// Clears the dirt around where it lands without hurting anyone.
    Riot,
}

impl Kind {
    /// Does no damage (it changes the ground, or only shows a path); bots don't buy or fire these.
    pub fn is_utility(self) -> bool {
        matches!(self, Kind::Dirt | Kind::Digger | Kind::Tracer | Kind::Riot)
    }

    /// Not a shot at anyone, only a look at the path: it doesn't count for accuracy, isn't logged as a miss and
    /// doesn't drain Mag Deflectors.
    pub fn is_probe(self) -> bool {
        self == Kind::Tracer
    }

    /// Sideways speed of the outermost MIRV warhead relative to the shell, in pixels per second (0 for other
    /// weapons). Times the fall from the top of the arc, it's how far from the shell's landing the outer blasts land.
    pub fn side_speed(self) -> f32 {
        match self {
            Kind::Mirv { warheads, spread } => f32::from(warheads - 1) / 2.0 * f32::from(spread),
            _ => 0.0,
        }
    }
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
        t!(&crate::i18n::name_key("weapon", self.name))
    }
}

/// Index 0 is the default weapon with infinite ammo.
/// New weapons go at the end: saves store ammo by index.
pub const WEAPONS: [Weapon; 14] = [
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
        bundle: 2,
    },
    Weapon {
        name: "MIRV",
        kind: Kind::Mirv {
            warheads: 5,
            spread: 45,
        },
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
    Weapon {
        name: "Baby Nuke",
        kind: Kind::Blast,
        radius: 55.0,
        damage: 75.0,
        price: 130,
        bundle: 1,
    },
    Weapon {
        name: "Roller",
        kind: Kind::Roller,
        radius: 30.0,
        damage: 50.0,
        price: 80,
        bundle: 2,
    },
    Weapon {
        name: "Leapfrog",
        kind: Kind::Leapfrog { hops: 2 },
        radius: 25.0,
        damage: 35.0,
        price: 70,
        bundle: 1,
    },
    Weapon {
        name: "Funky Bomb",
        kind: Kind::Funky { bomblets: 6 },
        radius: 22.0,
        damage: 28.0,
        price: 150,
        bundle: 1,
    },
    Weapon {
        name: "Napalm",
        kind: Kind::Napalm,
        radius: 20.0,
        damage: 20.0,
        price: 110,
        bundle: 1,
    },
    Weapon {
        name: "Digger",
        kind: Kind::Digger,
        radius: 12.0,
        damage: 0.0,
        price: 40,
        bundle: 2,
    },
    Weapon {
        name: "Tracer",
        kind: Kind::Tracer,
        radius: 0.0,
        damage: 0.0,
        price: 10,
        bundle: 5,
    },
    Weapon {
        name: "Riot Bomb",
        kind: Kind::Riot,
        radius: 60.0,
        damage: 0.0,
        price: 50,
        bundle: 2,
    },
    Weapon {
        name: "Death's Head",
        kind: Kind::Mirv {
            warheads: 9,
            spread: 30,
        },
        radius: 35.0,
        damage: 45.0,
        price: 300,
        bundle: 1,
    },
];

pub const WEAPON_COUNT: usize = WEAPONS.len();
