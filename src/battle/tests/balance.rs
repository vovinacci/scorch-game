//! Balance tools, run by hand: they print tables and assert nothing.

use super::*;

/// A balance scenario: the ground, where the shooter stands and where the targets stand.
struct Scenario {
    name: &'static str,
    ground: fn(f32) -> f32,
    shooter: f32,
    targets: &'static [f32],
}

const SCENARIOS: [Scenario; 3] = [
    Scenario {
        name: "flat ground, one target 700 px away",
        ground: |_| 600.0,
        shooter: 200.0,
        targets: &[900.0],
    },
    Scenario {
        name: "flat ground, three targets 60 px apart",
        ground: |_| 600.0,
        shooter: 200.0,
        targets: &[840.0, 900.0, 960.0],
    },
    Scenario {
        name: "one target at the bottom of a valley",
        ground: |x| 450.0 + 150.0 * (-((x - 900.0) / 140.0).powi(2)).exp(),
        shooter: 200.0,
        targets: &[900.0],
    },
];

/// (label, angle sigma in degrees, power sigma): perfect, a bot a few shots in, a bot's first tries.
/// Aims are chosen for `AIMS[CLOSE]`; kill% and self-damage are reported for `AIMS[ROUGH]`.
const CLOSE: usize = 1;

const ROUGH: usize = 2;

const AIMS: [(&str, f32, f32); 3] = [
    ("perfect", 0.0, 0.0),
    ("close", 1.0, 15.0),
    ("rough", 3.0, 45.0),
];

impl Scenario {
    /// A fresh battle: tank 0 is the shooter, the others the targets.
    fn battle(&self) -> (Vec<Player>, Battle) {
        let players = bots(1 + self.targets.len());
        let mut b = battle(&players, Setup::default().rules());
        b.terrain = Terrain::from_fn(self.ground);
        b.wind = 0.0;
        for (tank, x) in b
            .tanks
            .iter_mut()
            .zip(std::iter::once(&self.shooter).chain(self.targets))
        {
            tank.pos = vec2(*x, (self.ground)(*x));
        }
        (players, b)
    }

    /// Where every (angle, power) on the search grid lands; the same for every weapon, so worked
    /// out once.
    fn landings(&self) -> Vec<(f32, f32, Landing)> {
        let (_, b) = self.battle();
        let base = b.tanks[0].barrel_base();
        let mut grid = Vec::new();
        for angle in (40..=160).map(|a| a as f32 / 2.0) {
            for power in (300..=1000).step_by(5).map(|p| p as f32) {
                let muzzle = base + direction(angle) * BARREL;
                let vel = launch_velocity(angle, power);
                if let Some(landing) = simulate(muzzle, vel, 0.0, b.world(), &b.terrain, &[]) {
                    grid.push((angle, power, landing));
                }
            }
        }
        grid
    }

    /// The best aim for `weapon` at the middle target: among the aims whose first damage lands
    /// within 150 px of it, the one that does the most damage when fired with close aim (so a
    /// Leapfrog can land short and hop on, and bomblets scatter over the target, without picking a
    /// razor's-edge aim). Also returns how far that aim's first damage lands from the target.
    fn best_aim(&self, weapon: usize, landings: &[(f32, f32, Landing)]) -> (f32, f32, f32) {
        const NEAR: f32 = 150.0;
        const CANDIDATES: usize = 30;
        const TRIES: usize = 20;
        let (_, b) = self.battle();
        let target = b.tanks[1 + self.targets.len() / 2].center();
        let mut near: Vec<(f32, f32, f32)> = landings
            .iter()
            .map(|&(angle, power, landing)| {
                let point = damage_point(WEAPONS[weapon].kind, landing, &b.terrain);
                (point.distance(target), angle, power)
            })
            .collect();
        near.sort_by(|a, b| a.0.total_cmp(&b.0));
        // The closest aim always competes; then a spread of the rest within NEAR.
        let within = near.iter().take_while(|c| c.0 < NEAR).count().max(1);
        let step = (within / CANDIDATES).max(1);
        let mut best = (f32::MIN, near[0]);
        for &c in near[..within].iter().step_by(step) {
            let (_, sa, sp) = AIMS[CLOSE];
            let (damage, _, _) = self.measure(weapon, (c.1, c.2), (sa, sp), TRIES);
            if damage > best.0 {
                best = (damage, c);
            }
        }
        let (miss, angle, power) = best.1;
        (angle, power, miss)
    }

    /// Mean damage to all targets per shot, the share of shots that kill at least one target, and
    /// the mean damage the shooter does to itself.
    fn measure(
        &self,
        weapon: usize,
        aim: (f32, f32),
        sigma: (f32, f32),
        shots: usize,
    ) -> (f32, f32, f32) {
        let gauss = |s: f32| {
            if s == 0.0 {
                return 0.0;
            }
            let (u, v): (f32, f32) = (gen_range(1e-6, 1.0), gen_range(0.0, 1.0));
            s * (-2.0 * u.ln()).sqrt() * (std::f32::consts::TAU * v).cos()
        };
        let (mut damage, mut kills, mut own) = (0.0, 0, 0.0);
        for _ in 0..shots {
            let (mut players, mut b) = self.battle();
            // The shooter's turn, aiming within what the game allows, like a bot.
            b.current = 0;
            let angle = (aim.0 + gauss(sigma.0)).clamp(0.0, 180.0);
            let power = (aim.1 + gauss(sigma.1)).clamp(0.0, b.max_power(&b.tanks[0]));
            b.tanks[0].angle = angle;
            let vel = launch_velocity(angle, power);
            b.projectiles
                .push(Projectile::new(b.tanks[0].muzzle(), vel, weapon, 0, false));
            // The whole turn as the game plays it: flight, then falls and death blasts as things
            // settle.
            b.phase = Phase::Flight;
            for _ in 0..20_000 {
                if !matches!(b.phase, Phase::Flight | Phase::Settle) {
                    break;
                }
                b.update(STEP, &mut players);
            }
            // A tank can only lose its 100 health: blasts on a tank already at 0 don't count.
            let lost = |t: &Tank| (100.0 - t.health).clamp(0.0, 100.0);
            let targets = &b.tanks[1..];
            damage += targets.iter().map(lost).sum::<f32>();
            kills += usize::from(targets.iter().any(|t| !t.alive || t.health <= 0.0));
            own += lost(&b.tanks[0]);
        }
        let n = shots as f32;
        (damage / n, kills as f32 / n, own / n)
    }
}

/// Balance table: every weapon fired 300 times per aim through the real physics, in each scenario,
/// each shot played out like a turn (falls and death blasts included, a tank losing at most its 100
/// health). Prints mean damage to the targets per shot (perfect, close and rough aim), then kill%
/// and self-damage with rough aim, damage per dollar with close aim, and how far the best aim's first
/// damage lands from the target (miss). Run by hand:
/// `cargo test --release balance_table -- --ignored --nocapture`.
#[test]
#[ignore = "prints a table; run by hand when tuning weapons"]
fn balance_table() {
    const SHOTS: usize = 300;
    for scenario in &SCENARIOS {
        println!("\n{}", scenario.name);
        let landings = scenario.landings();
        println!(
            "{:<13} {:>6} | {:>7} {:>7} {:>7} | {:>5} {:>5} | {:>6} {:>5}",
            "weapon", "$/shot", "perfect", "close", "rough", "kill%r", "selfr", "dmg/$", "miss"
        );
        for (w, weapon) in WEAPONS.iter().enumerate() {
            if weapon.kind.is_utility() {
                continue;
            }
            let (angle, power, miss) = scenario.best_aim(w, &landings);
            let mut means = [0.0; 3];
            let (mut kills, mut own) = (0.0, 0.0);
            for (k, &(_, sa, sp)) in AIMS.iter().enumerate() {
                let (damage, kill, self_damage) =
                    scenario.measure(w, (angle, power), (sa, sp), SHOTS);
                means[k] = damage;
                if k == ROUGH {
                    (kills, own) = (kill, self_damage);
                }
            }
            let per_shot = weapon.shot_price() as f32;
            let per_dollar = if per_shot > 0.0 {
                means[CLOSE] / per_shot
            } else {
                f32::INFINITY
            };
            println!(
                "{:<13} {:>6.1} | {:>7.1} {:>7.1} {:>7.1} | {:>5.0} {:>5.1} | {:>6.2} {:>5.0}",
                weapon.name,
                per_shot,
                means[0],
                means[1],
                means[2],
                100.0 * kills,
                own,
                per_dollar,
                miss
            );
        }
    }
}
