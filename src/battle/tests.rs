//! Tests for the battle: weapons, shields, turns, bots' aim and the HUD layout.

use super::*;
use crate::ai::PRESETS;
use crate::items::PARACHUTE;
use crate::setup::Setup;
use crate::world::{GravityLevel, Sky, Walls, WindLevel};

fn bots(n: usize) -> Vec<Player> {
    (0..n)
        .map(|i| Player::new(&format!("B{i}"), WHITE, Some(PRESETS[1])))
        .collect()
}

fn battle(players: &[Player], rules: Rules) -> Battle {
    Battle::new(players, (0..players.len()).collect(), 1, 5, rules)
}

#[test]
fn health_limits_power_only_when_enabled() {
    let players = bots(2);
    let mut rules = Setup::default().rules();
    rules.power_limit = true;
    let mut b = battle(&players, rules);
    b.tanks[0].health = 30.0;
    assert_eq!(b.max_power(&b.tanks[0]), 300.0);
    b.tanks[0].health = 1.0;
    assert_eq!(b.max_power(&b.tanks[0]), MIN_POWER_LIMIT);
    b.rules.power_limit = false;
    assert_eq!(b.max_power(&b.tanks[0]), MAX_POWER);
}

#[test]
fn self_destruct_kills_self_without_a_kill() {
    let mut players = bots(2);
    let mut b = battle(&players, Setup::default().rules());
    b.self_destruct(&mut players);
    b.skip_to_end(&mut players);
    assert!(!b.tanks[0].alive);
    assert_eq!(players[0].kills, 0, "dying to yourself is not a kill");
    assert!(players[0].round_stats.died);
}

#[test]
fn dying_by_own_shot_credits_the_last_enemy_who_hit_you() {
    let mut players = bots(2);
    let mut b = battle(&players, Setup::default().rules());
    let at = b.tanks[0].center();
    // Enemy (1) hits tank 0, then tank 0 finishes itself off.
    b.blast(at, 20.0, 30.0, Some(1), &mut players);
    b.tanks[0].health = 5.0;
    b.blast(at, 20.0, 30.0, Some(0), &mut players);
    b.phase = Phase::Settle;
    b.update(0.1, &mut players);
    assert!(!b.tanks[0].alive);
    assert_eq!(players[1].kills, 1, "enemy gets the kill");
    assert_eq!(players[0].kills, 0);
}

#[test]
fn full_log_has_every_turn() {
    let mut players = bots(3);
    let mut b = battle(&players, Setup::default().rules());
    b.skip_to_end(&mut players);
    for turn in 1..=b.turns {
        assert!(
            b.history.iter().any(|l| l.turn == turn),
            "turn {turn} missing from the log"
        );
    }
    let shots: u32 = players.iter().map(|p| p.round_stats.shots).sum();
    let fires = b
        .history
        .iter()
        .filter(|l| l.text.contains(" fires "))
        .count();
    assert_eq!(fires as u32, shots, "every shot is logged");
}

#[test]
fn full_log_records_targets_and_hits_by_turn() {
    let mut players = bots(2);
    let mut b = battle(&players, Setup::default().rules());
    b.tanks[1].revenge = Some(0);
    let profile = players[1].bot.unwrap();
    b.choose_target(1, &players, &profile);
    let at = b.tanks[1].center();
    b.blast(at, 20.0, 10.0, Some(0), &mut players);
    let lines: Vec<_> = b
        .history
        .iter()
        .map(|l| (l.turn, l.text.as_str()))
        .collect();
    assert_eq!(lines[0], (1, "B1 targets B0 (revenge)"));
    assert!(lines[1].1.starts_with("B0 hit B1"), "{lines:?}");
    assert_eq!(b.log.len(), 1, "targeting stays out of the feed");
}

#[test]
fn carried_ammo_grows_the_death_blast() {
    let mut p = Player::new("P", WHITE, None);
    p.ammo = [0; crate::weapons::WEAPON_COUNT];
    assert_eq!(death_blast(&p), (DEATH_RADIUS, DEATH_DAMAGE));
    p.ammo[3] = 1; // one Nuke
    let (r, d) = death_blast(&p);
    assert!(r > DEATH_RADIUS && d > DEATH_DAMAGE);
    p.ammo[3] = 100;
    assert_eq!(death_blast(&p), (DEATH_MAX_RADIUS, DEATH_MAX_DAMAGE));
}

#[test]
fn sudden_death_scales_with_the_number_of_tanks() {
    let mut rules = Setup::default().rules();
    rules.sudden_death = Some(12);
    assert_eq!(battle(&bots(2), rules).rules.sudden_death, Some(24));
    assert_eq!(battle(&bots(4), rules).rules.sudden_death, Some(48));
}

#[test]
fn sudden_death_drains_everyone() {
    let mut players = bots(2);
    let mut rules = Setup::default().rules();
    rules.sudden_death = Some(0);
    let mut b = battle(&players, rules);
    b.phase = Phase::Settle;
    b.update(0.1, &mut players);
    for t in &b.tanks {
        assert_eq!(t.health, 100.0 - SUDDEN_DEATH_DRAIN);
    }
}

#[test]
fn winning_human_never_counts_as_bots_only() {
    let mut players = bots(2);
    players[0] = Player::new("You", WHITE, None);
    let mut b = battle(&players, Setup::default().rules());
    let at = b.tanks[1].center();
    b.blast(at, 20.0, 200.0, Some(0), &mut players);
    b.phase = Phase::Settle;
    for _ in 0..2000 {
        assert!(!b.only_bots_left(&players));
        if b.update(1.0 / 60.0, &mut players).is_some() {
            break;
        }
    }
    assert!(b.tanks[0].alive);
}

#[test]
fn a_shield_takes_hits_before_health() {
    let mut players = bots(2);
    players[1].items[1] = 1; // a Shield (50)
    let mut b = battle(&players, Setup::default().rules());
    b.raise_shields(&mut players);
    assert_eq!(players[1].items[1], 0, "raising uses it up");
    let at = b.tanks[1].center();
    b.blast(at, 20.0, 30.0, Some(0), &mut players);
    assert_eq!(b.tanks[1].health, 100.0);
    assert_eq!(b.tanks[1].shield.map(|s| s.energy), Some(20.0));
    b.blast(at, 20.0, 40.0, Some(0), &mut players);
    assert!(b.tanks[1].shield.is_none(), "shield is down");
    assert_eq!(b.tanks[1].health, 80.0, "the rest goes to health");
}

#[test]
fn hitting_a_shield_pays_like_hitting_health() {
    let mut players = bots(2);
    players[1].items[2] = 1; // Heavy Shield (100)
    let mut b = battle(&players, Setup::default().rules());
    b.raise_shields(&mut players);
    let at = b.tanks[1].center();
    b.blast(at, 20.0, 30.0, Some(0), &mut players);
    assert_eq!(b.tanks[1].health, 100.0, "all absorbed");
    assert_eq!(players[0].income.damage, 30, "but the shooter is paid");
}

#[test]
fn bots_see_deflectors_in_their_aim() {
    let world = Setup::default().rules().world;
    let terrain = Terrain::generate(crate::world::TerrainKind::Flat);
    let (pos, vel) = (vec2(200.0, 300.0), vec2(300.0, -200.0));
    let plain = simulate(pos, vel, 0.0, world, &terrain, &[]);
    let bent = simulate(pos, vel, 0.0, world, &terrain, &[vec2(330.0, 230.0)]);
    assert_ne!(plain, bent, "the deflector changes where the shell lands");
}

#[test]
fn switched_off_shields_stay_in_the_inventory() {
    let mut players = bots(2);
    players[0].items[2] = 1;
    players[0].items_on[2] = false;
    let mut b = battle(&players, Setup::default().rules());
    b.raise_shields(&mut players);
    assert!(b.tanks[0].shield.is_none());
    assert_eq!(players[0].items[2], 1);
}

#[test]
fn a_parachute_cancels_fall_damage() {
    let mut players = bots(2);
    players[0].items[PARACHUTE] = 1;
    let mut b = battle(&players, Setup::default().rules());
    for i in 0..2 {
        b.tanks[i].pos.y -= 150.0; // both in the air, well above the ground
    }
    // Only the landing: stop as soon as the turn moves on, before any bot fires.
    b.phase = Phase::Settle;
    for _ in 0..600 {
        if !matches!(b.phase, Phase::Settle) {
            break;
        }
        b.update(1.0 / 60.0, &mut players);
    }
    assert_eq!(b.tanks[0].health, 100.0, "the parachute opened");
    assert_eq!(players[0].items[PARACHUTE], 0, "and was used up");
    assert!(b.tanks[1].health < 100.0, "no parachute, the fall hurt");
}

#[test]
fn a_mag_deflector_pushes_enemy_shells_away() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut rules = Setup::default().rules();
    rules.world.terrain = crate::world::TerrainKind::Flat;
    let mut b = battle(&players, rules);
    b.raise_shields(&mut players);
    let target = b.tanks[1].center();
    // An enemy shell inside the field and above the (flat) ground, flying at the tank.
    let start = target + vec2(-60.0, -30.0);
    b.projectiles
        .push(Projectile::new(start, vec2(200.0, 0.0), 0, 0, false));
    b.step_projectiles(&mut players);
    let p = &b.projectiles[0];
    assert!(p.vel.x < 200.0, "slowed down on its way in: {}", p.vel.x);
    let full = ITEMS[3].shield_energy().unwrap();
    assert_eq!(
        b.tanks[1].shield.map(|s| s.energy),
        Some(full - DEFLECT_COST),
        "pays once"
    );
    b.step_projectiles(&mut players);
    assert_eq!(
        b.tanks[1].shield.map(|s| s.energy),
        Some(full - DEFLECT_COST),
        "not again for the same shell"
    );
}

#[test]
fn a_mag_deflector_runs_out_after_enough_shells() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut rules = Setup::default().rules();
    rules.world.terrain = crate::world::TerrainKind::Flat;
    let mut b = battle(&players, rules);
    b.raise_shields(&mut players);
    let target = b.tanks[1].center();
    let shells = (ITEMS[3].shield_energy().unwrap() / DEFLECT_COST) as usize;
    for _ in 0..shells {
        b.projectiles.push(Projectile::new(
            target + vec2(-60.0, -30.0),
            vec2(200.0, 0.0),
            0,
            0,
            false,
        ));
    }
    b.step_projectiles(&mut players);
    assert!(
        b.deflectors(0).is_empty(),
        "spent: new shells aren't pushed or charged"
    );
    // Once the shells are done, it goes down.
    b.drop_spent_deflectors(&players);
    assert!(b.tanks[1].shield.is_none(), "the deflector is down");
}

#[test]
fn a_mag_deflectors_last_energy_still_pushes_the_shell_that_paid_for_it() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut rules = Setup::default().rules();
    rules.world.terrain = crate::world::TerrainKind::Flat;
    let mut b = battle(&players, rules);
    b.raise_shields(&mut players);
    b.wind = 0.0;
    b.tanks[1].shield.as_mut().unwrap().energy = DEFLECT_COST;
    let target = b.tanks[1].center();
    b.projectiles.push(Projectile::new(
        target + vec2(-60.0, -30.0),
        vec2(200.0, 0.0),
        0,
        0,
        false,
    ));
    b.step_projectiles(&mut players);
    let p = &b.projectiles[0];
    assert!(
        p.vel.x < 200.0,
        "pushed with the last 10 energy: {}",
        p.vel.x
    );
    assert!(
        b.tanks[1].shield.is_some(),
        "still up while its shell flies"
    );

    // A second shell arriving now is neither charged nor pushed.
    let mut late = Projectile::new(target + vec2(-60.0, -30.0), vec2(200.0, 0.0), 0, 0, false);
    b.charge_deflectors(&mut late);
    assert!(
        b.deflectors_for(&late).is_empty(),
        "nothing left to push it"
    );
}

#[test]
fn bots_only_round_always_ends() {
    let mut players = bots(3);
    let mut b = battle(&players, Setup::default().rules());
    assert!(b.only_bots_left(&players));
    b.skip_to_end(&mut players);
    assert!(matches!(b.phase, Phase::Over { .. }));
}

fn weapon(name: &str) -> usize {
    WEAPONS.iter().position(|w| w.name == name).unwrap()
}

/// Two bots, tanks parked at the far edges, on ground shaped by `height`.
fn range(height: impl Fn(f32) -> f32) -> (Vec<Player>, Battle) {
    let players = bots(2);
    let mut b = battle(&players, Setup::default().rules());
    b.terrain = Terrain::from_fn(height);
    b.wind = 0.0;
    for (tank, x) in b.tanks.iter_mut().zip([30.0, 1250.0]) {
        tank.pos = vec2(x, b.terrain.height_at(x));
    }
    (players, b)
}

/// Drops `weapon` straight down onto the ground at `x`, fired by tank 0.
fn drop(b: &mut Battle, weapon: usize, x: f32, vel_x: f32) {
    let y = b.terrain.height_at(x) - 1.0;
    b.projectiles.push(Projectile::new(
        vec2(x, y),
        vec2(vel_x, 200.0),
        weapon,
        0,
        false,
    ));
}

/// Runs shells and fires until they are all done; returns every explosion.
fn run(b: &mut Battle, players: &mut [Player]) -> Vec<Vec2> {
    for _ in 0..20_000 {
        if b.projectiles.is_empty() && b.fires.is_empty() {
            break;
        }
        b.step_projectiles(players);
        b.step_fires(players);
    }
    assert!(
        b.projectiles.is_empty() && b.fires.is_empty(),
        "everything settles"
    );
    b.explosions
        .iter()
        .filter(|e| !e.dirt)
        .map(|e| e.pos)
        .collect()
}

/// A valley with its lowest point at x = 640.
fn valley(x: f32) -> f32 {
    600.0 - (x - 640.0).abs() * 0.2
}

#[test]
fn a_roller_rolls_down_into_the_dip() {
    let (mut players, mut b) = range(valley);
    drop(&mut b, weapon("Roller"), 500.0, 0.0);
    let blasts = run(&mut b, &mut players);
    assert_eq!(blasts.len(), 1);
    assert!(
        (blasts[0].x - 640.0).abs() < 6.0,
        "explodes in the dip: {}",
        blasts[0].x
    );
}

#[test]
fn a_roller_stops_at_a_tank() {
    let (mut players, mut b) = range(valley);
    b.tanks[1].pos = vec2(560.0, valley(560.0));
    drop(&mut b, weapon("Roller"), 500.0, 0.0);
    let blasts = run(&mut b, &mut players);
    assert!(
        blasts[0].x < 560.0,
        "explodes against the tank: {}",
        blasts[0].x
    );
    assert!(b.tanks[1].health < 100.0);
}

#[test]
fn a_leapfrog_explodes_three_times_moving_on() {
    let (mut players, mut b) = range(|_| 600.0);
    drop(&mut b, weapon("Leapfrog"), 300.0, 150.0);
    let blasts = run(&mut b, &mut players);
    assert_eq!(blasts.len(), 3);
    assert!(
        blasts[0].x < blasts[1].x && blasts[1].x < blasts[2].x,
        "hops on: {blasts:?}"
    );
}

#[test]
fn a_funky_bomb_throws_bomblets() {
    let (mut players, mut b) = range(|_| 600.0);
    drop(&mut b, weapon("Funky Bomb"), 640.0, 0.0);
    b.step_projectiles(&mut players);
    assert_eq!(b.projectiles.len(), 6);
    assert!(b.projectiles.iter().all(|p| p.child));
    let blasts = run(&mut b, &mut players);
    assert!(blasts.len() > 1, "the bomblets explode too");
}

#[test]
fn napalm_runs_downhill_and_burns_the_tank_there() {
    let (mut players, mut b) = range(valley);
    b.tanks[1].pos = vec2(640.0, valley(640.0));
    drop(&mut b, weapon("Napalm"), 600.0, 0.0);
    let blasts = run(&mut b, &mut players);
    assert!(blasts.is_empty(), "no explosion, only fire");
    let lost = 100.0 - b.tanks[1].health;
    assert!(lost > 10.0, "burned: {lost}");
    assert!(
        lost <= WEAPONS[weapon("Napalm")].damage * FIRE_TIME + 0.1,
        "at most the full burn: {lost}"
    );
    assert!(players[0].income.damage > 0, "the shooter is paid");
}

#[test]
fn a_digger_sinks_the_ground_under_a_tank() {
    let (mut players, mut b) = range(|_| 600.0);
    b.tanks[1].pos = vec2(700.0, 600.0);
    drop(&mut b, weapon("Digger"), 640.0, 100.0);
    let blasts = run(&mut b, &mut players);
    assert_eq!(blasts, []);
    assert_eq!(b.terrain.height_at(700.0), 600.0 + DIG_DEPTH);
    assert_eq!(
        b.terrain.height_at(640.0 + DIG_LENGTH + 20.0),
        600.0,
        "only as far as it digs"
    );
    assert_eq!(b.tanks[1].health, 100.0, "no damage by itself");
    assert_eq!(
        b.tanks[1].last_hit_by,
        Some(0),
        "a deadly fall would be the digger's kill"
    );
}

#[test]
fn bots_never_fire_utility_weapons_at_enemies() {
    let mut players = bots(2);
    players[0].ammo[weapon("Digger")] = 5;
    players[0].ammo[weapon("Dirt Bomb")] = 5;
    players[0].ammo[weapon("Roller")] = 1;
    let mut b = battle(&players, Setup::default().rules());
    for _ in 0..10 {
        let shot = b.plan_shot(0, &players);
        assert!(!WEAPONS[shot.weapon].kind.is_utility());
    }
}

#[test]
fn bots_aim_rollers_and_napalm_at_where_they_end_up() {
    let terrain = Terrain::from_fn(valley);
    let landing = Landing {
        pos: vec2(500.0, valley(500.0)),
        vel: vec2(0.0, 200.0),
        ground: true,
    };
    let roll = damage_point(Kind::Roller, landing, &terrain);
    assert!(
        (roll.x - 640.0).abs() < 6.0,
        "the Roller ends in the dip: {}",
        roll.x
    );
    let fire = damage_point(Kind::Napalm, landing, &terrain);
    assert!(
        fire.x > 600.0,
        "the fire runs down towards the dip: {}",
        fire.x
    );
    assert_eq!(damage_point(Kind::Blast, landing, &terrain), landing.pos);
}

#[test]
fn the_predicted_roll_matches_the_real_one() {
    let (mut players, mut b) = range(valley);
    drop(&mut b, weapon("Roller"), 500.0, 0.0);
    let blast = run(&mut b, &mut players)[0];
    let landing = Landing {
        pos: vec2(500.0, valley(500.0)),
        vel: vec2(0.0, 200.0),
        ground: true,
    };
    let predicted = damage_point(Kind::Roller, landing, &Terrain::from_fn(valley));
    assert!(
        (predicted.x - blast.x).abs() < 3.0,
        "{} vs {}",
        predicted.x,
        blast.x
    );
}

#[test]
fn the_weapon_strip_shows_owned_weapons_on_screen() {
    let mut players = bots(1);
    let p = &mut players[0];
    assert_eq!(
        weapon_strip(p).len(),
        2,
        "Baby Missile and the starting Missiles"
    );
    p.ammo = [1; crate::weapons::WEAPON_COUNT];
    let strip = weapon_strip(p);
    assert_eq!(strip.len(), WEAPONS.len());
    for (k, (_, a)) in strip.iter().enumerate() {
        assert!(a.w >= 140.0, "wide enough to read");
        assert!(a.left() >= 0.0 && a.right() <= ui::WIDTH && a.bottom() <= HEIGHT);
        assert!(
            strip[k + 1..].iter().all(|(_, b)| !a.overlaps(b)),
            "no overlap"
        );
    }
}

#[test]
fn a_tracer_hurts_no_one_and_its_path_stays() {
    let (mut players, mut b) = range(|_| 600.0);
    b.tanks[1].pos = vec2(645.0, 600.0);
    // A real shot first: its path must survive the Tracer.
    drop(&mut b, 0, 300.0, 50.0);
    run(&mut b, &mut players);
    let real = b.tanks[0].last_trail.clone();
    let ground = |b: &Battle| -> Vec<f32> {
        (0..128)
            .map(|k| b.terrain.height_at(k as f32 * 10.0))
            .collect()
    };
    let before = ground(&b);
    assert!(real.len() > 1, "the real shot left a path");
    b.explosions.clear();

    // Lobbed at the other tank, so it flies a real arc.
    b.projectiles.push(Projectile::new(
        vec2(500.0, 590.0),
        vec2(60.0, -250.0),
        weapon("Tracer"),
        0,
        false,
    ));
    let blasts = run(&mut b, &mut players);
    assert_eq!(blasts, []);
    assert_eq!(b.tanks[1].health, 100.0);
    assert_eq!(ground(&b), before, "no new crater anywhere");
    assert_eq!(b.tracers.len(), 1, "its path stays on the field");
    assert_eq!(b.tracers[0].owner, 0, "with who fired it");
    assert!(b.tracers[0].points.len() > 1, "a path to draw");
    assert_eq!(
        b.tanks[0].last_trail, real,
        "the last real shot's path is kept"
    );
}

#[test]
fn a_tracer_path_is_thinned_to_even_dashes() {
    let trail: Vec<Vec2> = (0..600)
        .map(|k| vec2(100.0 + k as f32 * 0.5, 300.0))
        .collect();
    let path = TracerPath::new(0, &trail);
    assert!(path.points.len() < trail.len() / 5, "far fewer points");
    for w in path.points.windows(2) {
        let d = (w[1][0] - w[0][0]).abs();
        assert!((TRACER_DASH..TRACER_DASH + 1.0).contains(&d), "{d}");
    }
}

#[test]
fn a_riot_bomb_clears_dirt_without_damage() {
    let (mut players, mut b) = range(|_| 600.0);
    b.tanks[1].pos = vec2(660.0, 600.0);
    drop(&mut b, weapon("Riot Bomb"), 640.0, 0.0);
    let blasts = run(&mut b, &mut players);
    assert_eq!(blasts, [], "no blast, only cleared dirt");
    assert!(
        b.terrain.height_at(640.0) > 640.0,
        "a deep hole where it landed"
    );
    assert_eq!(
        b.tanks[1].health, 100.0,
        "no blast damage (the fall comes later)"
    );
    assert_eq!(
        b.tanks[1].last_hit_by,
        Some(0),
        "a deadly fall would be the bomber's kill"
    );
}

#[test]
fn deaths_head_splits_into_nine_warheads() {
    let (mut players, mut b) = range(|_| 700.0);
    // Already falling, high above the ground: it splits on the next step.
    b.projectiles.push(Projectile::new(
        vec2(640.0, 100.0),
        vec2(0.0, 10.0),
        weapon("Death's Head"),
        0,
        false,
    ));
    b.step_projectiles(&mut players);
    assert_eq!(b.projectiles.len(), 9);
    assert!(b.projectiles.iter().all(|p| p.child));
    let blasts = run(&mut b, &mut players);
    assert_eq!(blasts.len(), 9, "every warhead explodes");
}

#[test]
fn a_tracer_is_not_a_shot_at_anyone() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut rules = Setup::default().rules();
    rules.world.terrain = crate::world::TerrainKind::Flat;
    let mut b = battle(&players, rules);
    b.raise_shields(&mut players);
    let target = b.tanks[1].center();
    b.projectiles.push(Projectile::new(
        target + vec2(-60.0, -30.0),
        vec2(200.0, 0.0),
        weapon("Tracer"),
        0,
        false,
    ));
    b.step_projectiles(&mut players);
    let full = ITEMS[3].shield_energy().unwrap();
    assert_eq!(
        b.tanks[1].shield.map(|s| s.energy),
        Some(full),
        "costs the deflector nothing"
    );

    let me = b.current;
    players[me].ammo[weapon("Tracer")] = 1;
    b.tanks[me].weapon = weapon("Tracer");
    b.fire(&mut players);
    assert_eq!(b.shot_line, None, "never logged as a miss");
    assert_eq!(players[me].round_stats.shots, 0, "not counted for accuracy");
}

#[test]
fn outer_warheads_drift_faster_on_a_deaths_head() {
    let side = |name: &str| WEAPONS[weapon(name)].kind.side_speed();
    assert_eq!(side("Missile"), 0.0);
    assert_eq!(side("MIRV"), 90.0);
    assert_eq!(side("Death's Head"), 120.0);
}

#[test]
fn a_riot_bomb_leaves_tanks_on_untouched_ground_alone() {
    // A tank down in a valley, within reach sideways of a Riot Bomb landing on a high ledge, but far below it.
    let (mut players, mut b) = range(|x| if x < 600.0 { 400.0 } else { 600.0 });
    b.tanks[1].pos = vec2(610.0, 600.0);
    drop(&mut b, weapon("Riot Bomb"), 560.0, 0.0);
    run(&mut b, &mut players);
    assert_eq!(b.tanks[1].last_hit_by, None, "its ground didn't move");
}

#[test]
fn the_weapon_strip_turns_see_through_over_a_tank_in_a_deep_valley() {
    let (players, mut b) = range(|_| 400.0);
    let strip = weapon_strip(&players[0]);
    assert!(!covers_a_tank(&strip, &b.tank_areas(&players)));
    // Down in a crater at the bottom of the screen, right behind the strip.
    b.tanks[1].pos = vec2(640.0, 690.0);
    assert!(covers_a_tank(&strip, &b.tank_areas(&players)));
}

#[test]
fn a_hole_under_only_part_of_a_tank_doesnt_count_as_dropping_it() {
    let (players, mut b) = range(|x| {
        if (639.0..=641.0).contains(&x) {
            700.0
        } else {
            600.0
        }
    });
    b.tanks[1].pos = vec2(640.0, 600.0);
    b.undermine(600.0, 680.0, 0, &players);
    assert_eq!(b.tanks[1].last_hit_by, None, "its edges still hold it up");
}

#[test]
fn dropping_a_tank_is_logged_and_asks_for_revenge() {
    let (mut players, mut b) = range(|_| 600.0);
    b.tanks[1].pos = vec2(650.0, 600.0);
    let before = b.history.len();
    drop(&mut b, weapon("Riot Bomb"), 640.0, 0.0);
    run(&mut b, &mut players);
    assert_eq!(b.tanks[1].revenge, Some(0));
    assert_eq!(
        b.history.len(),
        before + 1,
        "one line, so the shot isn't logged as a miss"
    );
}

#[test]
fn mouse_aiming_points_the_barrel_at_the_cursor() {
    let from = vec2(600.0, 500.0);
    let (angle, power) = point_aim(from, from + vec2(100.0, -100.0), 1000.0);
    assert!((angle - 45.0).abs() < 1e-3);
    assert!((power - 100.0 * 2.0_f32.sqrt() * POWER_PER_PIXEL).abs() < 1e-3);
    let (angle, _) = point_aim(from, from + vec2(0.0, -50.0), 1000.0);
    assert!((angle - 90.0).abs() < 1e-3, "straight up");
    let (angle, _) = point_aim(from, from + vec2(-80.0, -1.0), 1000.0);
    assert!(angle > 179.0, "almost flat to the left");
}

#[test]
fn mouse_aiming_below_the_barrel_stays_flat_and_power_is_capped() {
    let from = vec2(600.0, 500.0);
    assert_eq!(point_aim(from, from + vec2(50.0, 40.0), 1000.0).0, 0.0);
    assert_eq!(point_aim(from, from + vec2(-50.0, 40.0), 1000.0).0, 180.0);
    let (_, power) = point_aim(from, from + vec2(0.0, -600.0), 300.0);
    assert_eq!(power, 300.0, "never above the tank's maximum");
}

#[test]
fn random_world_settings_get_a_real_value_every_round() {
    let players = bots(2);
    let mut rules = Setup::default().rules();
    rules.world.wind = WindLevel::Random;
    rules.world.gravity = GravityLevel::Random;
    rules.world.walls = Walls::Random;
    rules.world.sky = Sky::Random;
    for _ in 0..20 {
        let b = battle(&players, rules);
        let w = b.rules.world;
        assert!(w.wind != WindLevel::Random && w.gravity != GravityLevel::Random);
        assert!(w.walls != Walls::Random && w.sky != Sky::Random);
        assert_eq!(b.history.len(), 1, "one line in the log says what came out");
        assert!(
            !b.random_picks(false).contains(&t!("setup-sky")),
            "the HUD leaves out what can be seen"
        );
    }
    rules.world = Setup::default().rules().world;
    assert!(
        battle(&players, rules).history.is_empty(),
        "nothing to say without Random"
    );
}

#[test]
fn random_picks_only_real_values_and_feeling_lucky_does_too() {
    use crate::choice::Choice;
    use crate::world::TerrainKind;
    for _ in 0..50 {
        assert!(!Walls::any_real().is_random());
        assert!(!Walls::Random.roll().is_random());
        assert_eq!(Walls::Bouncy.roll(), Walls::Bouncy, "a real value stays");
        let mut setup = Setup::default();
        setup.randomize();
        let w = setup.world;
        assert!(!w.terrain.is_random() && !w.wind.is_random() && !w.gravity.is_random());
        assert!(!w.walls.is_random() && !w.sky.is_random());
    }
    assert!(TerrainKind::Random.is_random() && !TerrainKind::Canyon.is_random());
}

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

#[test]
fn the_aiming_preview_follows_the_real_shot() {
    let (_, b) = range(|_| 600.0);
    let (pos, vel) = (vec2(200.0, 580.0), launch_velocity(50.0, 600.0));
    let path = predict_path(pos, vel, 30.0, b.world(), &b.terrain, &[]);
    let landing = simulate(pos, vel, 30.0, b.world(), &b.terrain, &[]).unwrap();
    assert_eq!(
        *path.last().unwrap(),
        landing.pos,
        "ends where the shot lands"
    );
    assert!(path.len() > 50);
    assert_eq!(crate::setup::AimHelp::ShortPath.preview(), 1.0 / 3.0);
    assert_eq!(crate::setup::AimHelp::Coaching.preview(), 0.0);
}

/// A human (tank 0, x = 200) against a bot (tank 1, x = 900) on `ground`, with coaching on.
fn coached(ground: fn(f32) -> f32, wind: f32) -> (Vec<Player>, Battle) {
    let mut players = bots(2);
    players[0].bot = None;
    let mut rules = Setup::default().rules();
    rules.aim_help = crate::setup::AimHelp::Coaching;
    let mut b = battle(&players, rules);
    b.terrain = Terrain::from_fn(ground);
    b.wind = wind;
    b.current = 0;
    for (tank, x) in b.tanks.iter_mut().zip([200.0, 900.0]) {
        tank.pos = vec2(x, ground(x));
    }
    (players, b)
}

/// Fires tank 0's shot with `weapon` at `angle` / `power` and plays the turn out; returns the history after it.
fn fire_and_settle(
    b: &mut Battle,
    players: &mut [Player],
    weapon: usize,
    angle: f32,
    power: f32,
) -> Vec<String> {
    let before = b.history.len();
    players[0].ammo[weapon] = players[0].ammo[weapon].max(1);
    b.tanks[0].weapon = weapon;
    b.tanks[0].angle = angle;
    b.tanks[0].power = power;
    b.fire(players);
    for _ in 0..40_000 {
        if !matches!(b.phase, Phase::Flight | Phase::Settle) {
            break;
        }
        b.update(STEP, players);
    }
    b.history[before..].iter().map(|l| l.text.clone()).collect()
}

#[test]
fn coaching_says_short_or_past_from_a_real_shot() {
    let flat = |_: f32| 600.0;
    let (mut players, mut b) = coached(flat, 0.0);
    let lines = fire_and_settle(&mut b, &mut players, 0, 45.0, 300.0);
    assert!(lines.iter().any(|l| l.contains("px short of")), "{lines:?}");
    assert!(
        !lines.iter().any(|l| l.contains("wind")),
        "no wind, no wind note: {lines:?}"
    );

    let (mut players, mut b) = coached(flat, 0.0);
    let lines = fire_and_settle(&mut b, &mut players, 0, 70.0, 1000.0);
    assert!(
        lines
            .iter()
            .any(|l| l.contains("Flew off the field") || l.contains("px past")),
        "{lines:?}"
    );
}

#[test]
fn coaching_reports_the_wind() {
    let (mut players, mut b) = coached(|_| 600.0, 40.0);
    let lines = fire_and_settle(&mut b, &mut players, 0, 50.0, 450.0);
    assert!(
        lines.iter().any(|l| l.contains("the wind moved it")),
        "{lines:?}"
    );
}

#[test]
fn a_roller_rolling_on_is_not_reported_as_wind() {
    // A slope down to the right, no wind: the Roller lands and rolls well past its first touch.
    let (mut players, mut b) = coached(|x| 400.0 + x * 0.25, 0.0);
    let lines = fire_and_settle(&mut b, &mut players, weapon("Roller"), 60.0, 380.0);
    assert!(!lines.iter().any(|l| l.contains("wind")), "{lines:?}");
}

#[test]
fn coaching_goes_the_short_way_round_with_wrap_walls() {
    let (players, mut b) = coached(|_| 600.0, 0.0);
    b.rules.world.walls = Walls::Wrap;
    b.tanks[0].pos.x = 100.0;
    b.tanks[1].pos.x = 1150.0;
    assert_eq!(b.across(100.0, 1150.0), -(WIDTH as f32) + 1050.0);
    // Fired left across the edge: 1200 is before the enemy at 1150 (80 px round the edge), 1100 is beyond it.
    for (x, expect) in [(1200.0, "50 px short"), (1100.0, "50 px past")] {
        let shot = CoachShot {
            landing: Some(vec2(x, 600.0)),
            still: Some(vec2(x, 600.0)),
            from: 100.0,
            dir: -1.0,
        };
        b.coach(shot, &players);
        let line = &b.history.last().unwrap().text;
        assert!(line.contains(expect), "{line}");
    }
}

#[test]
fn a_miss_is_marked_even_when_another_line_follows_the_shot() {
    let (players, mut b) = coached(|_| 600.0, 0.0);
    let k = b.history.len();
    b.shot_line = Some(k);
    b.shot_hit = false;
    b.note("fires".to_string(), WHITE);
    b.note("Sudden death!".to_string(), WHITE);
    b.next_turn(&players);
    assert!(
        b.history[k].text.ends_with(&t!("log-miss")),
        "{}",
        b.history[k].text
    );
}

#[test]
fn a_spent_deflector_absorbs_nothing_and_adds_nothing() {
    let mut players = bots(2);
    players[1].items[3] = 1;
    let mut b = battle(&players, Setup::default().rules());
    b.raise_shields(&mut players);
    b.tanks[1].shield.as_mut().unwrap().energy = -5.0;
    b.hurt(&[(1, 30.0)], Some(0), &mut players);
    assert_eq!(b.tanks[1].health, 70.0, "exactly the hit, nothing extra");
    assert!(b.tanks[1].shield_up().is_none(), "not shown while spent");
    assert_eq!(b.spent_deflectors(), 0b10);
    b.drop_spent_deflectors(&players);
    assert!(b.tanks[1].shield.is_none());
}

#[test]
fn coaching_stays_on_screen_until_the_next_shot() {
    let (mut players, mut b) = coached(|_| 600.0, 0.0);
    fire_and_settle(&mut b, &mut players, 0, 45.0, 300.0);
    let line = b.tanks[0]
        .coaching
        .clone()
        .expect("a coaching line for the shooter");
    assert!(line.contains("px short of"), "{line}");
    assert!(b.log.iter().any(|l| l.text == line), "also in the feed");
    b.current = 0;
    b.fire(&mut players);
    assert!(
        b.tanks[0].coaching.is_none(),
        "gone once the next shot is fired"
    );
}
