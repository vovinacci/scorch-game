//! What each weapon does where it lands.

use super::*;

#[test]
fn a_shell_explodes_on_the_bottom_of_the_screen() {
    // The ground is blown away all the way down; a tank sits on the bottom edge.
    let (mut players, mut b) = range(|_| HEIGHT);
    b.tanks[1].pos = vec2(640.0, HEIGHT);
    drop(&mut b, weapon("Missile"), 665.0, 0.0);
    let blasts = run(&mut b, &mut players);
    assert_eq!(
        blasts.len(),
        1,
        "it lands instead of falling out of the world"
    );
    assert!(blasts[0].y <= HEIGHT);
    assert!(b.tanks[1].health < 100.0, "a near miss hurts");
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

/// Fires `name` from 30 px under the surface, like a buried tank; returns where.
fn fire_buried(b: &mut Battle, name: &str) -> Vec2 {
    let start = vec2(500.0, valley(500.0) + 30.0);
    b.projectiles.push(Projectile::new(
        start,
        vec2(120.0, -200.0),
        weapon(name),
        0,
        false,
    ));
    start
}

#[test]
fn a_buried_shell_goes_off_once_where_it_is() {
    // No rolling away, no hops, no bomblets: one blast in the dirt.
    for name in ["Missile", "Roller", "Leapfrog", "Funky Bomb"] {
        let (mut players, mut b) = range(valley);
        let start = fire_buried(&mut b, name);
        let blasts = run(&mut b, &mut players);
        assert_eq!(blasts.len(), 1, "{name}");
        assert!(
            blasts[0].distance(start) < 5.0,
            "{name} explodes where it is: {}",
            blasts[0]
        );
    }
}

#[test]
fn buried_napalm_is_smothered() {
    let (mut players, mut b) = range(valley);
    fire_buried(&mut b, "Napalm");
    b.step_projectiles(&mut players);
    assert!(b.projectiles.is_empty());
    assert!(b.fires.is_empty(), "no fire runs out on the surface");
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
fn napalm_leaves_only_a_few_burn_marks() {
    // Its 18 drops burn out close together: on a slope they pool in the dip, on the flat they stay where they fell.
    for ground in [valley as fn(f32) -> f32, |_| 600.0] {
        let (mut players, mut b) = range(ground);
        drop(&mut b, weapon("Napalm"), 500.0, 0.0);
        run(&mut b, &mut players);
        let marks = b.terrain.burn_marks();
        assert!((1..=5).contains(&marks), "{marks} marks");
    }
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
fn the_predicted_roll_matches_the_real_one() {
    let (mut players, mut b) = range(valley);
    drop(&mut b, weapon("Roller"), 500.0, 0.0);
    let blast = run(&mut b, &mut players)[0];
    let landing = Landing {
        pos: vec2(500.0, valley(500.0)),
        vel: vec2(0.0, 200.0),
        ground: true,
        off_wall: false,
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
fn a_tracer_does_not_end_the_turn_and_only_one_is_allowed() {
    let (mut players, mut b) = coached(|_| 600.0, 0.0);
    players[0].ammo[weapon("Missile")] = 2;
    b.tanks[0].last_weapon = weapon("Missile");
    let turns = b.turns;
    players[0].ammo[weapon("Tracer")] = 2;
    fire_and_settle(&mut b, &mut players, weapon("Tracer"), 45.0, 400.0);
    assert_eq!(b.current, 0, "the same tank aims again");
    assert_eq!(b.turns, turns, "no new turn");
    assert!(matches!(b.phase, Phase::Aim { .. }));
    assert_eq!(
        b.tanks[0].weapon,
        weapon("Missile"),
        "back to the weapon fired last"
    );
    assert_eq!(b.tracers.len(), 1);

    // A second Tracer this turn becomes a Baby Missile, which ends the turn.
    b.tanks[0].weapon = weapon("Tracer");
    b.fire(&mut players);
    assert_eq!(
        players[0].ammo[weapon("Tracer")],
        1,
        "the second Tracer wasn't used"
    );
    for _ in 0..40_000 {
        if !matches!(b.phase, Phase::Flight | Phase::Settle) {
            break;
        }
        b.update(STEP, &mut players);
    }
    assert_ne!(b.current, 0, "this time the turn passed");
}
