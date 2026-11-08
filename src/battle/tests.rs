//! Tests for the battle: weapons, shields, turns, bots' aim and the HUD layout.

use super::*;
use crate::ai::PRESETS;
use crate::items::PARACHUTE;
use crate::setup::Setup;

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
    assert!(b.tanks[1].shield.is_none(), "the deflector is spent");
    assert!(b.deflectors(0).is_empty(), "and pushes nothing more");
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
    assert_eq!(blasts, [] as [Vec2; 0]);
    assert_eq!(b.terrain.height_at(700.0), 600.0 + DIG_DEPTH);
    assert_eq!(
        b.terrain.height_at(640.0 + DIG_LENGTH + 20.0),
        600.0,
        "only as far as it digs"
    );
    assert_eq!(b.tanks[1].health, 100.0, "no damage by itself");
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
    for pair in strip.windows(2) {
        assert!(pair[0].1.right() < pair[1].1.left(), "no overlap");
    }
    assert!(strip[0].1.left() >= 0.0 && strip.last().unwrap().1.right() <= ui::WIDTH);
}
