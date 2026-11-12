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

/// Firing order and round length: equal bots with the starting weapons play `ROUNDS` rounds for each line. Prints
/// the share of rounds won by the tank firing first, second and so on, the mean number of turns, and the share of
/// rounds that reached sudden death. Run by hand: `make balance`.
#[test]
#[ignore = "prints a table; run by hand when tuning turn order, bots or sudden death"]
fn balance_order() {
    const ROUNDS: u32 = 600;
    println!(
        "\nequal bots, {ROUNDS} rounds each: wins by firing order | mean turns | sudden death"
    );
    for (n, preset) in [(2, 0), (2, 1), (2, 2), (3, 0), (3, 1), (3, 2), (4, 1)] {
        let mut wins = vec![0u32; n];
        let (mut turns, mut sudden) = (0, 0);
        for _ in 0..ROUNDS {
            let mut players: Vec<Player> = (0..n)
                .map(|i| Player::new(&format!("B{i}"), WHITE, Some(PRESETS[preset])))
                .collect();
            let mut b = battle(&players, Setup::default().rules());
            b.skip_to_end(&mut players);
            if let Phase::Over {
                winner: Some(w), ..
            } = b.phase
            {
                wins[w] += 1;
            }
            turns += b.turns;
            sudden += u32::from(b.rules.sudden_death.is_some_and(|start| b.turns >= start));
        }
        let share = |count: u32| 100.0 * count as f32 / ROUNDS as f32;
        let by_order: Vec<String> = wins
            .iter()
            .map(|&w| format!("{:>3.0}%", share(w)))
            .collect();
        println!(
            "{n} x {:<8} {:<24} | {:>5.1} | {:>3.0}%",
            PRESETS[preset].name,
            by_order.join(" "),
            turns as f32 / ROUNDS as f32,
            share(sudden)
        );
    }
}

/// Digging out: a tank under a direct Dirt Bomb hit fires each weapon straight up until its barrel is clear (at
/// most 4 shots). Prints the shots it took and the health left. Run by hand: `make balance`.
#[test]
#[ignore = "prints a table; run by hand when tuning the Dirt Bomb or digging out"]
fn balance_buried() {
    println!("\nburied by a direct Dirt Bomb hit, firing straight up");
    println!(
        "{:<13} {:>6} | {:>5} {:>6}",
        "weapon", "$/shot", "shots", "health"
    );
    for (w, weapon) in WEAPONS.iter().enumerate() {
        if matches!(weapon.kind, Kind::Dirt | Kind::Tracer) {
            continue;
        }
        let (mut players, mut b) = range(|_| 600.0);
        let at = b.tanks[0].pos;
        b.terrain.add_dirt(at, 45.0);
        let muzzle = b.tanks[0].barrel_base() + direction(90.0) * BARREL;
        // Buried while a shell can't get out: it moves about 3 px a step, so less dirt than that doesn't hold it.
        let clear = muzzle - vec2(0.0, 4.0);
        let mut shots = 0;
        while shots < 4 && b.tanks[0].alive && b.terrain.is_solid(clear) {
            let vel = launch_velocity(90.0, 500.0);
            b.projectiles
                .push(Projectile::new(muzzle, vel, w, 0, false));
            run(&mut b, &mut players);
            shots += 1;
        }
        let out = if b.terrain.is_solid(clear) {
            "still buried"
        } else {
            ""
        };
        println!(
            "{:<13} {:>6.1} | {:>5} {:>6.0} {out}",
            weapon.name,
            weapon.shot_price() as f32,
            shots,
            b.tanks[0].health.max(0.0)
        );
    }
}

/// The Mag Deflector against one Missile on different paths: dropped from 300 px above the tank at an offset, or
/// lobbed in from the side. Prints the energy left of 40 and the tank's health. Run by hand: `make balance`.
#[test]
#[ignore = "prints a table; run by hand when tuning the Mag Deflector"]
fn balance_deflector() {
    println!("\none Missile against a full Mag Deflector");
    println!("{:<34} | {:>6} {:>6}", "path", "energy", "health");
    let paths = [
        ("dropped dead centre", 0.0, 0.0),
        ("dropped 5 px off centre", 5.0, 0.0),
        ("dropped 20 px off centre", 20.0, 0.0),
        ("lobbed from 150 px away", -150.0, 120.0),
        ("lobbed from 300 px away", -300.0, 200.0),
    ];
    for (name, dx, vx) in paths {
        let mut players = bots(2);
        players[1].items[3] = 1;
        let mut rules = Setup::default().rules();
        rules.world.terrain = crate::world::TerrainKind::Flat;
        let mut b = battle(&players, rules);
        b.raise_shields(&mut players);
        b.wind = 0.0;
        let centre = b.tanks[1].center();
        let start = centre + vec2(dx, -300.0);
        b.projectiles.push(Projectile::new(
            start,
            vec2(vx, 0.0),
            weapon("Missile"),
            0,
            false,
        ));
        run(&mut b, &mut players);
        println!(
            "{name:<34} | {:>6.0} {:>6.0}",
            b.tanks[1].shield.map_or(0.0, |s| s.energy),
            b.tanks[1].health
        );
    }
}

/// The ladder's rungs: a stand-in for the player (a Shooter whose aim the bots can't see, as they can't see a
/// human's) plays 400 rounds with the starting weapons against every line-up of one to three bots, firing first,
/// second and so on in turn. Prints the share of rounds it wins, hardest last. Run by hand: `make balance`.
#[test]
#[ignore = "prints a table; run by hand when tuning the ladder or the bots"]
fn balance_ladder() {
    const ROUNDS: usize = 400;
    let mut lineups: Vec<Vec<usize>> = Vec::new();
    for a in 0..3 {
        lineups.push(vec![a]);
        for b in 0..=a {
            lineups.push(vec![a, b]);
            for c in 0..=b {
                lineups.push(vec![a, b, c]);
            }
        }
    }
    let mut rows: Vec<(f32, String)> = lineups
        .iter()
        .map(|lineup| {
            let n = lineup.len() + 1;
            let mut wins = 0;
            for round in 0..ROUNDS {
                let mut players = vec![Player::new("You", WHITE, Some(PRESETS[1]))];
                for &preset in lineup {
                    players.push(Player::new("Bot", WHITE, Some(PRESETS[preset])));
                }
                let order: Vec<usize> = (0..n).map(|k| (k + round) % n).collect();
                let mut b = Battle::new(&players, order, 1, 5, Setup::default().rules());
                for _ in 0..200_000 {
                    if matches!(b.phase, Phase::Over { .. }) {
                        break;
                    }
                    // The bots plan without seeing whom the stand-in aims at.
                    let hidden = if b.current == 0 {
                        None
                    } else {
                        b.tanks[0].aim.take()
                    };
                    b.update(1.0 / 30.0, &mut players);
                    if hidden.is_some() && b.tanks[0].aim.is_none() {
                        b.tanks[0].aim = hidden;
                    }
                }
                if let Phase::Over {
                    winner: Some(0), ..
                } = b.phase
                {
                    wins += 1;
                }
            }
            let names: Vec<&str> = lineup.iter().map(|&p| PRESETS[p].name).collect();
            (100.0 * wins as f32 / ROUNDS as f32, names.join(", "))
        })
        .collect();
    rows.sort_by(|a, b| b.0.total_cmp(&a.0));
    println!("\na Shooter-like player against each line-up, {ROUNDS} rounds: rounds won");
    for (share, names) in rows {
        println!("{share:>5.1}%  {names}");
    }
}
