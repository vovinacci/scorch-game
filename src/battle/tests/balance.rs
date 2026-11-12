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

/// Whole games of a stand-in for the player (a bot of skill `standin` whose aim the other bots can't see, as they
/// can't see a human's) against `lineup`: 5 rounds with the default settings, everyone shopping between rounds as
/// bots do. Returns the share of games it wins outright, the share it shares first place in, and its mean round
/// wins per game.
fn ladder_games(standin: usize, lineup: &[usize], games: u64, walls: Walls) -> (f32, f32, f32) {
    let seed = 1_000 * lineup.iter().map(|&p| p as u64 + 1).sum::<u64>();
    drawn_games(standin, seed, games, walls, || {
        lineup.iter().map(|&p| PRESETS[p]).collect()
    })
}

/// `ladder_games` against a line-up that `draw` makes afresh for every game.
fn drawn_games(
    standin: usize,
    seed: u64,
    games: u64,
    walls: Walls,
    draw: impl Fn() -> Vec<AiProfile>,
) -> (f32, f32, f32) {
    const ROUNDS: u32 = 5;
    let mut setup = Setup::default();
    setup.world.walls = walls;
    let (mut won, mut shared, mut rounds_won) = (0u32, 0u32, 0u32);
    for game in 0..games {
        // The same numbers for a game whatever else has run, so a table can be compared with the one before.
        crate::rng::seed(seed + game);
        let mut players = vec![Player::new("You", WHITE, Some(PRESETS[standin]))];
        for bot in draw() {
            players.push(Player::new("Bot", WHITE, Some(bot)));
        }
        for round in 1..=ROUNDS {
            for p in &mut players {
                p.income = economy::Income::default();
            }
            let order = crate::setup::turn_order(setup.turn_order, &players, round);
            let mut b = Battle::new(&players, order, round, ROUNDS, setup.rules());
            b.raise_shields(&mut players);
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
            let winner = match b.phase {
                Phase::Over { winner, .. } => winner,
                _ => None,
            };
            economy::end_round(&mut players, winner);
            if round < ROUNDS {
                players.iter_mut().for_each(crate::shop::ai_shop);
            }
        }
        let best_bot = players[1..].iter().map(|p| p.wins).max().unwrap_or(0);
        won += u32::from(players[0].wins > best_bot);
        shared += u32::from(players[0].wins == best_bot);
        rounds_won += players[0].wins;
    }
    let share = |count: u32| 100.0 * count as f32 / games as f32;
    (share(won), share(shared), rounds_won as f32 / games as f32)
}

/// Line-ups of one to three bots a ladder rung could be made of, strongest bot first (indices into `PRESETS`).
fn lineups() -> Vec<Vec<usize>> {
    const M: usize = 0;
    const SH: usize = 1;
    const SN: usize = 2;
    const SP: usize = 3;
    const CY: usize = 4;
    const TO: usize = 5;
    const PO: usize = 6;
    const CH: usize = 7;
    let all: [&[usize]; 38] = [
        &[M],
        &[M, M],
        &[M, M, M],
        &[TO],
        &[TO, M],
        &[TO, TO, M],
        &[SH],
        &[SH, M, M],
        &[SH, TO, M],
        &[SH, SH, M],
        &[SH, SH, SH],
        &[CH],
        &[CH, M, M],
        &[CH, SH, M],
        &[CH, CH, CH],
        &[PO],
        &[PO, M, M],
        &[PO, SH, M],
        &[PO, PO, SH],
        &[SN],
        &[SN, M],
        &[SN, M, M],
        &[SP, M, M],
        &[SP, SH, M],
        &[SP, SH, SH],
        &[SP, PO, SH],
        &[SN, SH, SH],
        &[SN, SP, SH],
        &[SN, SN, SN],
        &[SN, SP, PO],
        &[CY],
        &[CY, M],
        &[CY, M, M],
        &[CY, SH, SH],
        &[CY, SN, SP],
        &[CY, SN, SN],
        &[CY, CY, SN],
        &[CY, CY, CY],
    ];
    all.iter().map(|lineup| lineup.to_vec()).collect()
}

/// Prints `ladder_games` for every line-up, easiest first.
fn ladder_table(standin: usize, games: u64) {
    games_table(standin, &lineups(), games, Setup::default().world.walls);
}

/// Prints `ladder_games` for each of `lineups`, easiest first.
fn games_table(standin: usize, lineups: &[Vec<usize>], games: u64, walls: Walls) {
    let mut rows: Vec<((f32, f32, f32), String)> = lineups
        .iter()
        .map(|lineup| {
            let names: Vec<&str> = lineup.iter().map(|&p| PRESETS[p].name).collect();
            (
                ladder_games(standin, lineup, games, walls),
                names.join(", "),
            )
        })
        .collect();
    rows.sort_by(|a, b| b.0.0.total_cmp(&a.0.0));
    println!(
        "\na {}-like player against each line-up, {games} games of 5 rounds, walls {walls:?}",
        PRESETS[standin].name
    );
    println!("{:>6} {:>7} {:>7}  line-up", "won", "shared", "rounds");
    for ((won, shared, rounds), names) in rows {
        println!("{won:>5.1}% {shared:>6.1}% {rounds:>7.2}  {names}");
    }
}

/// The ladder's rungs: whole games of a Shooter-like stand-in for the player against the line-ups a rung could be
/// made of (see `ladder_games`). Prints the share of games it wins outright, hardest line-up last. Takes about a
/// quarter of an hour. Run by hand: `make balance`.
#[test]
#[ignore = "prints a table; run by hand when tuning the ladder, the bots or the economy"]
fn balance_ladder() {
    ladder_table(1, 200);
}

/// The same against a Sniper-like stand-in: a ladder has to work for a strong player too. About a quarter of an hour more.
#[test]
#[ignore = "prints a table; run by hand when tuning the ladder, the bots or the economy"]
fn balance_ladder_sniper() {
    ladder_table(2, 200);
}

/// What a bot type is worth in a line-up: the same games as `balance_ladder` against each type alone and with two
/// Morons. Types next to each other in the table cost the player about the same. About ten minutes.
#[test]
#[ignore = "prints a table; run by hand when tuning the bot types or their threat points"]
fn balance_types() {
    let mut lineups = Vec::new();
    for top in 0..PRESETS.len() {
        lineups.extend([vec![top], vec![top, 0, 0]]);
    }
    for standin in [1, 2] {
        games_table(standin, &lineups, 200, Setup::default().world.walls);
    }
}

/// What each difficulty is worth: the same games against line-ups drawn as the game draws them (`ai::lineup`),
/// for one to three bots. About ten minutes.
#[test]
#[ignore = "prints a table; run by hand when tuning the difficulty budgets or the threat points"]
fn balance_difficulty() {
    for standin in [1, 2] {
        println!(
            "\na {}-like player against each difficulty, 200 games of 5 rounds",
            PRESETS[standin].name
        );
        println!(
            "{:>6} {:>7} {:>7}  difficulty, bots",
            "won", "shared", "rounds"
        );
        for difficulty in 0..crate::ai::CUSTOM {
            for bots in 1..=3 {
                let seed = 10_000 * (difficulty as u64 + 1) + 1_000 * bots as u64;
                let (won, shared, rounds) =
                    drawn_games(standin, seed, 200, Setup::default().world.walls, || {
                        crate::ai::lineup(difficulty, bots, PRESETS[1])
                    });
                println!(
                    "{won:>5.1}% {shared:>6.1}% {rounds:>7.2}  {}, {bots}",
                    crate::ai::DIFFICULTIES[difficulty]
                );
            }
        }
    }
}

/// What the types that aim differently are worth: a Tosser beside a Shooter (the same aim, lobs only) and a
/// Poolshark beside a Sniper, alone and with two Morons, behind each kind of wall a Poolshark cares about. The
/// same games as `balance_ladder`. About a quarter of an hour.
#[test]
#[ignore = "prints a table; run by hand when tuning the bot types or their threat points"]
fn balance_shots() {
    const SHOOTER: usize = 1;
    const SNIPER: usize = 2;
    const TOSSER: usize = 5;
    const POOLSHARK: usize = 6;
    let mut lineups = Vec::new();
    for top in [SHOOTER, TOSSER, SNIPER, POOLSHARK] {
        lineups.extend([vec![top], vec![top, 0, 0]]);
    }
    for walls in [Walls::Open, Walls::Bouncy, Walls::Wrap] {
        for standin in [SHOOTER, SNIPER] {
            games_table(standin, &lineups, 150, walls);
        }
    }
}
